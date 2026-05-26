use crate::config::{OpenAiConfig, QUIZ_PROMPT_TEMPLATE};
use crate::error::AppError;
use reqwest::Client;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct OpenAiClient {
    http: Client,
    base_url: String,
    model: String,
    api_key: String,
}

impl OpenAiClient {
    pub fn new(config: &OpenAiConfig) -> Self {
        let http = Client::builder().build().expect("创建 HTTP 客户端失败");
        Self {
            http,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            model: config.model.clone(),
            api_key: config.api_key.clone(),
        }
    }

    pub async fn ask(&self, question: &str) -> Result<String, AppError> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let prompt = QUIZ_PROMPT_TEMPLATE
            .replace("{}", &ts.to_string())
            .replacen("{}", question, 1);

        // 对齐 Python 版本的所有参数
        let body = serde_json::json!({
            "model": self.model,
            "enable_thinking": false,
            "thinking": {
                "type": "disabled"
            },
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        let url = if self.base_url.ends_with("/chat/completions") {
            self.base_url.clone()
        } else {
            format!("{}/chat/completions", self.base_url)
        };
        let resp = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() && self.base_url.contains("dashscope.aliyuncs.com") {
                    AppError::other("使用阿里云百炼请关闭系统代理，否则可能会报错")
                } else {
                    AppError::from(e)
                }
            })?;

        let text = resp.text().await?;
        let log_preview = if text.len() > 500 {
            &text[..text.floor_char_boundary(500)]
        } else {
            &text
        };
        tracing::info!("LLM response => {}", log_preview);
        let json: Value = serde_json::from_str(&text).map_err(|e| {
            let body_preview = if text.len() > 200 {
                &text[..text.floor_char_boundary(200)]
            } else {
                &text
            };
            AppError::other(format!("LLM JSON解析失败: {} | body: {}", e, body_preview))
        })?;
        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                AppError::other(format!(
                    "LLM 响应解析失败: {}",
                    &text[..text.len().min(200)]
                ))
            })
    }
}
