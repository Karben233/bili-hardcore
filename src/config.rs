use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const CONFIG_DIR_NAME: &str = ".bili-hardcore";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthData {
    pub access_token: String,
    pub csrf: String,
    pub mid: String,
    pub cookie: String,
}

fn config_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户主目录")
        .join(CONFIG_DIR_NAME)
}

pub fn openai_config_path() -> PathBuf {
    config_dir().join("openai_config.json")
}

pub fn auth_path() -> PathBuf {
    config_dir().join("auth.json")
}

pub fn ensure_config_dir() -> Result<()> {
    let dir = config_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).context("创建配置目录失败")?;
    }
    Ok(())
}

// --- OpenAI Config ---

pub fn load_openai_config() -> Result<Option<OpenAiConfig>> {
    let path = openai_config_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path).context("读取 API 配置失败")?;
    let config: OpenAiConfig = serde_json::from_str(&content).context("解析 API 配置失败")?;
    Ok(Some(config))
}

pub fn save_openai_config(config: &OpenAiConfig) -> Result<()> {
    ensure_config_dir()?;
    let path = openai_config_path();
    let content = serde_json::to_string_pretty(config).context("序列化 API 配置失败")?;
    fs::write(&path, content).context("写入 API 配置失败")?;
    Ok(())
}

// --- Auth ---

pub fn load_auth() -> Result<Option<AuthData>> {
    let path = auth_path();
    if !path.exists() {
        return Ok(None);
    }

    let metadata = fs::metadata(&path).context("读取认证文件元数据失败")?;
    let modified = metadata.modified().context("获取文件修改时间失败")?;
    let elapsed = modified.elapsed().unwrap_or_default();
    if elapsed.as_secs() > 7 * 24 * 3600 {
        return Ok(None);
    }

    let content = fs::read_to_string(&path).context("读取认证信息失败")?;
    let auth: AuthData = serde_json::from_str(&content).context("解析认证信息失败")?;
    Ok(Some(auth))
}

pub fn save_auth(auth: &AuthData) -> Result<()> {
    ensure_config_dir()?;
    let path = auth_path();
    let content = serde_json::to_string_pretty(auth).context("序列化认证信息失败")?;
    fs::write(&path, content).context("写入认证信息失败")?;
    Ok(())
}

pub fn delete_openai_config() -> Result<()> {
    let path = openai_config_path();
    if path.exists() {
        fs::remove_file(path).context("删除 API 配置失败")?;
    }
    Ok(())
}

pub fn delete_auth() -> Result<()> {
    let path = auth_path();
    if path.exists() {
        fs::remove_file(path).context("删除认证信息失败")?;
    }
    Ok(())
}

/// LLM prompt 模板
pub const QUIZ_PROMPT_TEMPLATE: &str = "\
当前时间：{}
你是一个高效精准的答题专家，面对选择题时，直接根据问题和选项判断正确答案，并返回对应选项的序号（1, 2, 3, 4）。示例：
问题：大的反义词是什么？
选项：['长', '宽', '小', '热']
回答：3
如果不确定正确答案，选择最接近的选项序号返回，不提供额外解释或超出 1-4 的内容。
---
不要思考，直接回答我的问题：{}";
