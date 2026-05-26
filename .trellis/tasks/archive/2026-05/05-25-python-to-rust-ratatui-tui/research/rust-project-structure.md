# Research: Rust Project Structure for TUI Application

- **Query**: Recommended Rust project structure for a TUI application replacing a Python CLI tool (bili-hardcore)
- **Scope**: Mixed (internal codebase analysis + external Rust ecosystem research)
- **Date**: 2025-05-25

## Findings

### 1. Recommended Cargo Project Layout

Based on the current Python structure and Rust conventions:

```
bili-hardcore-rs/
+-- Cargo.toml
+-- src/
|   +-- main.rs              # Entry point: CLI parsing, TUI bootstrap
|   +-- lib.rs               # Re-exports, library root
|   +-- app/
|   |   +-- mod.rs           # AppState, event handling, page routing
|   |   +-- state.rs         # Application state struct
|   |   +-- event.rs         # Event types and handler
|   |   +-- pages/
|   |       +-- mod.rs
|   |       +-- login.rs     # QR code login page
|   |       +-- quiz.rs      # Quiz/answer page
|   |       +-- result.rs    # Score result page
|   +-- api/
|   |   +-- mod.rs           # API client facade
|   |   +-- signing.rs       # MD5 app signing (appsign equivalent)
|   |   +-- bilibili.rs      # Bilibili API endpoints (login, senior, user)
|   |   +-- ticket.rs        # HMAC-SHA256 ticket generation
|   +-- llm/
|   |   +-- mod.rs           # LLM client facade
|   |   +-- openai.rs        # OpenAI-compatible API client
|   +-- config/
|   |   +-- mod.rs           # Config loading/saving
|   |   +-- auth.rs          # Auth data persistence
|   |   +-- openai_config.rs # OpenAI config persistence
|   +-- crypto/
|   |   +-- mod.rs           # Crypto utilities facade
|   |   +-- md5_sign.rs      # MD5 parameter signing
|   |   +-- hmac_ticket.rs   # HMAC-SHA256 ticket signing
|   +-- ui/
|   |   +-- mod.rs           # UI rendering root
|   |   +-- components.rs    # Shared UI components
|   +-- error.rs             # Unified error types
+-- tests/
|   +-- signing_test.rs      # Signing algorithm verification tests
+-- logs/                    # Runtime log directory
```

**Rationale**: The Python code has 5 clear modules (client, config, tools, scripts, tools/LLM). The Rust layout maps each to a dedicated directory with explicit `mod.rs` files. The `api/` directory consolidates the Python `client/` and `tools/request_b.py` since they are tightly coupled (API calls + signing). The `crypto/` directory isolates the signing algorithms so they can be tested independently and compared byte-for-byte against the Python version.

### 2. Module Organization for Low Coupling / High Cohesion

**Current Python coupling map** (from analyzing imports):

| Python Module | Depends On |
|---|---|
| `config/config.py` | nothing (global state) |
| `tools/request_b.py` | `config.config` (API_CONFIG, HEADERS) |
| `tools/bili_ticket.py` | `requests`, `hmac`, `hashlib` |
| `tools/LLM/openai.py` | `config.config` (PROMPT, keys), `requests` |
| `client/login.py` | `tools.request_b` (get, post) |
| `client/senior.py` | `tools.request_b`, `config.config` |
| `client/user_info.py` | `tools.request_b`, `config.config` |
| `scripts/login.py` | `client.login`, `tools.bili_ticket`, `tools.request_b`, `config.config` |
| `scripts/start_senior.py` | `client.senior`, `tools.LLM.openai` |

**Rust dependency direction** (recommended):

```
main.rs -> app/ -> api/ -> crypto/
                  llm/
         config/
         ui/
         error.rs
```

Key principles:
- `crypto/` depends on nothing external except `md-5`, `hmac`, `sha2` crates
- `config/` depends on `serde`, `serde_json`, `dirs` only
- `api/` depends on `crypto/`, `reqwest`, `serde_json`
- `llm/` depends on `reqwest`, `serde_json`
- `app/` depends on `api/`, `llm/`, `config/`, `ui/`
- `ui/` depends on `ratatui` only (no business logic)
- `error.rs` is a leaf module, depended on by all others

### 3. Error Handling Patterns

**Recommended: layered approach using `thiserror` + `anyhow`**

```rust
// error.rs -- domain-specific errors with thiserror
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),
    #[error("Authentication error: {0}")]
    Auth(String),
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Signing failed: {0}")]
    Signing(String),
    #[error("API returned error code {code}: {message}")]
    ApiResponded { code: i32, message: String },
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found")]
    NotFound,
    #[error("Invalid config: {0}")]
    Invalid(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

**Where to use `anyhow`**: In `main.rs` and binary-only code where you just need to propagate and display errors.

**Where to use `thiserror`**: In library modules (`api/`, `config/`, `llm/`) where callers need to match on specific error variants.

**Critical consideration**: The Python code uses `response.json()` which can fail silently. In Rust, `serde_json::from_str()` returns `Result`, so every API response parse must be handled explicitly. This is a significant behavioral difference from Python.

### 4. Configuration Management Crates

**Current Python config behavior** (from `config/config.py`):

1. OpenAI config: JSON file at `~/.bili-hardcore/openai_config.json` with `base_url`, `model`, `api_key`
2. Auth data: JSON file at `~/.bili-hardcore/auth.json` with `access_token`, `csrf`, `mid`, `cookie`
3. Hardcoded API config: `appkey`, `appsec`, `user_agent` in source code
4. CLI args: optional `program [url] [model] [apikey]` override

**Recommended Rust crates and approach**:

| Concern | Crate | Version | Purpose |
|---|---|---|---|
| JSON serialization | `serde` + `serde_json` | 1.0.228 | Deserialize/serialize config files |
| CLI argument parsing | `clap` | 4.6.1 | Replace `sys.argv` parsing |
| XDG/home directories | `dirs` | 6.0.0 | Replace `os.path.expanduser('~')` |

**Config structs**:

```rust
// config/openai_config.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

// config/auth.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthData {
    pub access_token: String,
    pub csrf: String,
    pub mid: String,
    pub cookie: String,
}

// config/mod.rs -- hardcoded API config
pub const APP_KEY: &str = "783bbb7264451d82";
pub const APP_SEC: &str = "2653583c8873dea268ab9386918b1d65";
pub const USER_AGENT: &str = "Mozilla/5.0 BiliDroid/1.12.0 (bbcallen@gmail.com)";
```

**Config directory**: Use `dirs::home_dir()` + `.bili-hardcore/` to match Python behavior exactly. The config directory path must remain `~/.bili-hardcore/` for backward compatibility if users have existing configs.

### 5. Logging in Rust TUI Apps

**Problem**: Standard logging writes to stdout/stderr, which interferes with the terminal alternate screen used by Ratatui.

**Recommended approach**: `tracing` + `tracing-appender` (file-only logging)

| Crate | Version | Purpose |
|---|---|---|
| `tracing` | 0.1 (latest stable) | Structured logging facade |
| `tracing-subscriber` | 0.3.23 | Subscriber implementation |
| `tracing-appender` | 0.2.5 | File appender (non-blocking) |

**Key pattern**: Initialize logging to file ONLY, never to stdout/stderr while TUI is active.

```rust
use tracing_subscriber::{fmt, EnvFilter};
use tracing_appender::non_blocking;

fn setup_logging() -> non_blocking::WorkerGuard {
    let log_dir = dirs::home_dir()
        .unwrap()
        .join(".bili-hardcore")
        .join("logs");
    std::fs::create_dir_all(&log_dir).ok();

    let file_appender = tracing_appender::rolling::daily(&log_dir, "bili-hardcore.log");
    let (non_blocking, guard) = non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive("bili_hardcore=debug".parse().unwrap()))
        .with_writer(non_blocking)
        .with_ansi(false)  // No ANSI codes in log files
        .init();

    guard  // Must be kept alive for the program's lifetime
}
```

**Critical**: The `WorkerGuard` returned by `non_blocking()` must be held in `main()` for the entire program lifetime. Dropping it flushes and closes the log writer. Also, `with_ansi(false)` is important for log files.

**Why `tracing` over `log`**: `tracing` provides structured spans which are useful for tracking async API call chains. It is also the de-facto standard in the Rust async ecosystem (tokio ecosystem uses tracing).

### 6. Cryptographic Operations in Rust

**Two signing algorithms must match the Python version exactly**:

#### Algorithm 1: MD5 App Signing (from `tools/request_b.py:appsign`)

Python logic (lines 25-43):
```python
params['ts'] = str(int(time.time()))
params['appkey'] = appkey
params = dict(sorted(params.items()))
query = urllib.parse.urlencode(params)
sign = hashlib.md5((query + appsec).encode()).hexdigest()
params['sign'] = sign
```

**Rust equivalent crates**:

| Crate | Version | Purpose |
|---|---|---|
| `md-5` | 0.11.0 | MD5 hashing (RustCrypto project) |
| `urlencoding` | 2.x | URL encoding to match `urllib.parse.urlencode` |

**Important details for exact match**:
- `urllib.parse.urlencode` uses `key=value&key2=value2` format with `=` and `&` separators
- `urllib.parse.urlencode` percent-encodes special characters
- Python `dict(sorted(params.items()))` sorts by key alphabetically
- All parameter values are strings
- The sign is computed as `md5(sorted_query_string + appsec)` and appended to params

```rust
// crypto/md5_sign.rs
use md5::{Md5, Digest};
use std::collections::BTreeMap;  // BTreeMap is sorted by key

pub fn appsign(params: &mut BTreeMap<String, String>, appkey: &str, appsec: &str) {
    params.insert("ts".into(), format!("{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()));
    params.insert("appkey".into(), appkey.into());

    // BTreeMap iteration is already sorted by key
    let query: String = params.iter()
        .map(|(k, v)| format!("{}={}", k, v))  // Must match urllib.parse.urlencode
        .collect::<Vec<_>>()
        .join("&");

    let sign_input = format!("{}{}", query, appsec);
    let mut hasher = Md5::new();
    hasher.update(sign_input.as_bytes());
    let result = hasher.finalize();
    let sign = format!("{:x}", result);

    params.insert("sign".into(), sign);
}
```

**WARNING**: `urllib.parse.urlencode` also percent-encodes values. If any parameter value contains special characters (spaces, Chinese characters, etc.), the Rust code must apply the same percent-encoding. For the current parameter set (all ASCII-safe values), simple string concatenation should work, but this must be verified with tests.

#### Algorithm 2: HMAC-SHA256 Ticket Signing (from `tools/bili_ticket.py`)

Python logic (lines 6-26, 28-42):
```python
key = "XgwSnGZ1p"
message = f"ts{int(time.time())}"
hmac_obj = hmac.new(key.encode(), message.encode(), hashlib.sha256)
hash_hex = hmac_obj.digest().hex()
```

**Rust equivalent crates**:

| Crate | Version | Purpose |
|---|---|---|
| `hmac` | 0.13.0 | HMAC implementation (RustCrypto) |
| `sha2` | 0.11.0 | SHA-256 hash |

```rust
// crypto/hmac_ticket.rs
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn generate_ticket() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let message = format!("ts{}", ts);
    let key = "XgwSnGZ1p";

    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).unwrap();
    mac.update(message.as_bytes());
    let result = mac.finalize();
    let code_bytes = result.into_bytes();

    hex::encode(code_bytes)  // "hex" crate for hex encoding
}
```

**Additional crate needed**: `hex` for hex encoding (to match Python's `.hex()`).

#### Verification strategy

The signing algorithms are the most critical part of the migration. Every API call depends on correct signing. The recommended approach:
1. Capture actual Python-signed requests (params + sign) as test fixtures
2. Write Rust unit tests that reproduce the exact same output for the same input
3. The `ts` parameter makes direct comparison tricky -- consider making `ts` injectable for testing:

```rust
pub fn appsign_with_ts(params: &mut BTreeMap<String, String>, appkey: &str, appsec: &str, ts: u64) {
    params.insert("ts".into(), ts.to_string());
    // ... rest of signing
}
```

### 7. State Management in Ratatui App (AppState Pattern)

**Ratatui version**: 0.30.0 (latest stable)
**Backend**: crossterm 0.29.0 (cross-platform terminal control)

**Recommended architecture**: The Elm/TEA (The Elm Architecture) pattern, which Ratatui's official examples use.

```rust
// app/state.rs
pub struct AppState {
    // Navigation
    pub current_page: Page,

    // Auth state
    pub access_token: Option<String>,
    pub csrf: Option<String>,
    pub mid: Option<String>,
    pub cookie: Option<String>,

    // Login page state
    pub qr_code_url: Option<String>,
    pub qr_code_ascii: Option<String>,  // Pre-rendered ASCII art
    pub auth_code: Option<String>,
    pub login_polling: bool,

    // Quiz page state
    pub question: Option<String>,
    pub answers: Vec<Answer>,
    pub question_id: Option<u64>,
    pub question_num: u32,
    pub current_score: u32,
    pub ai_answer: Option<String>,
    pub is_waiting_for_ai: bool,

    // Result page state
    pub total_score: Option<u32>,
    pub category_scores: Vec<CategoryScore>,

    // Config
    pub openai_config: Option<OpenAiConfig>,

    // Error display
    pub error_message: Option<String>,
    pub info_message: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Page {
    Login,
    Quiz,
    Result,
}

#[derive(Clone, Debug)]
pub struct Answer {
    pub ans_text: String,
    pub ans_hash: String,
}

#[derive(Clone, Debug)]
pub struct CategoryScore {
    pub category: String,
    pub score: u32,
    pub total: u32,
}
```

**Event handling pattern**:

```rust
// app/event.rs
use crossterm::event::{Event, KeyCode};

pub enum AppEvent {
    Key(KeyCode),
    Tick,           // Timer tick for polling
    ApiResult(ApiResponse),
    Error(String),
}

pub enum ApiResponse {
    QrCodeGenerated { url: String, auth_code: String },
    LoginSuccess { access_token: String, csrf: String, mid: String, cookie: String },
    QuestionReceived { question: String, answers: Vec<Answer>, id: u64, num: u32 },
    AiAnswer(String),
    AnswerSubmitted { correct: bool, new_score: u32 },
    ResultReceived { score: u32, scores: Vec<CategoryScore> },
}
```

**Main loop structure**:

```rust
// main.rs (simplified)
fn main() -> Result<()> {
    let _guard = setup_logging();

    // Load config
    let config = load_config()?;

    // Setup terminal
    crossterm::terminal::enable_raw_mode()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Init state
    let mut state = AppState::new(config);

    // Event loop
    loop {
        terminal.draw(|f| ui::render(f, &state))?;

        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = crossterm::event::read()? {
                if handle_key_event(key, &mut state) == ShouldExit {
                    break;
                }
            }
        }

        // Process async responses
        handle_polling(&mut state)?;
    }

    // Restore terminal
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}
```

**Async consideration**: The Python code uses blocking `requests.post()` calls. In Rust, you have two options:
1. **Synchronous** (simpler): Use `reqwest::blocking` -- matches Python behavior directly. The TUI will freeze during API calls but this is acceptable for short requests.
2. **Async with tokio** (better UX): Use async `reqwest` with a tokio runtime. The TUI stays responsive during API calls. Polling results arrive via a channel.

Given that the Python version blocks during all API calls and this is acceptable for its use case, starting with synchronous `reqwest::blocking` is the simpler path. Async can be added later.

### 8. Recommended CI/CD for Cross-Platform Rust Builds

**GitHub Actions** is the standard for Rust CI/CD.

**Recommended workflow** for cross-platform builds (Windows, macOS, Linux):

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test
      - run: cargo clippy -- -D warnings

  release:
    needs: test
    if: startsWith(github.ref, 'refs/tags/')
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: bili-hardcore-linux-amd64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: bili-hardcore-windows-amd64.exe
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact: bili-hardcore-macos-amd64
          - os: macos-latest
            target: aarch64-apple-darwin
            artifact: bili-hardcore-macos-arm64
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo build --release --target ${{ matrix.target }}
      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: target/${{ matrix.target }}/release/${{ matrix.artifact }}
```

**Alternative for single-job multi-target**: `cross` tool for cross-compilation, but for the targets above, native runners are simpler and more reliable.

**Key crates summary for CI**: No special cross-compilation crates needed for the 4 targets above (x86_64 Linux/Windows + x86_64/ARM64 macOS). All are first-tier Rust targets.

## Files Found (Current Python Codebase)

| File Path | Description |
|---|---|
| `bili-hardcore/main.py` | Entry point: auth -> validate -> start quiz |
| `bili-hardcore/config/config.py` | Global config: API keys, auth state, prompt template, file paths |
| `bili-hardcore/tools/request_b.py` | HTTP client with MD5 app signing (get/post wrappers) |
| `bili-hardcore/tools/bili_ticket.py` | HMAC-SHA256 ticket generation for Bilibili API |
| `bili-hardcore/tools/logger.py` | Python logging setup (file + console handlers) |
| `bili-hardcore/tools/LLM/openai.py` | OpenAI-compatible API client for quiz answering |
| `bili-hardcore/client/login.py` | QR code auth flow (get QR, poll status) |
| `bili-hardcore/client/senior.py` | Quiz API endpoints (category, captcha, question, answer, result) |
| `bili-hardcore/client/user_info.py` | User info validation (level check) |
| `bili-hardcore/scripts/login.py` | Login orchestration (cache check, QR display, polling loop) |
| `bili-hardcore/scripts/start_senior.py` | Quiz session state machine (QuizSession class) |
| `bili-hardcore/scripts/validate.py` | User level validation (must be level 6) |
| `bili-hardcore/scripts/check_config.py` | Config file existence check and cleanup |
| `requirements.txt` | Python deps: requests, qrcode, urllib3, certifi |

## Code Patterns from Python Version (Must-Preserve)

### API Parameter Ordering (from `tools/request_b.py:25-43`)
- `ts` is injected as `str(int(time.time()))`
- `appkey` is injected after `ts`
- Parameters are sorted alphabetically by key (`dict(sorted(params.items()))`)
- URL encoding uses `urllib.parse.urlencode` (percent-encoding with `&` separator)
- Sign is `md5(query_string + appsec)` as lowercase hex
- Sign is appended as `sign` parameter

### HMAC-SHA256 Ticket (from `tools/bili_ticket.py:6-42`)
- Key: `XgwSnGZ1p` (hardcoded)
- Message: `ts{unix_timestamp}` (no separator between "ts" and the number)
- Output: lowercase hex digest
- POST to `https://api.bilibili.com/bapis/bilibili.api.ticket.v1.Ticket/GenWebTicket`
- Params: `key_id=ec02`, `hexsign={hmac_result}`, `context[ts]={ts}`, `csrf=`

### Cookie Extraction (from `scripts/login.py:104-110`)
- Cookies come from `data.cookie_info.cookies` (array of `{name, value}`)
- `csrf` is extracted from cookie named `bili_jct`
- Full cookie string is `name1=value1;name2=value2` (semicolon separated, no spaces)

### Auth Caching (from `scripts/login.py:13-41`)
- Auth file at `~/.bili-hardcore/auth.json`
- Fields: `access_token`, `csrf`, `mid`, `cookie`
- Expiry: 7 days from file modification time (`os.path.getmtime`)
- On load: sets headers `x-bili-mid` and `cookie` on the request session

### API Response Pattern (from all client/*.py files)
- Success: `code == 0`, data in `.data` field
- Error codes: `41099` = rate limit / daily limit, `41103` = already hardcore member
- All API calls use signed parameters (appsign)

### LLM Integration (from `tools/LLM/openai.py`)
- Endpoint: `{base_url}/chat/completions`
- Extracts `choices[0].message.content` from response
- Sends `enable_thinking: false` and `thinking: {type: "disabled"}` in request body
- Timeout: 30 seconds default
- Answer parsing: tries `int(answer)`, falls back to regex `r'回答[:：]\s*(\d+)'`
- Retry: exponential backoff, max 7 retries

## External References

- [Ratatui 0.30 documentation](https://ratatui.rs/) -- TUI framework, crossterm backend
- [RustCrypto hashes (md-5)](https://github.com/RustCrypto/hashes) -- MD5 implementation
- [RustCrypto HMAC](https://github.com/RustCrypto/MACs) -- HMAC-SHA256 implementation
- [reqwest 0.13](https://docs.rs/reqwest) -- HTTP client (blocking and async)
- [clap 4.x](https://docs.rs/clap) -- CLI argument parser
- [thiserror 2.x](https://docs.rs/thiserror) -- Derive macro for error types
- [tracing](https://docs.rs/tracing) -- Instrumented logging and diagnostics
- [GitHub Actions for Rust](https://github.com/dtolnay/rust-toolchain) -- Standard CI setup

## Crate Dependency Summary (Recommended Cargo.toml)

```toml
[dependencies]
# TUI
ratatui = "0.30"
crossterm = "0.29"

# HTTP
reqwest = { version = "0.13", features = ["blocking", "json"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# CLI
clap = { version = "4", features = ["derive"] }

# Crypto
md-5 = "0.11"
hmac = "0.13"
sha2 = "0.11"
hex = "0.4"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"

# Config / directories
dirs = "6"

# QR code generation (for terminal display)
qrcode = "0.14"

# Error handling
anyhow = "1"
thiserror = "2"
```

## Caveats / Not Found

1. **URL encoding exact match**: The Python `urllib.parse.urlencode` behavior for special characters (Chinese text, spaces, etc.) must be verified against the Rust equivalent. Current API parameters appear to be ASCII-safe, but the `statistics` field contains JSON-as-string which has special characters like `{`, `}`, `"` -- these may or may not be encoded differently by Python vs Rust. Testing with captured real requests is essential.

2. **`context[ts]` parameter**: The ticket API uses `context[ts]` as a parameter key name with square brackets. In Python, `requests.post(url, params=params)` sends this literally. In Rust with reqwest, this should also pass through literally but must be verified.

3. **Session/cookie behavior**: Python `requests.Session` persists cookies across requests. The current code manually extracts and sets cookies as headers. In Rust with `reqwest::blocking::Client`, cookies are NOT persisted by default unless using `cookie_store` feature. The code should use manual header setting to match Python behavior exactly.

4. **Timestamp precision**: Python uses `int(time.time())` which gives seconds as integer. Rust `SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()` also gives seconds as u64. These should match, but the exact moment of capture may differ by a second between ts injection and sign computation. The Python code captures `ts` once per `appsign` call, so the Rust code should do the same.

5. **Async vs sync decision**: The research covers both but the primary recommendation is to start with `reqwest::blocking` for simplicity. If the TUI needs to remain responsive during long LLM API calls (30s timeout), async with tokio may become necessary. This is an architectural decision that affects the entire event loop.

6. **Cross-platform terminal behavior**: crossterm handles cross-platform raw mode and alternate screen, but Windows terminal has historical quirks. The QR code ASCII art rendering may look different on Windows terminals. Testing on all 3 platforms is recommended.
