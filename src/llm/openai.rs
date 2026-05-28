use crate::config::{OpenAiConfig, build_quiz_prompt};
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::Client;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

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
}

impl OpenAiClient {
    pub fn new(config: &OpenAiConfig) -> Self {
        let http = Client::builder().build().expect("创建 HTTP 客户端失败");
        Self {
            http,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            model: config.model.clone(),
            api_key: config.api_key.clone(),
            enable_thinking: config.enable_thinking,
        }
    }

    pub fn ask_stream(&self, question: &str, tx: mpsc::UnboundedSender<LlmChunk>) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let prompt = build_quiz_prompt(ts, question, self.enable_thinking);

        let mut body = serde_json::json!({
            "model": self.model,
            "stream": true,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        if self.enable_thinking {
            body["enable_thinking"] = serde_json::json!(true);
            body["thinking"] = serde_json::json!({ "type": "enabled" });
        } else {
            body["enable_thinking"] = serde_json::json!(false);
            body["thinking"] = serde_json::json!({ "type": "disabled" });
        }

        let url = if self.base_url.ends_with("/chat/completions") {
            self.base_url.clone()
        } else {
            format!("{}/chat/completions", self.base_url)
        };

        let http = self.http.clone();
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();

        tokio::spawn(async move {
            let resp = match http
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&body)
                .timeout(std::time::Duration::from_secs(120))
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    let msg = if e.is_connect() && base_url.contains("dashscope.aliyuncs.com") {
                        "使用阿里云百炼请关闭系统代理，否则可能会报错".to_string()
                    } else {
                        e.to_string()
                    };
                    let _ = tx.send(LlmChunk::Error(msg));
                    return;
                }
            };

            let status = resp.status();
            if !status.is_success() {
                let body_text = resp.text().await.unwrap_or_default();
                let preview = &body_text[..body_text.len().min(300)];
                let _ = tx.send(LlmChunk::Error(format!(
                    "LLM 请求失败 (HTTP {}): {}",
                    status, preview
                )));
                return;
            }

            let mut stream = resp.bytes_stream().eventsource();
            let mut full_content = String::new();

            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        if event.data == "[DONE]" {
                            break;
                        }
                        let json: serde_json::Value = match serde_json::from_str(&event.data) {
                            Ok(j) => j,
                            Err(_) => continue,
                        };

                        let delta = &json["choices"][0]["delta"];

                        // Always check reasoning_content (fallback for models that think regardless)
                        if let Some(reasoning) = delta["reasoning_content"].as_str()
                            && !reasoning.is_empty() {
                                let _ = tx.send(LlmChunk::Thinking(reasoning.to_string()));
                            }

                        if let Some(content) = delta["content"].as_str()
                            && !content.is_empty() {
                                full_content.push_str(content);
                                let _ = tx.send(LlmChunk::Content(content.to_string()));
                            }
                    }
                    Err(e) => {
                        tracing::warn!("SSE stream error: {}", e);
                        break;
                    }
                }
            }

            let _ = tx.send(LlmChunk::Done(full_content));
        });
    }
}
