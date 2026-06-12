use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const CONFIG_DIR_NAME: &str = ".bili-hardcore";

// --- Preset Templates ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetConfig {
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetTemplate {
    pub provider: String,
    pub provider_name: String,
    pub config: PresetConfig,
}

const PRESETS_JSON: &str = include_str!("presets.json");

pub fn load_presets() -> Vec<PresetTemplate> {
    serde_json::from_str(PRESETS_JSON).unwrap_or_default()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    #[serde(default)]
    pub enable_thinking: bool,
    #[serde(default)]
    pub enable_fast_mode: bool,
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

// --- Selected Categories ---

pub fn categories_path() -> PathBuf {
    config_dir().join("categories.json")
}

pub fn load_categories() -> Vec<String> {
    let path = categories_path();
    if !path.exists() {
        return vec![];
    }
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn save_categories(categories: &[String]) -> Result<()> {
    ensure_config_dir()?;
    let path = categories_path();
    let content = serde_json::to_string_pretty(categories).context("序列化分类失败")?;
    fs::write(&path, content).context("写入分类失败")?;
    Ok(())
}

// --- Quiz History ---

use crate::app::HistoryItem;

pub fn history_path() -> PathBuf {
    config_dir().join("history.json")
}

pub fn load_history() -> Vec<HistoryItem> {
    let path = history_path();
    if !path.exists() {
        return vec![];
    }
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn save_history(history: &[HistoryItem]) -> Result<()> {
    ensure_config_dir()?;
    let path = history_path();
    let content = serde_json::to_string_pretty(history).context("序列化答题记录失败")?;
    fs::write(&path, content).context("写入答题记录失败")?;
    Ok(())
}

/// 构建 LLM prompt
pub fn build_quiz_prompt(categories: &[String], question: &str, enable_thinking: bool) -> String {
    let cat_str = if categories.is_empty() {
        "未知".to_string()
    } else {
        categories.join("、")
    };
    let base = format!(
        "你是一个资深B站用户，目前正在完成硬核会员试炼考试，考试内容涉及的分区：[{}]，面对选择题时，直接根据问题和选项判断正确答案，并返回对应选项的序号（1, 2, 3, 4）。示例：\n\
         问题：大的反义词是什么？\n\
         选项：['长', '宽', '小', '热']\n\
         回答：3\n\
         如果不确定正确答案，选择最接近的选项序号返回，不提供额外解释或超出 1-4 的内容。",
        cat_str
    );
    if enable_thinking {
        format!("{}\n---\n{}", base, question)
    } else {
        format!("{}\n---\n不要思考，直接回答我的问题：{}", base, question)
    }
}
