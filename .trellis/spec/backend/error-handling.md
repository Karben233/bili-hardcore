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

### Pattern 2: Exponential backoff retry (LLM client)

```rust
for attempt in 0..3 {
    match self.ask_inner(question).await {
        Ok(result) => return Ok(result),
        Err(e) if attempt < 2 => {
            tokio::time::sleep(Duration::from_millis(500 * 2u64.pow(attempt as u32))).await;
        }
        Err(e) => return Err(e),
    }
}
```

### Pattern 3: TUI error display

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
