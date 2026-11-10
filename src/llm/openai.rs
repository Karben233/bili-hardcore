use super::http::{
    build_http_client, format_reqwest_error, safe_error_chain, safe_preview, send_error,
};
use crate::config::{OpenAiConfig, build_chat_prompt, build_quiz_prompt};
use eventsource_stream::Eventsource;
use futures::{StreamExt, stream};
use reqwest::{
    Client,
    header::{CONTENT_ENCODING, CONTENT_TYPE},
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const MAX_JSON_RESPONSE_SIZE: usize = 8 * 1024 * 1024;
const MAX_RESPONSE_SNIFF_SIZE: usize = 8 * 1024;
const MAX_VISIBLE_OUTPUT_TOKENS: u64 = 128;
/// 思考模式下推理 token 与正文共享 max_tokens 预算（DeepSeek、智谱等），
/// 128 会被推理过程耗尽导致正文被截断，因此思考时放宽上限。
const MAX_THINKING_OUTPUT_TOKENS: u64 = 4096;
const UTF8_BOM: &[u8] = b"\xef\xbb\xbf";
const SSE_PREFIXES: [&[u8]; 4] = [b"data:", b"event:", b"id:", b"retry:"];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrefixDecision {
    Detected(ResponseFormat),
    NeedMore,
    NoMatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SniffStop {
    Mismatch,
    EndOfResponse,
    Limit,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct ParsedPayload {
    reasoning: Option<String>,
    content: Option<String>,
    finish_reason: Option<String>,
}

fn api_dialect(base_url: &str, model: &str) -> ApiDialect {
    let host = reqwest::Url::parse(base_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned));

    match host.as_deref() {
        Some("api.openai.com") => ApiDialect::OpenAi,
        Some("api.x.ai") => ApiDialect::XAi,
        // 自定义 Grok 中转仍使用 xAI-compatible Chat Completions 参数。
        _ if model.trim().to_ascii_lowercase().starts_with("grok-") => ApiDialect::XAi,
        _ => ApiDialect::Extended,
    }
}

fn xai_reasoning_effort(enable_thinking: bool, saved_effort: &str) -> &'static str {
    if !enable_thinking {
        // 当前 Grok 推理模型仍会推理，low 是可用的最低档位。
        return "low";
    }

    match saved_effort.trim() {
        effort if effort.eq_ignore_ascii_case("low") => "low",
        effort if effort.eq_ignore_ascii_case("medium") => "medium",
        effort if effort.eq_ignore_ascii_case("high") => "high",
        effort if effort.eq_ignore_ascii_case("max") || effort.eq_ignore_ascii_case("xhigh") => {
            "xhigh"
        }
        // 配置文件可能被手动编辑；未知值回退到兼容性最好的 high。
        _ => "high",
    }
}

fn output_token_limit(enable_thinking: bool) -> u64 {
    if enable_thinking {
        MAX_THINKING_OUTPUT_TOKENS
    } else {
        MAX_VISIBLE_OUTPUT_TOKENS
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
    let max_output_tokens = output_token_limit(enable_thinking);

    match dialect {
        ApiDialect::OpenAi => {
            body["max_completion_tokens"] = serde_json::json!(max_output_tokens);
            body["reasoning_effort"] = serde_json::json!(effort);
        }
        ApiDialect::XAi => {
            // xAI 已弃用 max_tokens；max_completion_tokens 同时限制推理与正文，
            // 非思考时收紧到 128 防止啰嗦正文。
            body["max_completion_tokens"] = serde_json::json!(max_output_tokens);
            body["reasoning_effort"] =
                serde_json::json!(xai_reasoning_effort(enable_thinking, reasoning_effort,));
        }
        ApiDialect::Extended => {
            body["max_tokens"] = serde_json::json!(max_output_tokens);
            body["enable_thinking"] = serde_json::json!(enable_thinking);
            body["thinking"] = serde_json::json!({
                "type": if enable_thinking { "enabled" } else { "disabled" }
            });
            body["reasoning_effort"] = serde_json::json!(effort);
        }
    }

    body
}

fn inspect_body_signature(prefix: &[u8]) -> PrefixDecision {
    let mut body = prefix;
    if body.starts_with(UTF8_BOM) {
        body = &body[UTF8_BOM.len()..];
    } else if !body.is_empty() && body.len() < UTF8_BOM.len() && UTF8_BOM.starts_with(body) {
        return PrefixDecision::NeedMore;
    }

    body = body.trim_ascii_start();
    let Some(first) = body.first() else {
        return PrefixDecision::NeedMore;
    };
    if matches!(first, b'{' | b'[') {
        return PrefixDecision::Detected(ResponseFormat::Json);
    }
    if *first == b':' {
        return PrefixDecision::Detected(ResponseFormat::Sse);
    }
    for marker in SSE_PREFIXES {
        if body.starts_with(marker) {
            return PrefixDecision::Detected(ResponseFormat::Sse);
        }
        if body.len() < marker.len() && marker.starts_with(body) {
            return PrefixDecision::NeedMore;
        }
    }
    PrefixDecision::NoMatch
}

fn response_format_from_header(content_type: Option<&str>) -> Option<ResponseFormat> {
    let media_type = content_type
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .map(str::to_ascii_lowercase);
    match media_type.as_deref() {
        Some("text/event-stream") => Some(ResponseFormat::Sse),
        Some("application/json") => Some(ResponseFormat::Json),
        Some(value) if value.starts_with("application/") && value.ends_with("+json") => {
            Some(ResponseFormat::Json)
        }
        _ => None,
    }
}

fn strip_utf8_bom_from_chunks(chunks: &mut [bytes::Bytes]) {
    let mut remaining = UTF8_BOM.len();
    for chunk in chunks {
        let skipped = remaining.min(chunk.len());
        *chunk = chunk.slice(skipped..);
        remaining -= skipped;
        if remaining == 0 {
            break;
        }
    }
}

fn single_line_preview(message: &str, api_key: &str, max_chars: usize) -> String {
    safe_preview(message, api_key, max_chars)
        .chars()
        .flat_map(char::escape_debug)
        .collect()
}

fn has_unsupported_content_encoding(content_encoding: Option<&str>) -> bool {
    content_encoding.is_some_and(|value| {
        value
            .split(',')
            .map(str::trim)
            .any(|encoding| !encoding.is_empty() && !encoding.eq_ignore_ascii_case("identity"))
    })
}

fn fallback_response_format(
    content_type: Option<&str>,
    content_encoding: Option<&str>,
    prefix: &[u8],
    stop: SniffStop,
    api_key: &str,
) -> Result<ResponseFormat, String> {
    if has_unsupported_content_encoding(content_encoding) {
        return Err(unrecognized_format_error(
            content_type,
            content_encoding,
            prefix,
            stop,
            api_key,
        ));
    }
    response_format_from_header(content_type).ok_or_else(|| {
        unrecognized_format_error(content_type, content_encoding, prefix, stop, api_key)
    })
}

fn unrecognized_format_error(
    content_type: Option<&str>,
    content_encoding: Option<&str>,
    prefix: &[u8],
    stop: SniffStop,
    api_key: &str,
) -> String {
    let unsupported_encoding = has_unsupported_content_encoding(content_encoding);
    let content_type = single_line_preview(content_type.unwrap_or("<缺失>"), api_key, 100);
    let content_encoding = single_line_preview(content_encoding.unwrap_or("<缺失>"), api_key, 100);
    let preview = single_line_preview(&String::from_utf8_lossy(prefix), api_key, 200);
    let stop = match stop {
        SniffStop::Mismatch => "正文前缀不匹配 JSON/SSE",
        SniffStop::EndOfResponse => "响应已结束",
        SniffStop::Limit => "已达到 8 KiB 探测上限",
    };
    if unsupported_encoding {
        format!(
            "无法识别 LLM 响应格式，响应可能仍使用不支持的 Content-Encoding; Content-Type={content_type}; Content-Encoding={content_encoding}; {stop}; 响应摘要={preview}"
        )
    } else {
        format!(
            "无法识别 LLM 响应格式; Content-Type={content_type}; Content-Encoding={content_encoding}; {stop}; 响应摘要={preview}"
        )
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
        finish_reason: choice
            .get("finish_reason")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    }))
}

/// `finish_reason == "length"` 表示输出被 max_tokens 截断，正文可能还没开始。
fn is_truncated_by_token_limit(payload: &ParsedPayload) -> bool {
    payload
        .finish_reason
        .as_deref()
        .is_some_and(|reason| reason.eq_ignore_ascii_case("length"))
}

fn empty_content_error(truncated: bool, max_output_tokens: u64) -> String {
    if truncated {
        format!(
            "LLM 输出因达到 max_tokens={max_output_tokens} 上限被截断，未产生可用正文（思考模式的推理 token 也计入该上限）；可关闭思考模式或降低思考强度后重试"
        )
    } else {
        "LLM 响应结束但未包含可用正文".to_string()
    }
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

fn format_event_stream_error(
    error: &eventsource_stream::EventStreamError<reqwest::Error>,
    api_key: &str,
) -> String {
    match error {
        eventsource_stream::EventStreamError::Transport(error) => {
            format_reqwest_error("读取 LLM SSE 响应失败", error, api_key)
        }
        _ => format!(
            "读取 LLM SSE 响应失败 [响应解码]: {}",
            safe_error_chain(error, api_key)
        ),
    }
}

impl OpenAiClient {
    pub fn new(config: &OpenAiConfig) -> Self {
        // 兜底：配置文件手动编辑导致空值时回退到默认 high
        let reasoning_effort = if config.reasoning_effort.is_empty() {
            "high".to_string()
        } else {
            config.reasoning_effort.clone()
        };
        Self {
            http: build_http_client(),
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
        options: &[String],
        categories: &[String],
        tx: mpsc::UnboundedSender<LlmChunk>,
        token: CancellationToken,
    ) {
        let prompt = build_quiz_prompt(
            categories,
            &build_chat_prompt(question, options),
            self.enable_thinking,
        );
        tracing::info!("LLM prompt:\n{}", prompt);
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
        let max_output_tokens = output_token_limit(self.enable_thinking);

        tokio::spawn(async move {
            if token.is_cancelled() {
                return;
            }
            let mut resp = tokio::select! {
                biased;
                _ = token.cancelled() => return,
                result = http
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .header("Authorization", format!("Bearer {api_key}"))
                    .json(&body)
                    .send() => match result {
                        Ok(response) => response,
                        Err(error) => {
                            send_error(
                                &tx,
                                &api_key,
                                format_reqwest_error("LLM 请求失败", &error, &api_key),
                            );
                            return;
                        }
                    }
            };

            let status = resp.status();
            if !status.is_success() {
                let body_text = tokio::select! {
                    biased;
                    _ = token.cancelled() => return,
                    result = resp.text() => match result {
                        Ok(body_text) => body_text,
                        Err(error) => {
                            send_error(
                                &tx,
                                &api_key,
                                format_reqwest_error("读取 LLM 错误响应失败", &error, &api_key),
                            );
                            return;
                        }
                    }
                };
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
            let content_encoding = resp
                .headers()
                .get(CONTENT_ENCODING)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let mut sniff_prefix = Vec::new();
            let mut buffered_chunks = Vec::new();
            let (format, stop) = loop {
                match inspect_body_signature(&sniff_prefix) {
                    PrefixDecision::Detected(format) => break (Some(format), None),
                    PrefixDecision::NoMatch => break (None, Some(SniffStop::Mismatch)),
                    PrefixDecision::NeedMore if sniff_prefix.len() >= MAX_RESPONSE_SNIFF_SIZE => {
                        break (None, Some(SniffStop::Limit));
                    }
                    PrefixDecision::NeedMore => {}
                }

                let chunk = tokio::select! {
                    biased;
                    _ = token.cancelled() => return,
                    result = resp.chunk() => result,
                };
                match chunk {
                    Ok(Some(chunk)) => {
                        let remaining = MAX_RESPONSE_SNIFF_SIZE - sniff_prefix.len();
                        let inspected = remaining.min(chunk.len());
                        sniff_prefix.extend_from_slice(&chunk[..inspected]);
                        buffered_chunks.push(chunk);
                    }
                    Ok(None) => break (None, Some(SniffStop::EndOfResponse)),
                    Err(error) => {
                        send_error(
                            &tx,
                            &api_key,
                            format_reqwest_error("读取 LLM 响应失败", &error, &api_key),
                        );
                        return;
                    }
                }
            };
            let format = if let Some(format) = format {
                format
            } else {
                match fallback_response_format(
                    content_type.as_deref(),
                    content_encoding.as_deref(),
                    &sniff_prefix,
                    stop.expect("未识别格式必须包含停止原因"),
                    &api_key,
                ) {
                    Ok(format) => format,
                    Err(error) => {
                        send_error(&tx, &api_key, error);
                        return;
                    }
                }
            };
            if sniff_prefix.starts_with(UTF8_BOM) {
                strip_utf8_bom_from_chunks(&mut buffered_chunks);
            }

            let mut full_content = String::new();
            let mut truncated_by_limit = false;
            match format {
                ResponseFormat::Json => {
                    let mut bytes = Vec::new();
                    for chunk in buffered_chunks {
                        bytes.extend_from_slice(&chunk);
                    }
                    loop {
                        if bytes.len() > MAX_JSON_RESPONSE_SIZE {
                            send_error(&tx, &api_key, "LLM JSON 响应超过 8 MiB 限制");
                            return;
                        }
                        let chunk = tokio::select! {
                            biased;
                            _ = token.cancelled() => return,
                            result = resp.chunk() => result,
                        };
                        match chunk {
                            Ok(Some(chunk)) => bytes.extend_from_slice(&chunk),
                            Ok(None) => break,
                            Err(error) => {
                                send_error(
                                    &tx,
                                    &api_key,
                                    format_reqwest_error(
                                        "读取 LLM JSON 响应失败",
                                        &error,
                                        &api_key,
                                    ),
                                );
                                return;
                            }
                        }
                    }
                    let value: serde_json::Value = match serde_json::from_slice(&bytes) {
                        Ok(value) => value,
                        Err(error) => {
                            let preview =
                                safe_preview(&String::from_utf8_lossy(&bytes), &api_key, 200);
                            send_error(
                                &tx,
                                &api_key,
                                format!("解析 LLM JSON 响应失败: {error}; 响应摘要: {preview}"),
                            );
                            return;
                        }
                    };
                    match parse_payload(&value) {
                        Ok(Some(payload)) => {
                            truncated_by_limit |= is_truncated_by_token_limit(&payload);
                            emit_payload(payload, &mut full_content, &tx);
                        }
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
                    let replay =
                        stream::iter(buffered_chunks.into_iter().map(Ok::<_, reqwest::Error>));
                    let events = replay.chain(resp.bytes_stream()).eventsource();
                    futures::pin_mut!(events);
                    loop {
                        let event = tokio::select! {
                            biased;
                            _ = token.cancelled() => return,
                            event = events.next() => event,
                        };
                        let Some(event) = event else {
                            break;
                        };
                        let event = match event {
                            Ok(event) => event,
                            Err(error) => {
                                send_error(
                                    &tx,
                                    &api_key,
                                    format_event_stream_error(&error, &api_key),
                                );
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
                            Ok(Some(payload)) => {
                                truncated_by_limit |= is_truncated_by_token_limit(&payload);
                                emit_payload(payload, &mut full_content, &tx);
                            }
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
                send_error(
                    &tx,
                    &api_key,
                    empty_content_error(truncated_by_limit, max_output_tokens),
                );
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
            api_dialect(
                "https://api.x.ai.example/v1/chat/completions",
                "other-model"
            ),
            ApiDialect::Extended
        );
        assert_eq!(
            api_dialect("http://proxy.example/chat/completions", "deepseek-chat"),
            ApiDialect::Extended
        );
    }

    #[test]
    fn xai_body_maps_reasoning_effort_and_limits_completion() {
        for (saved, expected) in [
            ("low", "low"),
            ("medium", "medium"),
            ("high", "high"),
            ("max", "xhigh"),
            ("xhigh", "xhigh"),
            ("edited-value", "high"),
        ] {
            let body = build_request_body(ApiDialect::XAi, "grok-4.6", "question", true, saved);
            assert_eq!(body["reasoning_effort"], expected);
            assert_eq!(body["max_completion_tokens"], MAX_THINKING_OUTPUT_TOKENS);
        }

        let disabled = build_request_body(ApiDialect::XAi, "grok-4.6", "question", false, "max");
        assert_eq!(disabled["reasoning_effort"], "low");
        assert_eq!(disabled["max_completion_tokens"], MAX_VISIBLE_OUTPUT_TOKENS);
        assert!(disabled.get("enable_thinking").is_none());
        assert!(disabled.get("thinking").is_none());
    }

    #[test]
    fn thinking_mode_reserves_tokens_for_reasoning_across_dialects() {
        // 推理 token 与正文共享输出上限，思考时必须放宽，否则正文会被截断。
        assert_eq!(output_token_limit(true), MAX_THINKING_OUTPUT_TOKENS);
        assert_eq!(output_token_limit(false), MAX_VISIBLE_OUTPUT_TOKENS);

        for dialect in [ApiDialect::OpenAi, ApiDialect::XAi, ApiDialect::Extended] {
            let field = if dialect == ApiDialect::Extended {
                "max_tokens"
            } else {
                "max_completion_tokens"
            };
            let thinking = build_request_body(dialect, "model", "question", true, "max");
            assert_eq!(thinking[field], MAX_THINKING_OUTPUT_TOKENS);
            let plain = build_request_body(dialect, "model", "question", false, "max");
            assert_eq!(plain[field], MAX_VISIBLE_OUTPUT_TOKENS);
        }
    }

    #[test]
    fn existing_dialects_keep_their_reasoning_fields_and_output_limit() {
        let openai = build_request_body(ApiDialect::OpenAi, "gpt-test", "question", true, "high");
        assert_eq!(openai["reasoning_effort"], "high");
        assert_eq!(openai["max_completion_tokens"], MAX_THINKING_OUTPUT_TOKENS);
        assert!(openai.get("max_tokens").is_none());
        assert!(openai.get("enable_thinking").is_none());

        let extended = build_request_body(ApiDialect::Extended, "other", "question", false, "max");
        assert_eq!(extended["enable_thinking"], false);
        assert_eq!(extended["thinking"]["type"], "disabled");
        assert_eq!(extended["reasoning_effort"], "none");
        assert_eq!(extended["max_tokens"], MAX_VISIBLE_OUTPUT_TOKENS);
        assert!(extended.get("max_completion_tokens").is_none());
    }

    #[test]
    fn detects_complete_json_and_sse_signatures() {
        assert_eq!(
            inspect_body_signature(br#"{"ok":true}"#),
            PrefixDecision::Detected(ResponseFormat::Json)
        );
        assert_eq!(
            inspect_body_signature(b" \r\ndata: {}\n\n"),
            PrefixDecision::Detected(ResponseFormat::Sse)
        );
        assert_eq!(
            inspect_body_signature(b"event: message\n"),
            PrefixDecision::Detected(ResponseFormat::Sse)
        );
        assert_eq!(
            inspect_body_signature(b": keep-alive\n"),
            PrefixDecision::Detected(ResponseFormat::Sse)
        );
    }

    #[test]
    fn waits_for_split_bom_whitespace_and_sse_markers() {
        for prefix in [
            &b""[..],
            &b" \r\n"[..],
            &b"\xef"[..],
            &b"\xef\xbb"[..],
            &b"d"[..],
            &b"data"[..],
            &b"ev"[..],
            &b"retr"[..],
        ] {
            assert_eq!(inspect_body_signature(prefix), PrefixDecision::NeedMore);
        }
        assert_eq!(
            inspect_body_signature(b"\xef\xbb\xbf \r\ndata:"),
            PrefixDecision::Detected(ResponseFormat::Sse)
        );
        assert_eq!(
            inspect_body_signature(b"\xef\xbb\xbf [1]"),
            PrefixDecision::Detected(ResponseFormat::Json)
        );
        assert_eq!(inspect_body_signature(b"<html>"), PrefixDecision::NoMatch);
    }

    #[test]
    fn body_signatures_override_headers_and_headers_are_strict_fallbacks() {
        assert_eq!(
            inspect_body_signature(b"data: {}\n\n"),
            PrefixDecision::Detected(ResponseFormat::Sse)
        );
        assert_eq!(
            response_format_from_header(Some("application/json; charset=utf-8")),
            Some(ResponseFormat::Json)
        );
        assert_eq!(
            response_format_from_header(Some(" Application/Problem+Json ; charset=utf-8")),
            Some(ResponseFormat::Json)
        );
        assert_eq!(
            response_format_from_header(Some("TEXT/EVENT-STREAM")),
            Some(ResponseFormat::Sse)
        );
        assert_eq!(response_format_from_header(Some("text/plain")), None);
        assert_eq!(response_format_from_header(None), None);
        assert_eq!(
            fallback_response_format(
                Some("application/json"),
                None,
                b" ",
                SniffStop::EndOfResponse,
                "key"
            ),
            Ok(ResponseFormat::Json)
        );
        assert!(
            fallback_response_format(
                Some("application/json"),
                Some("zstd"),
                b"binary",
                SniffStop::Mismatch,
                "key"
            )
            .is_err()
        );
        assert!(
            fallback_response_format(
                Some("text/plain"),
                None,
                b"plain text",
                SniffStop::Mismatch,
                "key"
            )
            .is_err()
        );
    }

    #[test]
    fn strips_only_one_leading_utf8_bom_across_chunks() {
        let mut chunks = vec![
            bytes::Bytes::from_static(b"\xef"),
            bytes::Bytes::from_static(b"\xbb"),
            bytes::Bytes::from_static(b"\xbf {\"choices\":[]}"),
        ];
        strip_utf8_bom_from_chunks(&mut chunks);
        let json: Vec<u8> = chunks
            .iter()
            .flat_map(|chunk| chunk.iter().copied())
            .collect();
        assert_eq!(json, b" {\"choices\":[]}");
        assert!(serde_json::from_slice::<serde_json::Value>(&json).is_ok());
    }

    #[test]
    fn unrecognized_format_diagnostics_are_redacted_and_actionable() {
        let key = "secret-test-key";
        let error = unrecognized_format_error(
            Some("text/plain"),
            Some("zstd"),
            format!("<html>Bearer {key}\nblocked</html>").as_bytes(),
            SniffStop::Mismatch,
            key,
        );
        assert!(error.contains("不支持的 Content-Encoding"));
        assert!(error.contains("Content-Type=text/plain"));
        assert!(error.contains("Content-Encoding=zstd"));
        assert!(error.contains("\\n"));
        assert!(!error.contains(key));
    }

    #[test]
    fn parses_synchronous_message_content() {
        let value = serde_json::json!({
            "choices": [{"message": {"content": "3"}, "finish_reason": "stop"}]
        });
        assert_eq!(
            parse_payload(&value),
            Ok(Some(ParsedPayload {
                reasoning: None,
                content: Some("3".to_string()),
                finish_reason: Some("stop".to_string()),
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
                content: Some("答案是2".to_string()),
                finish_reason: None,
            }))
        );
    }

    #[test]
    fn detects_truncated_output_and_explains_it() {
        let truncated = serde_json::json!({
            "choices": [{"delta": {}, "finish_reason": "length"}]
        });
        let payload = parse_payload(&truncated).unwrap().unwrap();
        assert!(is_truncated_by_token_limit(&payload));

        let stopped = serde_json::json!({
            "choices": [{"delta": {"content": "2"}, "finish_reason": "stop"}]
        });
        let payload = parse_payload(&stopped).unwrap().unwrap();
        assert!(!is_truncated_by_token_limit(&payload));

        assert_eq!(
            empty_content_error(false, MAX_VISIBLE_OUTPUT_TOKENS),
            "LLM 响应结束但未包含可用正文"
        );
        let message = empty_content_error(true, MAX_THINKING_OUTPUT_TOKENS);
        assert!(message.contains("被截断"));
        assert!(message.contains(&MAX_THINKING_OUTPUT_TOKENS.to_string()));
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
}
