use crate::config::{OpenAiConfig, build_quiz_prompt};
use eventsource_stream::Eventsource;
use futures::{StreamExt, stream};
use reqwest::{
    Client,
    header::{CONTENT_ENCODING, CONTENT_TYPE},
};
use std::{error::Error as StdError, time::Duration};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const MAX_JSON_RESPONSE_SIZE: usize = 8 * 1024 * 1024;
const MAX_RESPONSE_SNIFF_SIZE: usize = 8 * 1024;
const MAX_VISIBLE_OUTPUT_TOKENS: u64 = 128;
const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const HTTP_READ_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const HTTP_TOTAL_TIMEOUT: Duration = Duration::from_secs(35 * 60);
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
            body["max_completion_tokens"] = serde_json::json!(MAX_VISIBLE_OUTPUT_TOKENS);
            body["reasoning_effort"] = serde_json::json!(effort);
        }
        ApiDialect::XAi => {
            // xAI 已弃用 max_tokens；该字段限制可见正文，推理开销由 effort 控制。
            body["max_completion_tokens"] = serde_json::json!(MAX_VISIBLE_OUTPUT_TOKENS);
            body["reasoning_effort"] =
                serde_json::json!(xai_reasoning_effort(enable_thinking, reasoning_effort,));
        }
        ApiDialect::Extended => {
            body["max_tokens"] = serde_json::json!(MAX_VISIBLE_OUTPUT_TOKENS);
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

fn redact_urls(message: &str) -> String {
    let mut output = String::with_capacity(message.len());
    let mut rest = message;
    loop {
        let start = match (rest.find("http://"), rest.find("https://")) {
            (Some(http), Some(https)) => Some(http.min(https)),
            (Some(http), None) => Some(http),
            (None, Some(https)) => Some(https),
            (None, None) => None,
        };
        let Some(start) = start else {
            output.push_str(rest);
            break;
        };
        output.push_str(&rest[..start]);
        let url_end = rest[start..]
            .find(|character: char| {
                character.is_whitespace()
                    || matches!(
                        character,
                        '"' | '\'' | '`' | ')' | ']' | '}' | ';' | '<' | '>'
                    )
            })
            .unwrap_or(rest.len() - start);
        output.push_str("[URL REDACTED]");
        rest = &rest[start + url_end..];
    }
    output
}

fn safe_preview(message: &str, api_key: &str, max_chars: usize) -> String {
    let redacted = redact_urls(&redact_secrets(message, api_key));
    let mut preview: String = redacted.chars().take(max_chars).collect();
    if redacted.chars().count() > max_chars {
        preview.push('…');
    }
    preview
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReqwestErrorClass {
    Timeout,
    Connect,
    Request,
    ResponseBody,
    Decode,
    Other,
}

impl ReqwestErrorClass {
    fn label(self) -> &'static str {
        match self {
            Self::Timeout => "超时",
            Self::Connect => "连接",
            Self::Request => "请求构造/发送",
            Self::ResponseBody => "响应体",
            Self::Decode => "响应解码",
            Self::Other => "其他",
        }
    }
}

fn classify_reqwest_error(error: &reqwest::Error) -> ReqwestErrorClass {
    if error.is_timeout() {
        ReqwestErrorClass::Timeout
    } else if error.is_connect() {
        ReqwestErrorClass::Connect
    } else if error.is_builder() || error.is_request() {
        ReqwestErrorClass::Request
    } else if error.is_decode() {
        ReqwestErrorClass::Decode
    } else if error.is_body() {
        ReqwestErrorClass::ResponseBody
    } else {
        ReqwestErrorClass::Other
    }
}

fn safe_error_chain(error: &dyn StdError, api_key: &str) -> String {
    let mut details = vec![redact_urls(&redact_secrets(&error.to_string(), api_key))];
    let mut current = error.source();
    while let Some(error) = current {
        details.push(redact_urls(&redact_secrets(&error.to_string(), api_key)));
        current = error.source();
    }
    details.join(" -> ")
}

fn format_reqwest_error(context: &str, error: &reqwest::Error, api_key: &str) -> String {
    format!(
        "{context} [{}]: {}",
        classify_reqwest_error(error).label(),
        safe_error_chain(error, api_key)
    )
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

fn send_error(tx: &mpsc::UnboundedSender<LlmChunk>, api_key: &str, message: impl AsRef<str>) {
    let message = message.as_ref();
    let _ = tx.send(LlmChunk::Error(redact_urls(&redact_secrets(
        message, api_key,
    ))));
}

impl OpenAiClient {
    pub fn new(config: &OpenAiConfig) -> Self {
        let http = Client::builder()
            .connect_timeout(HTTP_CONNECT_TIMEOUT)
            .read_timeout(HTTP_READ_TIMEOUT)
            .timeout(HTTP_TOTAL_TIMEOUT)
            .build()
            .expect("创建 HTTP 客户端失败");
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
            assert_eq!(body["max_completion_tokens"], MAX_VISIBLE_OUTPUT_TOKENS);
        }

        let disabled = build_request_body(ApiDialect::XAi, "grok-4.6", "question", false, "max");
        assert_eq!(disabled["reasoning_effort"], "low");
        assert!(disabled.get("enable_thinking").is_none());
        assert!(disabled.get("thinking").is_none());
    }

    #[test]
    fn existing_dialects_keep_their_reasoning_fields_and_output_limit() {
        let openai = build_request_body(ApiDialect::OpenAi, "gpt-test", "question", true, "high");
        assert_eq!(openai["reasoning_effort"], "high");
        assert_eq!(openai["max_completion_tokens"], MAX_VISIBLE_OUTPUT_TOKENS);
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
    fn redacts_keys_urls_and_truncates_unicode_safely() {
        let key = "secret-test-key";
        let message = "错误：Bearer secret-test-key；密钥 secret-test-key；访问 https://api.example.test/v1?key=secret-test-key 中文内容";
        let preview = safe_preview(message, key, 200);
        assert!(!preview.contains(key));
        assert!(!preview.contains("https://api.example.test"));
        assert!(preview.contains("[REDACTED]"));
        assert!(preview.contains("[URL REDACTED]"));

        let truncated = safe_preview("错误：中文内容", key, 4);
        assert_eq!(truncated, "错误：中…");
    }
}
