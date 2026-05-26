use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("API 错误: {message} (code={code})")]
    Api { code: i64, message: String },

    #[error("网络请求失败: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}
