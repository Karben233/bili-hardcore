# Research: Rust Ratatui TUI Development Patterns

- **Query**: Research Rust Ratatui TUI development for migrating bili-hardcore Python CLI tool to Rust
- **Scope**: Mixed (internal codebase analysis + external crate/pattern research)
- **Date**: 2025-05-25

## Findings

### 1. Current Python Codebase Summary

The Python app (bili-hardcore) is a bilibili hardcore membership quiz tool with these modules:

| Python File | Description | Rust Equivalent Needed |
|---|---|---|
| `main.py` | Entry point: auth -> validate -> start quiz | `main.rs` entry point |
| `config/config.py` | API config (base_url, model, api_key), bilibili API constants, auth file path | `config.rs` or `settings.rs` |
| `scripts/login.py` | QR code login flow: generate QR, display ASCII, poll status, save auth | Login page module |
| `scripts/validate.py` | Validate user level (must be level 6) | Validation step |
| `scripts/start_senior.py` | Quiz loop: get question, call LLM for answer, submit, track score | Quiz page module |
| `scripts/check_config.py` | Check/load/clear saved config at `~/.bili-hardcore/` | Config management |
| `client/login.py` | Bilibili QR code API calls (get auth_code, poll) | API client module |
| `client/senior.py` | Quiz API calls (get question, submit answer, get result, captcha) | API client module |
| `client/user_info.py` | Get account info | API client module |
| `tools/request_b.py` | HTTP session with retry, bilibili app signing (MD5 hmac) | HTTP client + signing |
| `tools/bili_ticket.py` | HMAC-SHA256 ticket generation | Ticket module |
| `tools/LLM/openai.py` | OpenAI-compatible API client for answering questions | LLM client module |
| `tools/logger.py` | Logging setup | `tracing` or `log` crate |

Key behaviors to replicate:
- Config persisted at `~/.bili-hardcore/openai_config.json` and `~/.bili-hardcore/auth.json`
- QR code displayed as ASCII art in terminal for bilibili login
- HTTP requests use bilibili app signing (params sorted, MD5 hash with app secret)
- CLI args accepted: `program [url] [model] [apikey]`
- Currently packaged as single binary via PyInstaller

### 2. Recommended Rust Crate Stack

#### Core Crates

| Crate | Version | Purpose |
|---|---|---|
| `ratatui` | 0.30.0 | TUI framework (now modular: core + widgets + crossterm backend) |
| `crossterm` | 0.29.0 | Cross-platform terminal backend (ratatui's default) |
| `tokio` | 1.52.x | Async runtime (features: `rt-multi-thread`, `macros`) |
| `reqwest` | 0.13.x | HTTP client (the de-facto Rust HTTP client, 493M+ downloads) |
| `serde` + `serde_json` | latest | JSON serialization for config and API responses |
| `color-eyre` | 0.6.x | Error handling with pretty panic reports (ratatui ecosystem standard) |
| `clap` | 4.x | CLI argument parsing (derives for `url`, `model`, `apikey` args) |
| `dirs` | 6.0.0 | Platform-specific config directories (231M+ downloads) |
| `tracing` + `tracing-subscriber` | latest | Structured logging (ratatui ecosystem standard) |

#### QR Code Crates (Two Options)

| Crate | Version | Downloads | Terminal Support | Notes |
|---|---|---|---|---|
| `qrcode` | 0.14.1 | 14.1M | Yes - `render()` with `.dark_color('#')`, `.light_color(' ')` produces string | Well-established, optional `image` dep. Has Unicode string rendering built-in |
| `fast_qr` | 0.13.1 | 272K | Yes - `.to_str()` and `.print()` methods | 6-7x faster than `qrcode`, simpler API for terminal use |

**Recommendation**: `qrcode` crate for maturity and proven track record. Its string rendering API:
```rust
let code = QrCode::new(url).unwrap();
let string = code.render::<char>()
    .quiet_zone(false)
    .module_dimensions(2, 1)
    .build();
println!("{}", string);
```

#### Text Input Crates

| Crate | Version | Downloads | Description |
|---|---|---|---|
| `ratatui-textarea` | 0.9.1 | 114K | Full-featured multi-line editor widget for ratatui, supports vim-like keybindings |
| `tui-input` | 0.15.3 | 1.36M | Headless input library (no rendering), just handles input state. Pairs with ratatui Paragraph widget |
| `tui-prompts` | latest | - | Interactive prompt widgets for ratatui |
| `edtui` | latest | - | Vim-inspired editor widget |

**Recommendation**: `tui-input` for simple single-line fields (config page base_url/model/api_key), combined with ratatui's `Paragraph` widget for rendering. Use `ratatui-textarea` only if multi-line editing is needed.

#### Async Event Handling

| Crate | Purpose |
|---|---|
| `tokio-stream` | `EventStream` for async crossterm events |
| `futures` | Stream utilities |

Required `crossterm` feature: `event-stream`

### 3. Ratatui 0.30 Architecture (2025/2026)

Ratatui 0.30.0 (released Dec 2025) reorganized into a workspace:

```
ratatui (main crate - re-exports everything)
  -> ratatui-core (Widget traits, Buffer, Layout, Style, Text types)
  -> ratatui-widgets (Block, Paragraph, List, Table, etc.)
  -> ratatui-crossterm (Crossterm backend)
  -> ratatui-macros (convenience macros)
```

Application developers should use the main `ratatui` crate. No changes needed from pre-0.30 code.

### 4. Recommended Project Structure

```
bili-hardcore-rs/
  Cargo.toml
  src/
    main.rs              # Entry point: init terminal, run app
    app.rs               # App struct, main event loop, page routing
    pages/
      mod.rs
      home.rs            # Welcome screen, status display
      config.rs          # API config management (base_url, model, api_key)
      quiz.rs            # Quiz questions, progress, results
      login.rs           # QR code login flow
    client/
      mod.rs
      bilibili.rs        # Bilibili API client (signing, requests)
      llm.rs             # OpenAI-compatible LLM client
    config.rs            # Config file management (load/save JSON)
    crypto.rs            # HMAC signing, MD5 hash for bilibili API
    qr.rs                # QR code generation for terminal display
```

### 5. Multi-Page Navigation Pattern

Ratatui does not have a built-in router. The standard pattern is an enum-based page state:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
enum Page {
    Home,
    Login,
    Config,
    Quiz,
}

struct App {
    current_page: Page,
    should_quit: bool,
    // page-specific state
    login_state: LoginState,
    config_state: ConfigState,
    quiz_state: QuizState,
}

impl App {
    fn render(&self, frame: &mut Frame) {
        match self.current_page {
            Page::Home => self.render_home(frame),
            Page::Login => self.render_login(frame),
            Page::Config => self.render_config(frame),
            Page::Quiz => self.render_quiz(frame),
        }
    }

    fn handle_event(&mut self, event: &Event) {
        match self.current_page {
            Page::Home => self.handle_home_event(event),
            Page::Login => self.handle_login_event(event),
            Page::Config => self.handle_config_event(event),
            Page::Quiz => self.handle_quiz_event(event),
        }
    }
}
```

### 6. Async + Ratatui Integration Pattern

The official async pattern from ratatui's `async-github` example uses `tokio::select!` with `EventStream`:

```rust
use crossterm::event::{Event, EventStream};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::default().run(terminal).await;
    ratatui::restore();
    result
}

impl App {
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let period = Duration::from_secs_f32(1.0 / 60.0);
        let mut interval = tokio::time::interval(period);
        let mut events = EventStream::new();

        // Spawn background tasks here
        // tokio::spawn(async move { ... });

        while !self.should_quit {
            tokio::select! {
                // Render at ~60fps
                _ = interval.tick() => {
                    terminal.draw(|frame| self.render(frame))?;
                },
                // Handle terminal events (keyboard, mouse, resize)
                Some(Ok(event)) = events.next() => {
                    self.handle_event(&event);
                },
            }
        }
        Ok(())
    }
}
```

For background HTTP calls with shared state, the pattern uses `Arc<RwLock<T>>`:

```rust
#[derive(Debug, Clone, Default)]
struct AsyncWidget {
    state: Arc<RwLock<WidgetState>>,
}

impl AsyncWidget {
    fn spawn_fetch(&self) {
        let this = self.clone();
        tokio::spawn(async move {
            *this.state.write().unwrap() = WidgetState::Loading;
            match fetch_data().await {
                Ok(data) => *this.state.write().unwrap() = WidgetState::Loaded(data),
                Err(e) => *this.state.write().unwrap() = WidgetState::Error(e.to_string()),
            }
        });
    }
}
```

For more complex scenarios, use `tokio::sync::mpsc` channels to send messages from background tasks to the main loop:

```rust
// Create a channel
let (tx, mut rx) = tokio::sync::mpsc::channel::<AppMessage>(100);

// In the main loop, add rx to tokio::select!
tokio::select! {
    _ = interval.tick() => { terminal.draw(|frame| self.render(frame))?; },
    Some(Ok(event)) = events.next() => self.handle_event(&event),
    Some(msg) = rx.recv() => self.handle_message(msg),
}
```

Required `Cargo.toml` dependencies for async:
```toml
crossterm = { version = "0.29", features = ["event-stream"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
tokio-stream = "0.1"
```

### 7. Text Input Handling for Config Page

For text input fields (base_url, model, api_key), use `tui-input` crate for state management:

```rust
use tui_input::Input;

struct ConfigState {
    base_url: Input,
    model: Input,
    api_key: Input,
    active_field: usize, // which field is focused
}

impl ConfigState {
    fn handle_key(&mut self, key: KeyEvent) {
        let active_input = match self.active_field {
            0 => &mut self.base_url,
            1 => &mut self.model,
            2 => &mut self.api_key,
            _ => return,
        };
        match key.code {
            KeyCode::Tab => self.active_field = (self.active_field + 1) % 3,
            KeyCode::BackTab => self.active_field = (self.active_field + 2) % 3,
            KeyCode::Enter => self.save_config(),
            _ => {
                // tui-input handles cursor movement, backspace, etc.
                active_input.handle_event(&crossterm::event::Event::Key(key));
            }
        }
    }
}
```

Render with ratatui Paragraph widget, applying visual cursor position from `tui_input::Input::cursor()`.

### 8. QR Code Terminal Display

Using the `qrcode` crate (direct equivalent of Python's `qrcode` library):

```rust
use qrcode::QrCode;

fn render_qr_to_string(url: &str) -> String {
    let code = QrCode::new(url.as_bytes()).unwrap();
    code.render::<char>()
        .quiet_zone(false)
        .module_dimensions(2, 1)
        .dark_color('\u{2588}')  // full block character
        .light_color(' ')
        .build()
}
```

Using `fast_qr` as alternative (simpler API):
```rust
use fast_qr::qr::QRBuilder;

fn render_qr_to_string(url: &str) -> String {
    let qrcode = QRBuilder::new(url).build().unwrap();
    qrcode.to_str()  // returns String with Unicode block chars
}
```

To render inside a ratatui widget, wrap the string in a `Paragraph` or use a custom `Widget` impl that writes the QR text into the `Buffer` at the correct position.

### 9. HTTP Client: reqwest

`reqwest` is the standard Rust HTTP client (493M+ downloads). For this project:

```toml
[dependencies]
reqwest = { version = "0.13", features = ["json"] }
```

Key features needed:
- `json` - for parsing JSON API responses
- Default TLS backend (native-tls or rustls)
- For static binary: use `reqwest` with `rustls-tls` feature to avoid OpenSSL dependency

```toml
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls-tls"] }
```

Retry strategy (matching Python's `urllib3.util.retry.Retry`):
```rust
use reqwest_middleware::{ClientBuilder, RetryTransientMiddleware};
use reqwest_retry::policies::ExponentialBackoff;

let retry_policy = ExponentialBackoff::builder()
    .retry_bounds(Duration::from_secs(1), Duration::from_secs(10))
    .build_with_max_retries(3);
let client = ClientBuilder::new(reqwest::Client::new())
    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    .build();
```

### 10. Cross-Platform Binary Building

#### Option A: cargo-dist (Recommended for distribution)

`cargo-dist` (v0.32.0) by Axodotdev automates building and releasing binaries for multiple targets.

```toml
# In Cargo.toml [workspace.metadata.dist]
[workspace.metadata.dist]
targets = [
    "x86_64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
    "aarch64-unknown-linux-gnu",
]
```

Generates CI workflows (GitHub Actions) that build for all targets and create releases with installers.

#### Option B: cross (for Docker-based cross-compilation)

`cross` uses Docker containers with pre-configured toolchains:
```bash
cross build --target aarch64-unknown-linux-gnu
cross build --target x86_64-pc-windows-msvc
```

#### Option C: Manual cross-compilation

For static Linux binaries (no glibc dependency):
```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

Target triples needed:
- `x86_64-unknown-linux-gnu` or `x86_64-unknown-linux-musl` (Linux)
- `aarch64-unknown-linux-gnu` (Linux ARM64)
- `x86_64-apple-darwin` (macOS Intel)
- `aarch64-apple-darwin` (macOS Apple Silicon)
- `x86_64-pc-windows-msvc` (Windows)
- `aarch64-pc-windows-msvc` (Windows ARM64)

For truly static binary (especially Linux), ensure all deps use `rustls-tls` instead of `native-tls` to avoid OpenSSL linking.

### 11. Cargo.toml Template (Starter)

```toml
[package]
name = "bili-hardcore"
version = "0.1.0"
edition = "2021"
rust-version = "1.80"

[dependencies]
ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
tokio-stream = "0.1"
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
color-eyre = "0.6"
clap = { version = "4", features = ["derive"] }
qrcode = "0.14"
tui-input = "0.15"
dirs = "6"
tracing = "0.1"
tracing-subscriber = "0.3"
md-5 = "0.10"        # for bilibili API signing
hmac = "0.12"        # for ticket generation
```

### 12. Bilibili API Signing in Rust

The Python code uses two signing mechanisms that need Rust equivalents:

**MD5 signing** (for API requests - `tools/request_b.py:appsign`):
```rust
use md5::{Md5, Digest};
use hmac::{Hmac, Mac};

type HmacSha256 = Hmac<Sha256>;

fn app_sign(params: &mut HashMap<String, String>, appsec: &str) {
    params.insert("ts".into(), format!("{}", SystemTime::now()
        .duration_since(UNIX_EPOCH).unwrap().as_secs()));
    params.insert("appkey".into(), APPKEY.into());
    let mut sorted: Vec<_> = params.iter().collect();
    sorted.sort_by_key(|(k, _)| *k);
    let query: String = sorted.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>().join("&");
    let sign = format!("{:x}", Md5::digest(format!("{}{}", query, appsec)));
    params.insert("sign".into(), sign);
}
```

**HMAC-SHA256** (for ticket - `tools/bili_ticket.py`):
```rust
fn hmac_sha256(key: &str, message: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).unwrap();
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}
```

### External References

- [Ratatui GitHub](https://github.com/ratatui/ratatui) - Main repo with examples
- [Ratatui Website](https://ratatui.rs/) - Tutorials and concepts
- [Ratatui Forum](https://forum.ratatui.rs/) - Q&A
- [Ratatui ARCHITECTURE.md](https://github.com/ratatui/ratatui/blob/main/ARCHITECTURE.md) - 0.30 workspace structure
- [Ratatui async-github example](https://github.com/ratatui/ratatui/tree/main/examples/apps/async-github) - Official async pattern
- [Awesome Ratatui](https://github.com/ratatui/awesome-ratatui) - Curated list of widgets and apps
- [Ratatui Templates](https://github.com/ratatui/templates) - Project scaffolding (hello-world, simple, simple-async, event-driven, event-driven-async, component)
- [qrcode crate docs](https://docs.rs/qrcode/0.14.1/qrcode/) - QR code encoder
- [fast_qr crate](https://github.com/erwanvivien/fast_qr) - Faster QR code generation with `.to_str()`
- [tui-input crate](https://crates.io/crates/tui-input) - Headless input state management
- [ratatui-textarea](https://crates.io/crates/ratatui-textarea) - Multi-line text editor widget
- [cargo-dist](https://crates.io/crates/cargo-dist) - Cross-platform binary distribution
- [cross](https://github.com/cross-rs/cross) - Docker-based cross-compilation

### Related Specs

- `.trellis/spec/backend/directory-structure.md` - May need updating for Rust project layout
- `.trellis/spec/backend/error-handling.md` - Error handling patterns for Rust
- `.trellis/spec/backend/quality-guidelines.md` - Quality guidelines

## Caveats / Not Found

- Ratatui website is heavily SPA-based; content extraction via curl was limited. Recommend browsing ratatui.rs directly for tutorials.
- The `qrcode` crate's `image` feature is optional and not needed for terminal display (string rendering is built-in without the `image` dep).
- `reqwest` with `rustls-tls` is preferred for static binary builds, but verify that all TLS features work correctly with bilibili API endpoints.
- For the login QR code polling (1-second interval for 60 retries), consider using `tokio::time::interval` in a spawned task rather than blocking the main event loop.
- The ratatui templates repository URL structure changed; the canonical way to scaffold is `cargo generate ratatui/templates`.
- Windows terminal support for Unicode block characters (QR code rendering) may require Windows Terminal or similar modern terminal; legacy cmd.exe has limited Unicode support.
