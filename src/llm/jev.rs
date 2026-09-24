use super::LlmChunk;
use super::http::{HTTP_CONNECT_TIMEOUT, format_reqwest_error, safe_preview, send_error};
use crate::config::OpenAiConfig;
use futures::StreamExt;
use reqwest::{
    Client, Response,
    header::{CONTENT_TYPE, RETRY_AFTER},
};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const MAX_RESPONSE_SIZE: usize = 1024 * 1024;
const MAX_OVERLOAD_RETRIES: usize = 3;
const RETRY_BASE_DELAY: Duration = Duration::from_secs(2);
const MAX_RETRY_AFTER: Duration = Duration::from_secs(30);
const JEV_READ_TIMEOUT: Duration = Duration::from_secs(60);
const JEV_TOTAL_TIMEOUT: Duration = Duration::from_secs(120);
const ANSWER_ID: &str = "answer";

/// JEV 主训练语言为英语、中文准确率偏低，故指令用英文、state 承载中文题目。
const CHOICE_INSTRUCTIONS: &str = "Pick the single option that correctly answers the question in `question`. Judge only from the question text and the option texts; exactly one option is correct.";

pub fn is_jev_endpoint(base_url: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(base_url) else {
        return false;
    };
    if url
        .host_str()
        .is_some_and(|host| host.eq_ignore_ascii_case("api.typesafe.ai"))
    {
        return true;
    }
    // 中转/代理走路径识别：Chat Completions 端点不会以 /systemone 结尾
    url.path()
        .trim_end_matches('/')
        .to_ascii_lowercase()
        .ends_with("/systemone")
}

fn build_request_body(
    model: &str,
    question: &str,
    options: &[String],
    categories: &[String],
) -> serde_json::Value {
    let mut state = serde_json::Map::new();
    state.insert("question".into(), serde_json::json!(question));
    if !categories.is_empty() {
        state.insert("categories".into(), serde_json::json!(categories));
    }

    let criteria: serde_json::Map<String, serde_json::Value> = options
        .iter()
        .enumerate()
        .map(|(i, option)| ((i + 1).to_string(), serde_json::json!(option)))
        .collect();

    serde_json::json!({
        "state": serde_json::Value::Object(state),
        "model": model,
        "questions": {
            ANSWER_ID: {
                "type": "choice",
                "instructions": CHOICE_INSTRUCTIONS,
                "criteria": serde_json::Value::Object(criteria),
            }
        }
    })
}

struct JevChoice {
    index: usize,
    model: String,
    confidence: Option<f64>,
    probabilities: Vec<(usize, f64)>,
    input_tokens: Option<u64>,
}

fn parse_choice(value: &serde_json::Value, option_count: usize) -> Result<JevChoice, String> {
    let answer = value
        .get("answers")
        .and_then(|answers| answers.get(ANSWER_ID))
        .and_then(|answer| answer.as_object())
        .ok_or("JEV 响应缺少 answers.answer")?;

    match answer.get("type").and_then(serde_json::Value::as_str) {
        Some("choice") | None => {}
        Some(other) => return Err(format!("JEV 返回了非 choice 答案类型: {other}")),
    }

    let raw_choice = answer
        .get("choice")
        .and_then(serde_json::Value::as_str)
        .ok_or("JEV 响应缺少 answers.answer.choice")?;
    let index: usize = raw_choice
        .trim()
        .parse()
        .map_err(|_| format!("JEV 返回了无法解析的选项: {raw_choice}"))?;
    if index == 0 || index > option_count {
        return Err(format!(
            "JEV 返回的选项 {index} 超出有效范围 1-{option_count}"
        ));
    }

    let probabilities: Vec<(usize, f64)> = answer
        .get("probabilities")
        .and_then(serde_json::Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(key, value)| {
                    let index: usize = key.trim().parse().ok()?;
                    if index == 0 || index > option_count {
                        return None;
                    }
                    Some((index, value.as_f64()?))
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(JevChoice {
        index,
        model: value
            .get("model")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        confidence: answer.get("confidence").and_then(serde_json::Value::as_f64),
        probabilities,
        input_tokens: value
            .get("usage")
            .and_then(|usage| usage.get("input_tokens"))
            .and_then(serde_json::Value::as_u64),
    })
}

impl JevChoice {
    fn summary(&self) -> String {
        let mut summary = format!("JEV {}｜", self.model);
        if let Some(confidence) = self.confidence {
            summary.push_str(&format!("置信度 {confidence:.2}｜"));
        }
        if !self.probabilities.is_empty() {
            let mut ordered = self.probabilities.clone();
            ordered.sort_by_key(|(key, _)| *key);
            let parts: Vec<String> = ordered
                .iter()
                .map(|(key, probability)| format!("{key}:{probability:.2}"))
                .collect();
            summary.push_str(&format!("选项概率 {}", parts.join(" ")));
        }
        summary
    }
}

fn parse_retry_after(response: &Response) -> Option<Duration> {
    // 仅支持 delta-seconds；HTTP-date 形式退回指数退避
    let seconds: u64 = response
        .headers()
        .get(RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some(Duration::from_secs(seconds))
}

fn backoff_delay(attempt: usize, retry_after: Option<Duration>) -> Duration {
    match retry_after {
        Some(delay) => delay.min(MAX_RETRY_AFTER),
        None => RETRY_BASE_DELAY * 2u32.pow(attempt as u32),
    }
}

fn describe_status(status: reqwest::StatusCode) -> &'static str {
    match status.as_u16() {
        401 => "API Key 无效或缺失",
        422 => "请求体未通过校验",
        429 => "超出速率限制",
        529 => "服务端过载",
        _ => "未知错误",
    }
}

enum ReadOutcome {
    Cancelled,
    Failed(String),
}

async fn read_success_body(
    response: Response,
    api_key: &str,
    token: &CancellationToken,
) -> Result<serde_json::Value, ReadOutcome> {
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    loop {
        let chunk = tokio::select! {
            biased;
            _ = token.cancelled() => return Err(ReadOutcome::Cancelled),
            chunk = stream.next() => chunk,
        };
        let Some(chunk) = chunk else { break };
        let chunk = chunk.map_err(|error| {
            ReadOutcome::Failed(format_reqwest_error("读取 JEV 响应失败", &error, api_key))
        })?;
        if bytes.len() + chunk.len() > MAX_RESPONSE_SIZE {
            return Err(ReadOutcome::Failed(format!(
                "JEV 响应超过 {} MiB 限制",
                MAX_RESPONSE_SIZE / 1024 / 1024
            )));
        }
        bytes.extend_from_slice(&chunk);
    }

    let preview = safe_preview(&String::from_utf8_lossy(&bytes), api_key, 200);
    serde_json::from_slice(&bytes).map_err(|error| {
        ReadOutcome::Failed(format!(
            "解析 JEV JSON 响应失败: {error}; 响应摘要: {preview}"
        ))
    })
}

pub struct JevClient {
    http: Client,
    endpoint: String,
    model: String,
    api_key: String,
}

impl JevClient {
    pub fn new(config: &OpenAiConfig) -> Self {
        let http = Client::builder()
            .connect_timeout(HTTP_CONNECT_TIMEOUT)
            .read_timeout(JEV_READ_TIMEOUT)
            .timeout(JEV_TOTAL_TIMEOUT)
            .build()
            .expect("创建 HTTP 客户端失败");
        Self {
            http,
            endpoint: config.base_url.trim_end_matches('/').to_string(),
            model: config.model.clone(),
            api_key: config.api_key.clone(),
        }
    }

    pub fn ask(
        &self,
        question: &str,
        options: &[String],
        categories: &[String],
        tx: mpsc::UnboundedSender<LlmChunk>,
        token: CancellationToken,
    ) {
        if options.is_empty() {
            send_error(&tx, &self.api_key, "JEV 无法作答：题目没有可选项");
            return;
        }

        let body = build_request_body(&self.model, question, options, categories);
        tracing::info!("JEV request:\n{}", body);

        let endpoint = self.endpoint.clone();
        let http = self.http.clone();
        let api_key = self.api_key.clone();
        let option_count = options.len();

        tokio::spawn(async move {
            if token.is_cancelled() {
                return;
            }

            let mut attempt = 0usize;
            loop {
                let response = tokio::select! {
                    biased;
                    _ = token.cancelled() => return,
                    result = http
                        .post(&endpoint)
                        .header(CONTENT_TYPE, "application/json")
                        .header("Authorization", format!("Bearer {api_key}"))
                        .json(&body)
                        .send() => match result {
                            Ok(response) => response,
                            Err(error) => {
                                send_error(
                                    &tx,
                                    &api_key,
                                    format_reqwest_error("JEV 请求失败", &error, &api_key),
                                );
                                return;
                            }
                        }
                };

                let status = response.status();
                if status.is_success() {
                    let value = match read_success_body(response, &api_key, &token).await {
                        Ok(value) => value,
                        Err(ReadOutcome::Cancelled) => return,
                        Err(ReadOutcome::Failed(message)) => {
                            send_error(&tx, &api_key, message);
                            return;
                        }
                    };
                    let choice = match parse_choice(&value, option_count) {
                        Ok(choice) => choice,
                        Err(message) => {
                            send_error(&tx, &api_key, message);
                            return;
                        }
                    };
                    if let Some(tokens) = choice.input_tokens {
                        tracing::info!("JEV input_tokens={tokens}");
                    }
                    if token.is_cancelled() {
                        return;
                    }
                    let _ = tx.send(LlmChunk::Thinking(choice.summary()));
                    let _ = tx.send(LlmChunk::Done(choice.index.to_string()));
                    return;
                }

                let retry_after = parse_retry_after(&response);
                let body_text = tokio::select! {
                    biased;
                    _ = token.cancelled() => return,
                    result = response.text() => match result {
                        Ok(body_text) => body_text,
                        Err(error) => {
                            send_error(
                                &tx,
                                &api_key,
                                format_reqwest_error("读取 JEV 错误响应失败", &error, &api_key),
                            );
                            return;
                        }
                    }
                };

                // 429/529 是推理开始前的拒绝，不会产生计费，官方要求指数退避重试。
                if matches!(status.as_u16(), 429 | 529) && attempt < MAX_OVERLOAD_RETRIES {
                    let delay = backoff_delay(attempt, retry_after);
                    attempt += 1;
                    tracing::warn!(
                        "JEV 返回 HTTP {status}，{}s 后进行第 {attempt} 次重试",
                        delay.as_secs()
                    );
                    tokio::select! {
                        _ = token.cancelled() => return,
                        _ = tokio::time::sleep(delay) => {}
                    }
                    continue;
                }

                let preview = safe_preview(&body_text, &api_key, 300);
                let retried = if attempt > 0 {
                    format!("（已重试 {attempt} 次）")
                } else {
                    String::new()
                };
                send_error(
                    &tx,
                    &api_key,
                    format!(
                        "JEV 请求失败 (HTTP {status} {}): {preview}{retried}",
                        describe_status(status)
                    ),
                );
                return;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Vec<String> {
        ["长", "宽", "小", "热"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn detects_jev_by_official_host_and_by_endpoint_path() {
        assert!(is_jev_endpoint("https://api.typesafe.ai/v1/systemone"));
        assert!(is_jev_endpoint("https://API.TypeSafe.AI/v1/systemone"));
        assert!(is_jev_endpoint("https://relay.example.com/v1/systemone/"));
        assert!(is_jev_endpoint(
            "https://relay.example.com/typesafe/systemone"
        ));

        assert!(!is_jev_endpoint(
            "https://api.openai.com/v1/chat/completions"
        ));
        assert!(!is_jev_endpoint("https://api.x.ai/v1/chat/completions"));
        assert!(!is_jev_endpoint(
            "https://relay.example.com/v1/chat/completions"
        ));
        assert!(!is_jev_endpoint("not a url"));
    }

    #[test]
    fn request_body_uses_choice_question_with_indexed_criteria() {
        let body = build_request_body("jev-latest", "大的反义词是什么？", &options(), &[]);
        assert_eq!(body["model"], "jev-latest");
        assert_eq!(body["state"]["question"], "大的反义词是什么？");
        assert!(body["state"].get("categories").is_none());

        let question = &body["questions"]["answer"];
        assert_eq!(question["type"], "choice");
        assert!(
            question["instructions"]
                .as_str()
                .unwrap()
                .contains("`question`")
        );
        assert_eq!(question["criteria"]["1"], "长");
        assert_eq!(question["criteria"]["2"], "宽");
        assert_eq!(question["criteria"]["3"], "小");
        assert_eq!(question["criteria"]["4"], "热");
        assert_eq!(question["criteria"].as_object().unwrap().len(), 4);
    }

    #[test]
    fn request_body_includes_selected_categories() {
        let body = build_request_body(
            "jev-latest",
            "题目",
            &options(),
            &["科技".to_string(), "生活".to_string()],
        );
        assert_eq!(body["state"]["categories"][0], "科技");
        assert_eq!(body["state"]["categories"][1], "生活");
    }

    #[test]
    fn parses_documented_choice_answer() {
        let value = serde_json::json!({
            "model": "jev-1.13.0",
            "answers": {
                "answer": {
                    "type": "choice",
                    "choice": "3",
                    "probabilities": {"1": 0.04, "2": 0.05, "3": 0.88, "4": 0.03},
                    "confidence": 0.81
                }
            },
            "usage": { "input_tokens": 318, "output_tokens": 34 }
        });
        let choice = parse_choice(&value, 4).expect("应解析出选项");
        assert_eq!(choice.index, 3);
        assert_eq!(choice.model, "jev-1.13.0");
        assert_eq!(choice.confidence, Some(0.81));
        assert_eq!(choice.input_tokens, Some(318));
        assert_eq!(choice.probabilities.len(), 4);

        let summary = choice.summary();
        assert!(summary.contains("jev-1.13.0"));
        assert!(summary.contains("0.81"));
        assert!(summary.contains("1:0.04"));
        assert!(summary.contains("4:0.03"));
    }

    #[test]
    fn rejects_out_of_range_missing_and_wrong_type_answers() {
        let out_of_range = serde_json::json!({
            "answers": {"answer": {"type": "choice", "choice": "5"}}
        });
        assert!(parse_choice(&out_of_range, 4).is_err());

        let missing = serde_json::json!({"model": "jev-1.13.0", "usage": {}});
        assert!(parse_choice(&missing, 4).is_err());

        let wrong_type = serde_json::json!({
            "answers": {"answer": {"type": "noul", "noul": 0.9}}
        });
        assert!(parse_choice(&wrong_type, 4).is_err());

        let non_numeric = serde_json::json!({
            "answers": {"answer": {"type": "choice", "choice": "third"}}
        });
        assert!(parse_choice(&non_numeric, 4).is_err());
    }

    #[test]
    fn choice_answer_without_optional_fields_still_yields_an_index() {
        let value = serde_json::json!({
            "answers": {"answer": {"type": "choice", "choice": "2"}}
        });
        let choice = parse_choice(&value, 4).expect("最小响应也应可用");
        assert_eq!(choice.index, 2);
        assert_eq!(choice.confidence, None);
        assert!(choice.probabilities.is_empty());
        assert_eq!(choice.model, "unknown");
    }

    #[test]
    fn backoff_doubles_and_prefers_retry_after_within_cap() {
        assert_eq!(backoff_delay(0, None), Duration::from_secs(2));
        assert_eq!(backoff_delay(1, None), Duration::from_secs(4));
        assert_eq!(backoff_delay(2, None), Duration::from_secs(8));
        assert_eq!(
            backoff_delay(0, Some(Duration::from_secs(1))),
            Duration::from_secs(1)
        );
        assert_eq!(
            backoff_delay(0, Some(Duration::from_secs(999))),
            Duration::from_secs(30),
            "retry-after 超过上限时按上限等待"
        );
    }

    #[test]
    fn maps_documented_error_statuses_to_hints() {
        assert_eq!(
            describe_status(reqwest::StatusCode::UNAUTHORIZED),
            "API Key 无效或缺失"
        );
        assert_eq!(
            describe_status(reqwest::StatusCode::UNPROCESSABLE_ENTITY),
            "请求体未通过校验"
        );
        assert_eq!(
            describe_status(reqwest::StatusCode::TOO_MANY_REQUESTS),
            "超出速率限制"
        );
        assert_eq!(
            describe_status(reqwest::StatusCode::from_u16(529).unwrap()),
            "服务端过载"
        );
    }
}
