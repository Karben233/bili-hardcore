# Error Handling

> How errors are handled in this project.

---

## Overview

Errors are defined as an `AppError` enum using `thiserror` derive macros. Errors propagate via `?` operator and `Result<T, AppError>`. The TUI catches errors at the event loop level and displays them to the user.

---

## Error Types

```rust
// src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("API error {code}: {message}")]
    Api { code: i64, message: String },

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}
```

---

## Error Handling Patterns

### Pattern 1: API response → AppError

Bilibili API calls check the `code` field and convert non-zero responses:

```rust
if code != 0 {
    return Err(AppError::Api { code, message });
}
```

### Pattern 2: Event-driven retry with exponential backoff (LLM)

LLM requests retry on failure — empty content, request error, or unparseable reply. Retry is **event-driven** (not an in-task `for` loop) so the TUI can render a visible retry state and the backoff timer runs without blocking the event loop.

**Retry parameters**: `App::MAX_LLM_RETRIES = 3` retries (excluding the first request); backoff `secs = 2u64 << (attempt-1)` → 2s / 4s / 8s.

**Flow**:
1. Every failure path sends `AppEvent::LlmRetry { reason }` — it does **not** decide the retry limit. Sources: empty `LlmChunk::Done`, `LlmChunk::Error`, and an unparseable `Done` reply.
2. The `LlmRetry` handler computes `attempt = llm_retries + 1`:
   - `attempt > MAX_LLM_RETRIES` → give up, set `QuizPhase::Error("AI 回答错误: {reason}（已重试 N 次）")`.
   - otherwise → clear streaming text, compute `secs = 2u64 << (attempt-1)` and `deadline = Instant::now() + secs`, set `QuizPhase::WaitingRetry { attempt, deadline }`, and `tokio::spawn` a `sleep(secs)` task that sends `AppEvent::LlmRetryFire`.
3. `LlmRetryFire` re-checks the phase is still `WaitingRetry` (via `matches!`) before `spawn_llm()`; a stale fire (user navigated away / new question arrived) is ignored.
4. `llm_retries` resets to 0 on each `QuestionReady`.

```rust
// src/app.rs — LlmRetry handler (main event loop)
AppEvent::LlmRetry { reason } => {
    let attempt = self.llm_retries + 1;
    if attempt > Self::MAX_LLM_RETRIES {
        self.phase = QuizPhase::Error(format!(
            "AI 回答错误: {}（已重试 {} 次）", reason, Self::MAX_LLM_RETRIES));
    } else {
        self.llm_retries = attempt;
        let secs = 2u64 << (attempt - 1); // 2 / 4 / 8
        self.thinking_text.clear();
        self.answer_text.clear();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
        self.phase = QuizPhase::WaitingRetry { attempt, deadline };
        let tx = self.tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
            let _ = tx.send(AppEvent::LlmRetryFire);
        });
    }
}
```

**Why event-driven, not an in-task loop**: the backoff `sleep` must not block the TUI event loop, and the user must see the retry state. Centralizing the limit decision in the `LlmRetry` handler also means all failure paths converge regardless of origin.

**Countdown rendering**: `WaitingRetry` stores `deadline: Instant` (not a fixed second count). The status line renders `↻ 第 N/3 次重试，{remaining}s 后重试...` where `remaining = deadline.saturating_duration_since(Instant::now()).as_secs()`, recomputed every frame; the TUI's 100ms redraw tick drives the countdown, so no extra timer or tick counter is needed.

**Stale-fire guard**: background timers are not cancelled on navigation (see Pattern 3). The `matches!(self.phase, QuizPhase::WaitingRetry { .. })` check on `LlmRetryFire` prevents a late fire from launching a stray request after the user left or a new question arrived.

### Pattern 3: Background task lifecycle (fire-and-forget via channel)

All background work uses `tokio::spawn` + `mpsc::UnboundedSender<AppEvent>` to communicate results back to the TUI loop. Tasks are **not** tracked with `JoinHandle` — they run until completion and send a single result event.

**Critical: ESC cleanup**. When the user presses ESC to exit a screen, the app calls `self.back()` which changes `self.page`. Spawned tasks continue running but their events are handled by the new page's event handler, which typically ignores irrelevant events or treats them as a no-op.

```rust
// src/app.rs — spawn pattern
pub fn spawn_fetch_question(&self) {
    let tx = self.tx.clone();
    tokio::spawn(async move {
        // ... async work ...
        let _ = tx.send(AppEvent::QuestionReady { ... });
    });
}
```

**Key invariant**: `self.tx` is always available because it's created once at app startup. The `let _ = tx.send(...)` pattern silently drops send errors if the receiver is gone (app shutting down).

**Gotcha**: There is no task cancellation on ESC. If a long-running task (e.g., LLM request) completes after the user navigated away, the event arrives at the main loop but may be ignored based on current page/phase state. Future improvement: use `tokio::select!` with a cancellation token.

### Pattern 4: TUI error display

Errors from background tasks are sent via `mpsc` channel as `AppEvent` variants and displayed in the UI. The app never crashes — errors are shown as messages.

---

## Bilibili API Error Codes

| Code | Meaning | Handling |
|------|---------|----------|
| `0` | Success | Return `data` field |
| `41099` | Daily quiz limit reached (3 attempts/day) | Show error to user |
| `41103` | Submission error | Log error, stop quiz |
| Other | Unknown error | Show to user with code |

---

## Forbidden Patterns

- **Don't use `unwrap()` on fallible operations** — use `?`, `map_err`, or explicit match
- **Don't silently ignore errors** — always propagate or log
- **Don't panic in async tasks** — return `Err` so the TUI can display it

---

## Common Mistakes

- Forgetting to check Bilibili API `code` field before accessing `data` — non-zero responses may lack it
- Using `unwrap()` on channel `recv()` — prefer handling `Err` gracefully
