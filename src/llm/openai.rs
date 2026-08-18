use crate::config::{OpenAiConfig, build_quiz_prompt};
use eventsource_stream::Eventsource;
use futures::{StreamExt, stream};
use reqwest::{Client, header::CONTENT_TYPE};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const MAX_JSON_RESPONSE_SIZE: usize = 8 * 1024 * 1024;

#[derive(Debug)]
pub enum LlmChunk {
    Thinking(String),
    Content(String),
    Done(String),
    Error(String),
}

pub struct OpenAiClient {
    http: Client,
    base_url: String,
    model: String,
    api_key: String,
    enable_thinking: bool,
    reasoning_effort: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApiDialect {
    OpenAi,
    XAi,
    Extended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResponseFormat {
    Sse,
    Json,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct ParsedPayload {
    reasoning: Option<String>,
    content: Option<String>,
}

fn api_dialect(base_url: &str, model: &str) -> ApiDialect {
    let host = reqwest::Url::parse(base_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned));

    match host.as_deref() {
        Some("api.openai.com") => ApiDialect::OpenAi,
        Some("api.x.ai") => ApiDialect::XAi,
        _ if model.trim().to_ascii_lowercase().starts_with("grok-") => ApiDialect::XAi,
        _ => ApiDialect::Extended,
    }
}

fn build_request_body(
    dialect: ApiDialect,
    model: &str,
    prompt: &str,
    enable_thinking: bool,
    reasoning_effort: &str,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": model,
        "stream": true,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });
    let effort = if enable_thinking {
        reasoning_effort
    } else {
        "none"
    };

    match dialect {
        ApiDialect::OpenAi => {
            body["reasoning_effort"] = serde_json::json!(effort);
        }
        ApiDialect::XAi => {}
        ApiDialect::Extended => {
            body["enable_thinking"] = serde_json::json!(enable_thinking);
            body["thinking"] = serde_json::json!({
                "type": if enable_thinking { "enabled" } else { "disabled" }
            });
            body["reasoning_effort"] = serde_json::json!(effort);
        }
    }

    body
}

fn detect_response_format(content_type: Option<&str>, prefix: &[u8]) -> Result<ResponseFormat, String> {
    let text = String::from_utf8_lossy(prefix);
    let trimmed = text.trim_start_matches('\u{feff}').trim_start();

    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return Ok(ResponseFormat::Json);
    }
    if trimmed.starts_with("data:")
        || trimmed.starts_with("event:")
        || trimmed.starts_with("id:")
        || trimmed.starts_with("retry:")
        || trimmed.starts_with(':')
    {
        return Ok(ResponseFormat::Sse);
    }

    let media_type = content_type
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .map(str::to_ascii_lowercase);
    match media_type.as_deref() {
        Some("text/event-stream") => Ok(ResponseFormat::Sse),
        Some("application/json") => Ok(ResponseFormat::Json),
        Some(value) if value.starts_with("application/") && value.ends_with("+json") => {
            Ok(ResponseFormat::Json)
        }
        _ => Err("无法识别 LLM 响应格式".to_string()),
    }
}

fn extract_text(value: Option<&serde_json::Value>) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    match value {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(text) => Ok((!text.is_empty()).then(|| text.clone())),
        serde_json::Value::Array(parts) => {
            let mut text = String::new();
            for part in parts {
                match part {
                    serde_json::Value::Null => {}
                    serde_json::Value::String(value) => text.push_str(value),
                    serde_json::Value::Object(object) => {
                        let part_type = object.get("type").and_then(serde_json::Value::as_str);
                        let part_text = object.get("text").and_then(serde_json::Value::as_str);
                        if matches!(part_type, None | Some("text") | Some("output_text")) {
                            if let Some(value) = part_text {
                                text.push_str(value);
                            } else if !object.is_empty() {
                                return Err("LLM content 数组包含无法识别的文本结构".to_string());
                            }
                        } else {
                            return Err("LLM content 数组包含不支持的内容类型".to_string());
                        }
                    }
                    _ => return Err("LLM content 数组包含不支持的内容类型".to_string()),
                }
            }
            Ok((!text.is_empty()).then_some(text))
        }
        serde_json::Value::Object(object) => {
            if let Some(text) = object.get("text").and_then(serde_json::Value::as_str) {
                Ok((!text.is_empty()).then(|| text.to_string()))
            } else {
                Err("LLM content 使用了无法识别的对象结构".to_string())
            }
        }
        _ => Err("LLM content 不是文本或文本数组".to_string()),
    }
}

fn parse_payload(value: &serde_json::Value) -> Result<Option<ParsedPayload>, String> {
    if let Some(error) = value.get("error") {
        let message = error
            .get("message")
            .and_then(serde_json::Value::as_str)
            .or_else(|| error.as_str())
            .unwrap_or("未知 API 错误");
        return Err(format!("LLM API 返回错误: {message}"));
    }

    let Some(choice) = value
        .get("choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|choices| choices.first())
    else {
        return Ok(None);
    };
    let Some(payload) = choice
        .get("delta")
        .filter(|value| value.is_object())
        .or_else(|| choice.get("message").filter(|value| value.is_object()))
    else {
        return Ok(None);
    };

    Ok(Some(ParsedPayload {
        reasoning: extract_text(
            payload
                .get("reasoning_content")
                .or_else(|| payload.get("reasoning")),
        )?,
        content: extract_text(payload.get("content"))?,
    }))
}

fn emit_payload(
    payload: ParsedPayload,
    full_content: &mut String,
    tx: &mpsc::UnboundedSender<LlmChunk>,
) {
    if let Some(reasoning) = payload.reasoning {
        let _ = tx.send(LlmChunk::Thinking(reasoning));
    }
    if let Some(content) = payload.content {
        full_content.push_str(&content);
        let _ = tx.send(LlmChunk::Content(content));
    }
}

fn redact_secrets(message: &str, api_key: &str) -> String {
    if api_key.is_empty() {
        return message.to_string();
    }
    message
        .replace(&format!("Bearer {api_key}"), "Bearer [REDACTED]")
        .replace(api_key, "[REDACTED]")
}

fn safe_preview(message: &str, api_key: &str, max_chars: usize) -> String {
    let redacted = redact_secrets(message, api_key);
    let mut preview: String = redacted.chars().take(max_chars).collect();
    if redacted.chars().count() > max_chars {
        preview.push('…');
    }
    preview
}

fn send_error(tx: &mpsc::UnboundedSender<LlmChunk>, api_key: &str, message: impl AsRef<str>) {
    let _ = tx.send(LlmChunk::Error(redact_secrets(message.as_ref(), api_key)));
}

impl OpenAiClient {
    pub fn new(config: &OpenAiConfig) -> Self {
        let http = Client::builder().build().expect("创建 HTTP 客户端失败");
        // 兜底：配置文件手动编辑导致空值时回退到默认 high
        let reasoning_effort = if config.reasoning_effort.is_empty() {
            "high".to_string()
        } else {
            config.reasoning_effort.clone()
        };
        Self {
            http,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            model: config.model.clone(),
            api_key: config.api_key.clone(),
            enable_thinking: config.enable_thinking,
            reasoning_effort,
        }
    }

    pub fn ask_stream(
        &self,
        question: &str,
        categories: Vec<String>,
        tx: mpsc::UnboundedSender<LlmChunk>,
        token: CancellationToken,
    ) {
        let prompt = build_quiz_prompt(&categories, question, self.enable_thinking);
        let body = build_request_body(
            api_dialect(&self.base_url, &self.model),
            &self.model,
            &prompt,
            self.enable_thinking,
            &self.reasoning_effort,
        );

        let url = self.base_url.clone();
        let http = self.http.clone();
        let api_key = self.api_key.clone();

        tokio::spawn(async move {
            if token.is_cancelled() {
                return;
            }
            let mut resp = match http
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {api_key}"))
                .json(&body)
                .timeout(std::time::Duration::from_secs(120))
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    send_error(&tx, &api_key, format!("LLM 请求失败: {error}"));
                    return;
                }
            };

            let status = resp.status();
            if !status.is_success() {
                let body_text = resp.text().await.unwrap_or_default();
                let preview = safe_preview(&body_text, &api_key, 300);
                send_error(
                    &tx,
                    &api_key,
                    format!("LLM 请求失败 (HTTP {status}): {preview}"),
                );
                return;
            }

            let content_type = resp
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let first_chunk = match resp.chunk().await {
                Ok(Some(chunk)) => chunk,
                Ok(None) => {
                    send_error(&tx, &api_key, "LLM 返回了空响应体");
                    return;
                }
                Err(error) => {
                    send_error(&tx, &api_key, format!("读取 LLM 响应失败: {error}"));
                    return;
                }
            };
            let format = match detect_response_format(content_type.as_deref(), &first_chunk) {
                Ok(format) => format,
                Err(error) => {
                    send_error(&tx, &api_key, error);
                    return;
                }
            };

            let mut full_content = String::new();
            match format {
                ResponseFormat::Json => {
                    let mut bytes = first_chunk.to_vec();
                    loop {
                        if bytes.len() > MAX_JSON_RESPONSE_SIZE {
                            send_error(&tx, &api_key, "LLM JSON 响应超过 8 MiB 限制");
                            return;
                        }
                        match resp.chunk().await {
                            Ok(Some(chunk)) => bytes.extend_from_slice(&chunk),
                            Ok(None) => break,
                            Err(error) => {
                                send_error(&tx, &api_key, format!("读取 LLM JSON 响应失败: {error}"));
                                return;
                            }
                        }
                    }
                    let value: serde_json::Value = match serde_json::from_slice(&bytes) {
                        Ok(value) => value,
                        Err(error) => {
                            let preview = safe_preview(&String::from_utf8_lossy(&bytes), &api_key, 200);
                            send_error(
                                &tx,
                                &api_key,
                                format!("解析 LLM JSON 响应失败: {error}; 响应摘要: {preview}"),
                            );
                            return;
                        }
                    };
                    match parse_payload(&value) {
                        Ok(Some(payload)) => emit_payload(payload, &mut full_content, &tx),
                        Ok(None) => {
                            send_error(&tx, &api_key, "LLM JSON 响应不包含 choices[0] 内容");
                            return;
                        }
                        Err(error) => {
                            send_error(&tx, &api_key, error);
                            return;
                        }
                    }
                }
                ResponseFormat::Sse => {
                    let first =
                        stream::once(async move { Ok::<_, reqwest::Error>(first_chunk) });
                    let mut events = first.chain(resp.bytes_stream()).eventsource();
                    while let Some(event) = events.next().await {
                        if token.is_cancelled() {
                            return;
                        }
                        let event = match event {
                            Ok(event) => event,
                            Err(error) => {
                                send_error(&tx, &api_key, format!("解析 LLM SSE 响应失败: {error}"));
                                return;
                            }
                        };
                        if event.data == "[DONE]" {
                            break;
                        }
                        let value: serde_json::Value = match serde_json::from_str(&event.data) {
                            Ok(value) => value,
                            Err(error) => {
                                send_error(
                                    &tx,
                                    &api_key,
                                    format!("解析 LLM SSE 事件 JSON 失败: {error}"),
                                );
                                return;
                            }
                        };
                        match parse_payload(&value) {
                            Ok(Some(payload)) => emit_payload(payload, &mut full_content, &tx),
                            Ok(None) => {}
                            Err(error) => {
                                send_error(&tx, &api_key, error);
                                return;
                            }
                        }
                    }
                }
            }

            if token.is_cancelled() {
                return;
            }
            if full_content.trim().is_empty() {
                send_error(&tx, &api_key, "LLM 响应结束但未包含可用正文");
            } else {
                let _ = tx.send(LlmChunk::Done(full_content));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_api_dialect_by_host_and_model() {
        assert_eq!(
            api_dialect("https://api.openai.com/v1/chat/completions", "gpt-test"),
            ApiDialect::OpenAi
        );
        assert_eq!(
            api_dialect("https://API.X.AI:443/v1/chat/completions", "grok-4.6"),
            ApiDialect::XAi
        );
        assert_eq!(
            api_dialect("http://proxy.example/chat/completions", " Grok-4.6 "),
            ApiDialect::XAi
        );
        assert_eq!(
            api_dialect("https://api.x.ai.example/v1/chat/completions", "other-model"),
            ApiDialect::Extended
        );
        assert_eq!(
            api_dialect("http://proxy.example/chat/completions", "deepseek-chat"),
            ApiDialect::Extended
        );
    }

    #[test]
    fn xai_proxy_body_preserves_model_and_omits_thinking_fields() {
        let body = build_request_body(ApiDialect::XAi, "grok-4.6", "question", true, "high");
        assert_eq!(body["model"], "grok-4.6");
        assert_eq!(body["stream"], true);
        assert!(body.get("enable_thinking").is_none());
        assert!(body.get("thinking").is_none());
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn existing_dialects_keep_their_request_fields() {
        let openai = build_request_body(ApiDialect::OpenAi, "gpt-test", "question", true, "high");
        assert_eq!(openai["reasoning_effort"], "high");
        assert!(openai.get("enable_thinking").is_none());

        let extended =
            build_request_body(ApiDialect::Extended, "other", "question", false, "max");
        assert_eq!(extended["enable_thinking"], false);
        assert_eq!(extended["thinking"]["type"], "disabled");
        assert_eq!(extended["reasoning_effort"], "none");
    }

    #[test]
    fn detects_json_and_sse_with_missing_or_wrong_content_type() {
        assert_eq!(
            detect_response_format(Some("application/json; charset=utf-8"), br#"{"ok":true}"#),
            Ok(ResponseFormat::Json)
        );
        assert_eq!(
            detect_response_format(Some("text/event-stream"), b"data: {}\n\n"),
            Ok(ResponseFormat::Sse)
        );
        assert_eq!(
            detect_response_format(Some("text/plain"), b"  {\"choices\":[]}"),
            Ok(ResponseFormat::Json)
        );
        assert_eq!(
            detect_response_format(Some("application/json"), b"data: {}\n\n"),
            Ok(ResponseFormat::Sse)
        );
        assert_eq!(
            detect_response_format(Some("text/event-stream"), b"{\"choices\":[]}"),
            Ok(ResponseFormat::Json)
        );
    }

    #[test]
    fn parses_synchronous_message_content() {
        let value = serde_json::json!({
            "choices": [{"message": {"content": "3"}}]
        });
        assert_eq!(
            parse_payload(&value),
            Ok(Some(ParsedPayload {
                reasoning: None,
                content: Some("3".to_string())
            }))
        );
    }

    #[test]
    fn parses_reasoning_and_text_part_arrays() {
        let value = serde_json::json!({
            "choices": [{"message": {
                "reasoning_content": "分析",
                "content": [
                    {"type": "text", "text": "答案"},
                    {"type": "output_text", "text": "是2"}
                ]
            }}]
        });
        assert_eq!(
            parse_payload(&value),
            Ok(Some(ParsedPayload {
                reasoning: Some("分析".to_string()),
                content: Some("答案是2".to_string())
            }))
        );
    }

    #[test]
    fn parses_stream_delta_and_message_fallback() {
        let delta = serde_json::json!({"choices": [{"delta": {"content": "1"}}]});
        let message = serde_json::json!({"choices": [{"message": {"content": "4"}}]});
        assert_eq!(
            parse_payload(&delta).unwrap().unwrap().content.as_deref(),
            Some("1")
        );
        assert_eq!(
            parse_payload(&message).unwrap().unwrap().content.as_deref(),
            Some("4")
        );
    }

    #[test]
    fn empty_choices_and_api_errors_are_not_content() {
        assert_eq!(parse_payload(&serde_json::json!({"choices": []})), Ok(None));
        assert!(
            parse_payload(&serde_json::json!({"error": {"message": "bad request"}}))
                .unwrap_err()
                .contains("bad request")
        );
        assert!(
            parse_payload(&serde_json::json!({
                "choices": [{"message": {"content": {"unsupported": true}}}]
            }))
            .is_err()
        );
    }

    #[test]
    fn redacts_keys_and_truncates_unicode_safely() {
        let key = "secret-test-key";
        let message = "错误：Bearer secret-test-key；密钥 secret-test-key；中文内容";
        let preview = safe_preview(message, key, 24);
        assert!(!preview.contains(key));
        assert!(preview.contains("[REDACTED]"));
    }
}
