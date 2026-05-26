# Logging Guidelines

> How logging is done in this project.

---

## Overview

Logging uses the `tracing` crate with `tracing-appender` for non-blocking file output. Logs are written to `./logs/bili-hardcore.log`.

---

## Logger Setup

Configured in `main.rs`:

```rust
let file_appender = tracing_appender::rolling::never("./logs", "bili-hardcore.log");
let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
tracing_subscriber::fmt()
    .with_writer(non_blocking)
    .with_target(false)
    .init();
```

- **Library**: `tracing` + `tracing-subscriber` + `tracing-appender`
- **Writer**: Non-blocking file appender (no console output — TUI owns the terminal)
- **Format**: Default subscriber format without target module path

---

## Log Levels

| Level | When to use | Example |
|-------|-------------|---------|
| `info!` | Normal flow progress | `info!("答题开始")`, `info!("第{}题: {}", num, question)` |
| `warn!` | Unexpected but recoverable | `warn!("AI回复了无关内容: [{}]", answer)` |
| `error!` | Operation failures | `error!("获取题目失败: {}", e)` |

---

## What to Log

- Quiz progress: question number, question text, AI answer, correct/wrong
- API interactions: request details, response codes
- Login status: QR code events, login success/failure
- Config actions: key saved, config loaded
- LLM retry: backoff attempts

---

## What NOT to Log

- **API keys** — never log `api_key` values
- **Full auth tokens** — log only "登录成功", not the token value
- **User cookies**

---

## Usage Pattern

```rust
use tracing::{info, warn, error};

info!("当前得分: {}, 正确率: {:.1}%", score, accuracy);
error!("提交失败: {}", result);
```

---

## Forbidden Patterns

- **Don't use `println!` for status** — it conflicts with the TUI rendering
- **Don't log to console** — the terminal is owned by ratatui
- **Don't create additional subscribers** — single global subscriber in main

---

## Common Mistakes

- Using `println!` or `eprintln!` which corrupts the TUI display — always use `tracing` macros
- Dropping the `_guard` too early — it must live for the entire app lifetime or logs stop writing
