# Thinking Toggle + Streaming

## Goal

将 LLM 请求从一次性返回改为流式输出，新增「开启思考」配置开关；开启后展示模型的推理过程（reasoning_content），关闭时也做兜底解析——如果模型仍然返回了思考内容则直接展示。

## What I already know

- 三家服务商（硅基流动/百炼/DeepSeek）的 OpenAI 兼容接口统一使用 `choices[0].delta.reasoning_content` 返回思考内容（流式），与 `content` 互斥
- 部分模型（DeepSeek-R1、QwQ）始终思考，无法关闭；部分模型（Qwen3）可通过 `enable_thinking` 切换
- 当前代码已发送 `enable_thinking: false`，但解析端完全不处理思考内容
- `reqwest` 0.12 支持 `bytes_stream()`，需要加 `stream` feature；还需 `eventsource-stream` crate 解析 SSE
- 当前 UI 是 ratatui TUI，答题界面左侧有题目+选项区域，显示 "AI 思考中..." spinner

## Requirements

1. **配置开关**：在 OpenAiConfig 中新增 `enable_thinking: bool` 字段（默认 false）
   - 配置页面新增一个 toggle 控件（开/关），位于 API Key 之后、保存按钮之前
   - 保存时持久化到 `openai_config.json`
2. **流式输出**：LLM 请求改为 SSE streaming
   - 请求时根据配置决定是否发送 `enable_thinking` / `thinking` 参数
   - 逐 chunk 解析 SSE，实时更新 UI
3. **思考内容展示**：
   - 如果 `enable_thinking` 开启：请求时传参开启，UI 展示思考过程
   - 如果 `enable_thinking` 关闭：请求时传参关闭，但兜底检测 `reasoning_content`——如果有内容则直接展示
4. **答题 UI 改造**：
   - WaitingLlm 阶段改为实时显示流式内容（思考 + 答案）
   - 思考内容用暗色/不同颜色区分
   - 答案内容正常显示

## Acceptance Criteria

- [ ] 配置页面有「开启思考」toggle，可切换并持久化
- [ ] LLM 请求改为流式，逐 token 更新 UI
- [ ] enable_thinking=true 时，答题界面展示思考过程（用不同颜色）
- [ ] enable_thinking=false 时，若模型仍返回 reasoning_content，也能展示
- [ ] 答案解析（parse_answer）兼容流式累积的完整文本
- [ ] 兼容不同服务商（硅基流动/百炼/DeepSeek）的思考参数差异

## Definition of Done

- `cargo build` 无 error
- `cargo clippy` 无 new warning
- 手动测试：关闭思考 → 正常答题
- 手动测试：开启思考 → 答题界面展示思考内容

## Technical Approach

### 1. 依赖变更
- `reqwest` 加 `stream` feature
- 新增 `eventsource-stream = "0.2"`

### 2. Config 变更 (`src/config.rs`)
- `OpenAiConfig` 新增 `enable_thinking: bool`，`#[serde(default)]` 保持向后兼容
- prompt 模板：当 `enable_thinking=true` 时去掉 "不要思考，直接回答我的问题"

### 3. LLM Client 改造 (`src/llm/openai.rs`)
- `ask()` 改为 `ask_stream()` 返回流式 channel
- 新增 `LlmChunk` 枚举：`Thinking(String)` / `Content(String)` / `Done` / `Error(String)`
- SSE 解析：提取 `delta.reasoning_content` 和 `delta.content`
- 请求参数：根据 config 决定是否发送 thinking 参数

### 4. App Event 扩展 (`src/app.rs`)
- `AppEvent::LlmOk(String)` → 改为流式事件：
  - `AppEvent::LlmChunk(LlmChunk)` — 逐 chunk 更新
  - `AppEvent::LlmDone(String)` — 完成时传最终答案文本
  - `AppEvent::LlmErr(String)` — 保留
- App 新增 `thinking_text: String` 和 `answer_text: String` 字段累积流式内容

### 5. 答题 UI 改造 (`src/ui/quiz.rs`)
- WaitingLlm 阶段：左侧面板从 spinner 改为实时显示
  - 同一区域分段显示：思考内容在上（暗色 DarkGray），答案在下（正常白色）
  - 分隔线区分思考与答案
  - 自动滚动到最新内容

### 6. 配置页面 UI (`src/ui/config_page.rs`)
- 在 API Key 输入框后新增 toggle 行
- `ConfigFocus` 新增 `ThinkingToggle` variant
- 显示：`[ ] 关闭思考` / `[✓] 开启思考`

### 7. Input 处理 (`src/input.rs`)
- ConfigFocus 导航循环中插入 ThinkingToggle
- Enter/Space 切换 toggle 状态

## Decision (ADR-lite)

**Context**: 多家 LLM 服务商的思考参数和字段名不统一，且部分模型无法关闭思考
**Decision**: 采用双层策略——请求时按配置发参数，响应时始终检测 `reasoning_content` 字段（兜底）
**Consequences**: 无论服务商是否遵守关闭参数，用户都能看到思考内容（如果模型返回了的话）

## Out of Scope

- OpenAI o-series 的 reasoning_tokens（不暴露思考内容，无意义）
- 思考内容的 budget/thinking_tokens 配置
- 非流式模式保留

## Technical Notes

- 研究：`.trellis/tasks/05-28-thinking-toggle-streaming/research/llm-thinking-streaming.md`
- 关键文件：`src/config.rs`, `src/llm/openai.rs`, `src/app.rs`, `src/ui/quiz.rs`, `src/ui/config_page.rs`, `src/input.rs`
- `reasoning_content` 和 `content` 在同一个 delta chunk 中互斥，不会同时出现
- SSE 格式：`data: {json}\n\n`，结束标记 `data: [DONE]`
