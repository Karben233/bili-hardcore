# 历史记录持久化得分（修复重新进入答题界面得分为0）

## Goal

重新进入答题界面时，累计得分（`score`）与正确率被重置为 0。根因是 `LevelOk` 事件处理里硬编码 `self.score = 0`，而历史记录里也没有保存得分，无法恢复。

目标：把每题答完后的累计得分记录到历史记录里，并在重新进入答题时从历史记录恢复，使「得分 / 正确率」显示与上次离开时一致。

## What I already know

- `score: i64`（[app.rs:185](src/app.rs#L185)）是 B站服务器返回的累计答对数；`SubmitOk { score }` 时 `correct = score > self.score`（[app.rs:936-938](src/app.rs#L936-L938)）。
- `LevelOk` 强制 `self.score = 0`（[app.rs:805](src/app.rs#L805)）——根因。
- `question_num`（UI 题号）来自服务器 `QuestionReady{num}`（[app.rs:812-818](src/app.rs#L812-L818)），重新进入时服务器会返回正确题号，**不**被重置。
- 正确率 = `score / (num-1)`（非结果阶段，[ui/quiz.rs:172-185](src/ui/quiz.rs#L172-L185)）。恢复 score 后，正确率会自动正确。
- 历史写入点：`ShowingResult` 倒计时结束时 `self.history.push(...)`（[app.rs:359-366](src/app.rs#L359-L366)），随后 `config::save_history`。
- 历史存取：`~/.bili-hardcore/history.json`，`load_history` 用 `serde_json::from_str(...).unwrap_or_default()`（[config.rs:168-186](src/config.rs#L168-L186)）。新字段加 `#[serde(default)]` 即可向后兼容旧文件。
- `NeedCaptcha` 会清空历史（[app.rs:828-834](src/app.rs#L828-L834)）——新一轮答题会重置，恢复逻辑对空历史应返回 0。

## Requirements

- 重新进入答题（`LevelOk`）时，从历史记录恢复 `self.score`，而非硬编码 0。
- 恢复值 = 历史中 `correct==true` 的计数（与服务器累计得分恒等）；空历史 → 0。
- 向后兼容：旧 `history.json` 无任何新字段时也能正确恢复。
- 恢复后，下次提交答案（`SubmitOk`）仍以服务器返回值覆盖，保证权威性。

## Acceptance Criteria

- [ ] `LevelOk` 从历史恢复 `self.score`：空历史 → 0；有历史 → `correct==true` 计数。
- [ ] 旧 `history.json`（无新字段）加载与恢复均正确、不报错。
- [ ] 重新进入答题界面，得分/正确率与上次离开时一致；下次提交答案仍以服务器值覆盖。
- [ ] `cargo build` / `cargo clippy` / `cargo fmt` 通过。

## Definition of Done

- Lint / typecheck / clippy 绿。
- 行为变更无需额外文档（内部状态持久化）。
- 重新进入答题界面，得分/正确率与上次离开时一致。

## Technical Approach

**不新增字段，加载时从历史派生**（用户决策）：

- `LevelOk` 把 `self.score = 0` 改为从历史计算累计答对数：
  ```rust
  self.score = self.history.iter().filter(|h| h.correct).count() as i64;
  ```
- 依据：`correct` 字段本就由服务器累计 `score` 派生（`correct = score > self.score`，[app.rs:937](src/app.rs#L937)），所以 `correct==true` 的计数恒等于累计得分。对旧 `history.json`（无任何新字段）也立即生效。
- `HistoryItem` 结构体、写入点、`config.rs`、`ui/quiz.rs` 均**不改**。
- 下次 `SubmitOk` 仍以服务器返回值覆盖，恢复值仅用于刷新前的展示。

仅改动 `src/app.rs` 一个文件、一处（`LevelOk`）。

## Decision (ADR-lite)

- **Context**：得分显示为 0 有两种成因——`LevelOk` 重置 + 历史未存得分。
- **Decision**：不持久化得分字段，`LevelOk` 时用 `history` 中 `correct==true` 计数恢复 `score`（与服务器累计得分恒等）。
- **Consequences**：零 schema 变更、向后兼容旧文件、旧历史立即生效；代价是「得分」是派生值而非落盘权威值，但因 `correct` 由服务器 `score` 派生，二者始终一致，可接受。

## Out of Scope

- 不改 `NeedCaptcha` 清空历史的行为。
- 不改 `Finished` 终局面板（其 score/scores 来自服务器 `QuizDone`，无需持久化）。
- 不做历史记录 UI 展示每题得分。

## Technical Notes

- 关键文件：[src/app.rs](src/app.rs)（结构体 149-158、写入 359-366、`LevelOk` 805、`SubmitOk` 936-938）、[src/config.rs](src/config.rs#L168-L186)、[src/ui/quiz.rs](src/ui/quiz.rs#L172-L185)。
- `score` 语义 = 累计答对数（由正确率公式 `score/answered` 反推），与 `correct` 计数一致。
