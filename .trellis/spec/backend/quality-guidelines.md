# Quality Guidelines

> Code quality standards for backend development.

---

## Overview

Rust project using Cargo. The compiler enforces type safety and error handling. No automated test suite exists yet. Quality is enforced through consistent patterns and `cargo check`/`cargo clippy`.

---

## Required Patterns

### Async function signatures

All async operations return `Result<T, AppError>`:

```rust
pub async fn question_get(
    client: &reqwest::Client,
    access_token: &str,
    csrf: &str,
    cookie: &str,
    category_id: i64,
) -> Result<QuestionData, AppError>
```

### LLM client interface

```rust
impl OpenAiClient {
    pub fn new(base_url: &str, model: &str, api_key: &str) -> Self;
    pub async fn ask(&self, question: &str) -> Result<String, AppError>;
}
```

### TUI event-driven architecture

Background tasks communicate with the UI via `tokio::sync::mpsc`:

```rust
pub enum AppEvent {
    QuizReady(...),
    QuestionLoaded(...),
    AnswerResult(...),
    Error(String),
    // ...
}
```

---

## Forbidden Patterns

- **Don't use `unwrap()` on fallible operations** — use `?` or explicit error handling
- **Don't skip `appsign()`** — all Bilibili API calls must use signed parameters
- **Don't block the TUI event loop** — use `tokio::spawn` for async work
- **Don't store secrets in the repository** — user credentials go to `~/.bili-hardcore/`
- **Don't use `println!`** — it corrupts the TUI display; use `tracing` macros
- **Don't use Tab for focus navigation** — use ↑↓ arrows only (see [Keyboard Handling](./keyboard-handling.md))

---

## UI Help Bar Pattern

Every screen displays a context-sensitive help bar at the bottom showing available keyboard shortcuts.

### Convention

```rust
// At the bottom of each screen's render function:
Paragraph::new("↑↓ 选择分类  空格 勾选  Ctrl+R 刷新  ESC 取消")
    .style(Style::default().fg(Color::DarkGray))
```

### Rules

1. **Always show available actions** — never leave the user guessing
2. **Use Chinese text** for key descriptions (matches the app's user-facing language)
3. **Style with `Color::DarkGray`** — visible but not distracting
4. **Show keys in consistent order**: navigation → action → escape
5. **Update when phase changes** — the help text reflects the current sub-phase

### Examples by Screen

| Screen | Help Bar Text |
|--------|--------------|
| WaitingScan | `B 浏览器打开二维码  Ctrl+R 刷新  ESC 返回` |
| Captcha (categories) | `↑↓ 选择分类  空格 勾选  Ctrl+R 刷新  ESC 取消` |
| Captcha (submit) | `↑↓ 切换  Enter 确认` |
| Answer display | `↑↓ 滚动历史  ESC 退出答题` |

---

## Testing Requirements

No test suite exists. When adding tests:

- Place tests in `#[cfg(test)] mod tests` within each module
- Use `tokio::test` for async test functions
- Mock HTTP calls — don't hit real Bilibili APIs in tests

---

## Code Review Checklist

- [ ] New Bilibili API calls go through `api/client.rs` with proper signing
- [ ] Error messages are descriptive with context
- [ ] `tracing` macros are used instead of `println!`
- [ ] New LLM providers follow the `ask()` interface
- [ ] No secrets or tokens committed to the repo
- [ ] Async work is spawned, not awaited in the TUI loop

---

## Known Tech Debt

- No test suite
- No `cargo clippy` CI configuration
- Some API client functions take many individual parameters instead of a context struct

---

## Common Mistakes

- Using `unwrap()` instead of `?` — the Rust compiler will warn about unused Results
- Awaiting async work directly in the TUI render loop — must `tokio::spawn` and receive via channel
