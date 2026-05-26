# Storage Guidelines

> Data persistence patterns for this project.

---

## Overview

This project has **no database**. All persistent data is stored as JSON files in `~/.bili-hardcore/`. Bilibili API is the only external data source.

---

## Storage Pattern

### Config directory

```
~/.bili-hardcore/
├── auth.json              # Login credentials (access_token, csrf, mid, cookie)
└── openai_config.json     # OpenAI-compatible config (base_url, model, api_key)
```

### Rust structs

```rust
// config.rs
#[derive(Serialize, Deserialize)]
pub struct OpenAiConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuthData {
    pub access_token: String,
    pub csrf: String,
    pub mid: String,
    pub cookie: String,
}
```

### Read pattern

```rust
let path = dirs::home_dir().unwrap().join(".bili-hardcore").join("openai_config.json");
if path.exists() {
    let data = fs::read_to_string(&path)?;
    let config: OpenAiConfig = serde_json::from_str(&data)?;
}
```

### Write pattern

```rust
let dir = dirs::home_dir().unwrap().join(".bili-hardcore");
fs::create_dir_all(&dir)?;
let json = serde_json::to_string_pretty(&config)?;
fs::write(dir.join("openai_config.json"), json)?;
```

---

## Auth Token Lifecycle

- **Storage**: `~/.bili-hardcore/auth.json` with fields: `access_token`, `csrf`, `mid`, `cookie`
- **Expiration**: 7 days (checked via file mtime, not token content)
- **Refresh**: Re-login via QR code when expired or missing
- **Cleanup**: Option to delete `~/.bili-hardcore/` directory from TUI

---

## Forbidden Patterns

- **Don't store credentials in the repo** — all user-specific data goes to `~/.bili-hardcore/`
- **Don't use binary formats** — JSON only for human readability
- **Don't hardcode the home directory** — use `dirs::home_dir()` for cross-platform support

---

## Common Mistakes

- Forgetting `fs::create_dir_all()` before writing — will crash on first run
- Checking auth token age by token content instead of file mtime — the current code checks file modification time
