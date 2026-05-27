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

### Pattern 2: Exponential backoff retry (spawned tasks)

LLM requests and other flaky operations retry with exponential backoff inside a `tokio::spawn` block. The retry logic lives in the spawned task, not the `ask()` method itself.

**Retry parameters**: max 3 attempts, base delay 500ms, doubles each attempt (500ms → 1s → 2s).

```rust
// src/app.rs — spawn_llm()
tokio::spawn(async move {
    let max_retries = 3u32;
    let mut last_err = String::new();
    for attempt in 0..max_retries {
        match client.ask(&prompt).await {
            Ok(ans) => {
                let _ = tx.send(AppEvent::LlmOk(ans));
                return;
            }
            Err(e) => {
                last_err = e.to_string();
                tracing::warn!("LLM 请求失败 (第{}次): {}", attempt + 1, last_err);
                if attempt + 1 < max_retries {
                    let delay = std::time::Duration::from_millis(500 * 2u64.pow(attempt));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    let _ = tx.send(AppEvent::LlmErr(last_err));
});
```

**Why retry in spawn, not in `ask()`**: The spawned task owns the `tx` channel sender and can report errors. Retrying inside `ask()` would require the caller to re-invoke, complicating the event loop.

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
