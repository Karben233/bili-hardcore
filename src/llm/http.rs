use reqwest::Client;
use std::{error::Error as StdError, time::Duration};
use tokio::sync::mpsc;

use crate::llm::LlmChunk;

pub(crate) const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const HTTP_READ_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const HTTP_TOTAL_TIMEOUT: Duration = Duration::from_secs(35 * 60);

pub(crate) fn build_http_client() -> Client {
    Client::builder()
        .connect_timeout(HTTP_CONNECT_TIMEOUT)
        .read_timeout(HTTP_READ_TIMEOUT)
        .timeout(HTTP_TOTAL_TIMEOUT)
        .build()
        .expect("创建 HTTP 客户端失败")
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

pub(crate) fn safe_preview(message: &str, api_key: &str, max_chars: usize) -> String {
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

pub(crate) fn safe_error_chain(error: &dyn StdError, api_key: &str) -> String {
    let mut details = vec![redact_urls(&redact_secrets(&error.to_string(), api_key))];
    let mut current = error.source();
    while let Some(error) = current {
        details.push(redact_urls(&redact_secrets(&error.to_string(), api_key)));
        current = error.source();
    }
    details.join(" -> ")
}

pub(crate) fn format_reqwest_error(context: &str, error: &reqwest::Error, api_key: &str) -> String {
    format!(
        "{context} [{}]: {}",
        classify_reqwest_error(error).label(),
        safe_error_chain(error, api_key)
    )
}

pub(crate) fn send_error(
    tx: &mpsc::UnboundedSender<LlmChunk>,
    api_key: &str,
    message: impl AsRef<str>,
) {
    let message = message.as_ref();
    let _ = tx.send(LlmChunk::Error(redact_urls(&redact_secrets(
        message, api_key,
    ))));
}

#[cfg(test)]
mod tests {
    use super::*;

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
