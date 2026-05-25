# PRD: 简化 API Provider 为仅 OpenAI 兼容格式

## 目标
移除 DeepSeek 和 Gemini 的独立 provider 支持，只保留 OpenAI 兼容格式（自定义 base_url + model + api_key）。

## 背景
项目目前支持 3 种 LLM provider：DeepSeek（固定配置）、Gemini（固定配置）、OpenAI 兼容（自定义配置）。DeepSeek 和 Gemini 本质上都可以通过 OpenAI 兼容格式调用，维护独立的 provider 类增加了不必要的复杂度。

## 改动清单

### 删除文件
- `bili-hardcore/tools/LLM/deepseek.py` — DeepSeek 独立实现
- `bili-hardcore/tools/LLM/gemini.py` — Gemini 独立实现
- `CONFIG_EXAMPLE.md` — 旧的配置示例（内容已是 OpenAI 格式，但位置在根目录不合理）

### 修改文件

#### `bili-hardcore/config/config.py`
- 移除 `load_api_key()`、`save_api_key()`、`load_gemini_key()`、`save_gemini_key()` 函数
- 移除 `API_KEY_GEMINI`、`API_KEY_DEEPSEEK` 变量
- 移除 provider 选择菜单（print + input + if-elif-else）
- 启动时直接加载 OpenAI 配置，不存在则引导用户输入 base_url、model、api_key
- 移除 `model_choice` 导出（`start_senior.py` 不再需要）
- 保留 `load_openai_config()`、`save_openai_config()`
- 保留 `PROMPT`、`API_CONFIG`、`HEADERS`、`AUTH_FILE` 等无关配置

#### `bili-hardcore/tools/LLM/openai.py`
- 保留不变（已经是 OpenAI 兼容格式实现）
- 移除 deepseek 特有的思考模型参数（`enable_thinking`、`thinking`），因为这是 deepseek 特定的

#### `bili-hardcore/scripts/start_senior.py`
- 移除 `from tools.LLM.gemini import GeminiAPI` 和 `from tools.LLM.deepseek import DeepSeekAPI`
- 移除 `from config.config import model_choice`
- 移除 `start()` 方法中的 if-elif-else provider 选择逻辑
- 直接实例化 `OpenAIAPI()`（在 `__init__` 或 start 开头初始化一次即可）
- 每次 retry 不需要重新创建 LLM 实例

#### `bili-hardcore/README.md`
- 更新配置说明，移除 DeepSeek/Gemini 选项提及

## 不变的部分
- `bili-hardcore/tools/LLM/openai.py` 的核心请求逻辑
- `bili-hardcore/scripts/check_config.py`
- `bili-hardcore/client/` 下的所有文件
- 认证、验证码、答题提交逻辑
