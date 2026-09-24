pub(crate) mod http;
pub mod jev;
pub mod openai;

use crate::config::OpenAiConfig;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub use jev::{JevClient, is_jev_endpoint};
pub use openai::{LlmChunk, OpenAiClient};

/// JEV 无 OpenAI 兼容端点，与 Chat Completions 是两套协议，按 base_url 分流。
pub enum LlmClient {
    OpenAi(OpenAiClient),
    Jev(JevClient),
}

impl LlmClient {
    pub fn new(config: &OpenAiConfig) -> Self {
        if is_jev_endpoint(&config.base_url) {
            Self::Jev(JevClient::new(config))
        } else {
            Self::OpenAi(OpenAiClient::new(config))
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
        match self {
            Self::OpenAi(client) => client.ask_stream(question, options, categories, tx, token),
            Self::Jev(client) => client.ask(question, options, categories, tx, token),
        }
    }
}
