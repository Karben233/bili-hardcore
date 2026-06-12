# Directory Structure

> How backend code is organized in this project.

---

## Overview

Single-crate Rust project using Cargo. The `src/` directory contains all application code, organized into modules by domain. TUI-based application using ratatui + crossterm.

---

## Directory Layout

```
src/
├── main.rs                # Entry point: CLI args, TUI setup, event loop
├── app.rs                 # App state machine, quiz flow orchestration
├── error.rs               # AppError enum (thiserror derive)
├── config.rs              # Config load/save (JSON), prompt template, preset loader, CLI overrides
├── presets.json            # LLM provider preset templates (compile-time include_str!)
├── input.rs               # Keyboard event handling
├── crypto.rs              # B站 API signing (MD5 appsign, HMAC-SHA256 ticket)
├── api/
│   ├── mod.rs             # Re-exports
│   └── client.rs          # B站 HTTP client (reqwest), QR login, quiz APIs
├── ui/
│   ├── mod.rs             # UI dispatcher (renders by App state)
│   ├── home.rs            # Home screen (menu options)
│   ├── config_page.rs     # Config form (base_url, model, api_key input)
│   └── quiz.rs            # Quiz screen (question display, captcha, history)
└── llm/
    ├── mod.rs             # Re-exports
    └── openai.rs          # OpenAI-compatible API client with retry
```

---

## Module Organization

### Where to put new code

| Type | Location | Example |
|------|----------|---------|
| New Bilibili API endpoint | `api/client.rs` | `pub async fn question_get(...)` |
| New LLM provider | `llm/<provider>.rs` | Add module + re-export in `llm/mod.rs` |
| Cryptographic utility | `crypto.rs` | `pub fn hmac_sha256(...)` |
| New UI screen | `ui/<screen>.rs` | Add module + dispatch in `ui/mod.rs` |
| Config field or constant | `config.rs` | `pub const QUIZ_PROMPT_TEMPLATE` |
| Error variant | `error.rs` | Add to `AppError` enum |

### Import conventions

```rust
use crate::api::client::{self as api};
use crate::config::{self, QUIZ_PROMPT_TEMPLATE};
use crate::crypto::{appsign, hmac_sha256};
use crate::error::AppError;
use crate::llm::openai::OpenAiClient;
```

---

## Naming Conventions

- **Files**: `snake_case.rs`
- **Functions**: `snake_case()` — e.g., `question_get()`, `appsign()`
- **Types/Structs**: `PascalCase` — e.g., `AppError`, `OpenAiClient`, `AuthData`
- **Constants**: `UPPER_SNAKE_CASE` — e.g., `QUIZ_PROMPT_TEMPLATE`, `APPKEY`
- **Modules**: `snake_case` — e.g., `api`, `ui`, `llm`

---

## Common Mistakes

- **Don't create new `reqwest::Client` instances** — use the shared client built in `api/client.rs` with proper headers and cookie store
- **Don't hardcode Bilibili API URLs** — keep them in `api/client.rs` functions
- **Don't block the TUI event loop** — use `tokio::spawn` for async work, send results via `mpsc` channel
