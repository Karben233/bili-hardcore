# 优化 LLM 重试：统一上限常量 + 可见提示 + 指数退避

## Goal

当前 LLM 答题失败时会自动重试最多 3 次，但有两个问题：

1. **重试不可见**：`LlmRetry` 事件处理（`src/app.rs:912-917`）只清空文本 + 重新 `spawn_llm`，没有任何 UI 提示。加上 HTTP 错误瞬间返回，3 次重试几乎瞬间打完，用户（测试时）完全无法观察到重试正在发生，体感像"没重试就报错"。
2. **硬编码上限**：解析失败路径里的上限 `3` 是硬编码（`src/app.rs:887` `self.llm_retries + 1 < 3`、`:890` `(x/3)`），未复用其它路径的 `max_retries` 变量。

目标：让重试**可观察**（UI 提示 + 指数退避间隔），并统一重试上限常量。

> 背景：用户反馈"HTTP 400 没重试"实为测试场景（故意填错模型名测试重试）。代码本身会重试 3 次，痛点是重试过程不可见 + 瞬间连发。因此**保留对所有错误的重试**（不引入 4xx 不重试的分类），重点是可见性与节奏。

## Requirements

1. **统一重试上限**：引入模块级常量（如 `const MAX_LLM_RETRIES: u32 = 3;`），消除 `src/app.rs` 中所有硬编码的 `3`，所有重试路径引用同一常量。
2. **重试 UI 提示**：重试期间在状态行（`src/ui/quiz.rs:287-310` 的 phase-specific status line）显示提示，如 `↻ 第 N/3 次重试，Xs 后重试...`（黄色警告色），替代静默重试。
3. **指数退避延迟**：每次重试前等待 `2s × 2^(attempt-1)`（即 2s / 4s / 8s），避免瞬间连发。延迟必须**异步**实现，不阻塞 TUI 主循环。
4. **路径汇聚**：当前三条触发路径中，解析失败（`src/app.rs:885-902`）在主循环原地 `spawn_llm`，绕过了 `LlmRetry` 事件。将其改为统一走 `LlmRetry` 事件，使退避 + 提示逻辑单一来源。

## Acceptance Criteria

- [ ] `src/app.rs` 中不再有硬编码的重试上限 `3`；改常量一处即全局生效。
- [ ] 重试触发时，UI 状态行可见地显示"第 N/3 次重试"提示（N 随每次重试递增）。
- [ ] 相邻两次重试之间存在可感知的指数退避间隔（2s/4s/8s），而非瞬间连发。
- [ ] 退避等待期间 TUI 主循环不阻塞（界面仍可渲染 spinner、响应输入）。
- [ ] 退避定时器 fire 时若 `phase` 已不再是重试态（如用户已退出、已切到新题），则忽略该次重试（防御性，避免错乱）。
- [ ] 新题到达（`NewQuestion`）时 `llm_retries` 与重试提示状态正确重置。
- [ ] `cargo build` / `cargo clippy`（无新 warning）/ 现有测试通过。

## Definition of Done

- build / clippy / 测试 green。
- 行为变更在 commit message 说明（重试可见性 + 指数退避 + 统一常量）。

## Technical Approach

**常量**：`const MAX_LLM_RETRIES: u32 = 3;`（放 `src/app.rs` 顶部或 `impl App` 旁）。

**Phase 变体**：新增 `QuizPhase::WaitingRetry { attempt: u32, deadline: std::time::Instant }`，表示"退避等待中"；`deadline` 供 UI 每帧算剩余秒数实现倒计时。
- `LlmRetry` 事件处理：`attempt = llm_retries + 1`；若 `attempt > MAX` → 置 `QuizPhase::Error`（已重试 N 次）；否则 `secs = 2 << (attempt-1)`（2/4/8），`deadline = Instant::now() + secs`，`phase = WaitingRetry { attempt, deadline }`，并 `tokio::spawn` 一个 `sleep(secs)` 后发 `AppEvent::LlmRetryFire` 的定时 task（复用 `self.tx`）。
- 新增 `LlmRetryFire` 事件：若当前 `phase` 是 `WaitingRetry` → 清空 `thinking_text`/`answer_text`、`spawn_llm()`、`phase = WaitingLlm`；否则忽略。
- 解析失败路径（`Done → None`）：改为发 `AppEvent::LlmRetry`（不再原地 `spawn_llm`），与空内容/请求失败路径汇聚。
- 空内容 / 请求失败路径（spawn task 内）：保持发 `LlmRetry`，但终止判断从 `current_retries + 1 >= max_retries` 改为引用常量（语义不变）。

**UI**：`src/ui/quiz.rs` 状态行 `match` 增加 `QuizPhase::WaitingRetry { attempt, deadline }` 分支，每帧用 `deadline.saturating_duration_since(Instant::now()).as_secs()` 算剩余秒数，渲染 `↻ 第 {attempt}/{MAX} 次重试，{remaining}s 后重试...`（`Color::Yellow`，实时倒计时）。TUI 每 100ms 重绘驱动倒计时，无需额外定时器。

**退避序列**（已确认）：attempt 1→2s，2→4s，3→8s；最坏情况 3 次重试累计等待 14s。无 jitter（单客户端无需）。

## Decision (ADR-lite)

- **Context**：重试逻辑存在但完全不可见，瞬间连发既不利于用户观察，也不像真实重试节奏。
- **Decision**：保留"对所有错误重试 3 次"的现有语义，新增 `WaitingRetry` phase 承载退避与提示，所有触发路径汇聚到 `LlmRetry`/`LlmRetryFire` 事件对，统一常量。
- **Consequences**：重试有了可感知节奏（最坏 14s）与可见反馈；代价是多一个 phase 变体 + 一个异步定时事件，需保证 fire 时的 phase 一致性检查。

## Out of Scope

- 退避参数（base、上限）可配置化（写入 config）——MVP 用硬编码常量，预留。
- 错误分类（4xx 不重试 / 5xx 重试）——用户澄清 HTTP 400 系测试触发，本次保留全重试。
- `SubmitFail`（提交失败）重试——不在本次范围。

> 追加（原 MVP 外，已实现）：重试倒计时 —— `WaitingRetry` 存 `deadline: Instant`，UI 每帧算剩余秒数（8→7→…→0），由 TUI 100ms 重绘驱动。

## Technical Notes

- `src/app.rs`：`spawn_llm`（582-649）、`AppEvent` 枚举（~121）、`QuizPhase` 枚举（35-60）、事件处理 `LlmChunk::Done`（878-903）、`LlmErr`/`LlmRetry`（909-917）、字段 `thinking_text`/`answer_text`/`llm_retries`（187-211）。
- `src/ui/quiz.rs`：phase-specific status line（287-310），WaitingLlm 渲染（289-294）。
- `src/llm/openai.rs`：`LlmChunk` 枚举（8-14）、错误产生（79-99）。
- 退避用确定性 `2^(attempt-1)`，无需随机数。
