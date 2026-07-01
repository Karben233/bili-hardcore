use crate::config::{OpenAiConfig, build_quiz_prompt};
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::Client;

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

    pub fn ask_stream(&self, question: &str, categories: Vec<String>, tx: mpsc::UnboundedSender<LlmChunk>) {
        let prompt = build_quiz_prompt(&categories, question, self.enable_thinking);

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

        let is_openai = self.base_url.contains("api.openai.com");
        let effort = if self.enable_thinking { "medium" } else { "none" };

        if is_openai {
            // OpenAI 官方 API 仅识别 reasoning_effort；enable_thinking / thinking 为非官方参数，不下发
            body["reasoning_effort"] = serde_json::json!(effort);
        } else {
            body["enable_thinking"] = serde_json::json!(self.enable_thinking);
            body["thinking"] = serde_json::json!({
                "type": if self.enable_thinking { "enabled" } else { "disabled" }
            });
            body["reasoning_effort"] = serde_json::json!(effort);
        }

        let url = self.base_url.clone();
        let http = self.http.clone();
        let api_key = self.api_key.clone();

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
                    let _ = tx.send(LlmChunk::Error(e.to_string()));
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
