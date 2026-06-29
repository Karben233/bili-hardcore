# 修复答题正确率在新题加载时误降的问题

## 背景
答题界面顶部 Gauge 显示「正确率」。用户反馈：每当加载新一道题目时，正确率会立即下降，但新题还没提交答案。

## 根因
正确率计算位于 `src/ui/quiz.rs:172-176`：

```rust
let accuracy = if num > 0 {
    (app.score as f64 / num as f64 * 100.0) as u32   // num = app.question_num
} else { 0 };
```

- 分子 `app.score`：仅在提交答案后（`AppEvent::SubmitOk`，app.rs:936-938）更新，反映**已提交题目**的累计得分。
- 分母 `app.question_num`：在新题加载完成时（`AppEvent::QuestionReady`，app.rs:818）立即更新为**当前题号**。

分子分母口径不一致。时序（第 N 题答对后）：

| 阶段 | score | question_num | 当前正确率 | 应有 |
|---|---|---|---|---|
| 第 N 题 `ShowingResult`（已提交） | 含 N 题 | N | score/N ✅ | score/N |
| 加载第 N+1 题 → `WaitingLlm` | 仍只含 N 题 | 跳到 N+1 | score/(N+1) ❌ 突降 | score/N |
| 第 N+1 题 `SubmitOk` | 含 N+1 题 | N+1 | score/(N+1) ✅ 回升 | — |

加载新题时分母先涨、分子后涨，导致正确率先掉再弹。

## 方案
将正确率分母改为「**已提交答案的题数**」，而非「当前题号」：

- `ShowingResult` 阶段：当前题已提交 → 已提交题数 = `question_num`
- `WaitingLlm` / `WaitingRetry` / `Submitting` 阶段：当前题尚未提交 → 已提交题数 = `question_num.saturating_sub(1)`
- 已提交题数为 0 时（如第 1 题加载后尚未作答）正确率显示 0%

仅修改 `src/ui/quiz.rs` 中 accuracy 的计算一处。不引入新状态字段，不改动事件机/状态机。

## 验收标准
1. 第 N 题 `ShowingResult` 期间，正确率 = score/N（与现有行为一致）。
2. 加载第 N+1 题后、提交前（`WaitingLlm` / `Submitting`），正确率保持 = score/N，**不再突降**。
3. 第 1 题加载后未作答时正确率 = 0%（无回归）。
4. `cargo check` 与 `cargo clippy` 通过，不引入 `unwrap()` / `println!`。

## 影响范围
- `src/ui/quiz.rs`：accuracy 计算逻辑（1 处）
