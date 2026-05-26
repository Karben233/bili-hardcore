# Research: Sixel Image Rendering in Rust Ratatui TUI

- **Query**: How to display images using the Sixel protocol in a Rust Ratatui TUI application
- **Scope**: mixed (external crates + internal codebase analysis)
- **Date**: 2026-05-26

## Findings

### 1. Rust Crates for Sixel Image Rendering

| Crate | Version | Description | Relevant For |
|---|---|---|---|
| `ratatui-image` | 9.0.0 (for ratatui 0.29) / 11.0.2 (for ratatui 0.30) | Unified image widget for ratatui supporting Sixel, Kitty, iTerm2, and Halfblocks protocols | Primary crate to use |
| `icy_sixel` | 0.1.x (v9) / 0.5.0 (v11) | 100% Rust Sixel encoder/decoder library (no C dependency) | Used internally by ratatui-image |
| `image` | 0.25.x | Image loading/decoding (PNG, JPEG, etc.) | Used by ratatui-image to load image data |
| `sixel` | 0.3.2 | Safe Rust wrapper for libsixel (C library) | Alternative, requires C lib |

**CRITICAL VERSION COMPATIBILITY**: The project uses `ratatui = "0.29"`. The latest `ratatui-image = "11.0.2"` depends on `ratatui ^0.30.0`. You MUST use `ratatui-image = "9.0.0"` which depends on `ratatui ^0.29.0` and `crossterm ^0.29.0`, matching the project's dependencies.

### 2. ratatui-image 9.0.0 API Overview

#### Cargo.toml Addition

```toml
ratatui-image = "9"
# Note: default features include crossterm and chafa-dyn.
# chafa-dyn requires libchafa at runtime.
# To avoid libchafa dependency, use:
# ratatui-image = { version = "9", default-features = false, features = ["image-defaults", "crossterm"] }
```

#### Key Types

- `picker::Picker` -- auto-detects terminal protocol and font size
- `picker::ProtocolType` -- enum: `Halfblocks`, `Sixel`, `Kitty`, `Iterm2`
- `protocol::Protocol` -- fixed-size image protocol (for `Image` widget)
- `protocol::StatefulProtocol` -- resizable image protocol (for `StatefulImage` widget)
- `Image` -- stateless fixed-size image widget
- `StatefulImage` -- stateful resizable image widget (adapts to render area)

#### Protocol Detection (Picker::from_query_stdio)

```rust
use ratatui_image::picker::Picker;

// Auto-detect terminal capabilities and font size
// MUST be called after entering alternate screen but before reading terminal events
let picker = Picker::from_query_stdio()?;
// picker.protocol_type() returns the detected ProtocolType
// picker.font_size() returns the terminal's font size in pixels
```

The detection process:
1. Guesses protocol from environment variables (TERM, TERM_PROGRAM, etc.)
2. If that fails, queries the terminal with escape sequences (XTVERSION, etc.)
3. Falls back to `Halfblocks` if no graphics protocol is detected

#### Widget Usage (ratatui-image 9.0.0)

```rust
use ratatui_image::{Image, StatefulImage, picker::Picker, protocol::StatefulProtocol, Resize};

// Create picker
let picker = Picker::from_query_stdio()?;

// Load image (from file or bytes)
let dyn_img = image::ImageReader::open("image.png")?.decode()?;
// Or from bytes: image::load_from_memory(&bytes)?

// Fixed-size widget (stateless, non-blocking)
let protocol = picker.new_protocol(dyn_img, size, Resize::Fit(None))?;
let image_widget = Image::new(&protocol);
f.render_widget(image_widget, area);

// Resizable widget (stateful, blocking at render-time)
let state = picker.new_resize_protocol(dyn_img);
let image_widget = StatefulImage::default();
f.render_stateful_widget(image_widget, area, &mut state);
```

### 3. Terminal Sixel Support Status

| Terminal | Sixel Support | Notes |
|---|---|---|
| **Alacritty** | NOT supported | Open issue since 2016, PR exists but not merged |
| **Kitty** | NOT supported (Sixel) | Has its own Kitty Graphics Protocol (supported by ratatui-image) |
| **WezTerm** | Supported | Since 2020-06-20 |
| **Windows Terminal** | NOT supported | No Sixel references in source code |
| **Windows Console** | NOT supported | - |
| **foot** | Supported | Since version 1.2.0 |
| **iTerm2** | Supported | Since version 3.3.0 |
| **xterm** | Supported | With `-ti 340` flag or compiled with Sixel support |
| **xterm.js** | Supported | Requires xterm-addon-image |
| **VS Code** | Supported | Since 1.80 (uses xterm.js with xterm-addon-image) |
| **konsole** | Supported | - |
| **mlterm** | Supported | - |
| **GNOME Terminal** | NOT supported | Blocked on VTE upstream |
| **Terminal.app** (macOS) | NOT supported | - |
| **Ghostty** | Supported | - |
| **tmux** | NOT directly | Use `sixel-tmux` for Sixel passthrough |

Source: https://arewesixelyet.com/

### 4. Downloading and Rendering an Image from URL

The project already has `reqwest` for HTTP. The flow is:

```rust
// 1. Download image bytes
let response = reqwest::get("https://example.com/image.png").await?;
let bytes = response.bytes().await?;

// 2. Decode into DynamicImage
let dyn_img = image::load_from_memory(&bytes)?;

// 3. Create protocol via Picker
let picker = Picker::from_query_stdio()?;
let protocol = picker.new_protocol(dyn_img, desired_size, Resize::Fit(None))?;

// 4. Render with Image widget
let widget = Image::new(&protocol);
f.render_widget(widget, area);
```

### 5. Known Issues and Limitations

#### ratatui-image 9.0.0 Specific

- **Blocking resize**: `StatefulImage` performs resize+encode at render-time, which blocks the UI thread. For responsive UIs, use `thread::ThreadProtocol` to offload to a worker thread. See `examples/thread.rs` and `examples/tokio.rs`.
- **Alternate screen requirement**: `Picker::from_query_stdio()` must be called after entering alternate screen but before reading terminal events. The project calls `ratatui::init()` which enters alternate screen, so call `Picker::from_query_stdio()` after that.
- **Font size detection**: If font size cannot be detected, pixel-based protocols (Sixel, Kitty, iTerm2) may not render correctly. Fallback to Halfblocks works with character cells only.
- **Sixel rendering behavior**: Sixel is "immediate-mode" -- the image is drawn directly to the terminal buffer. The TUI must skip drawing characters over the image area to avoid overwriting it. ratatui-image handles this internally.

#### Terminal-Specific Issues

- **Alacritty**: No Sixel support. Would need to fall back to Kitty protocol (not supported either) or Halfblocks. Halfblocks provides a very low-resolution approximation.
- **Windows Terminal / Windows Console**: No Sixel support. Halfblocks fallback only.
- **Kitty**: Does not support Sixel but has its own graphics protocol. `ratatui-image` auto-detects and uses Kitty protocol on Kitty terminal.
- **tmux**: Sixel passthrough requires special configuration or `sixel-tmux`.
- **SSH / remote sessions**: Sixel support depends on the client terminal, not the remote host. If connecting via SSH from an unsupported terminal, images will not render.

#### Chafa Dependency

- ratatui-image's default features include `chafa-dyn` which requires `libchafa` shared library at compile and runtime.
- If you want to avoid the libchafa dependency (useful for static binaries), disable default features: `default-features = false, features = ["image-defaults", "crossterm"]`.
- Without chafa, the crate still supports Sixel, Kitty, iTerm2, and Halfblocks protocols.

### 6. Integration Plan for This Project

The captcha flow is in `src/ui/quiz.rs` (function `draw_captcha`, line 398). The `CaptchaState` (defined in `src/app.rs`, line 56) holds a `captcha_url` field. The goal would be to:

1. Download the captcha image from `captcha_url` using reqwest (already a dependency)
2. Store the decoded `DynamicImage` or `Protocol` in the `CaptchaState`
3. Render the image in the captcha UI using `Image` or `StatefulImage` widget
4. Fall back to the existing URL display if image rendering is not supported

#### Key Files to Modify

| File Path | Description |
|---|---|
| `Cargo.toml` | Add `ratatui-image = "9"` dependency |
| `src/app.rs` | Add image/protocol fields to `CaptchaState`; add image download in `spawn_fetch_captcha` |
| `src/ui/quiz.rs` | Replace/add image rendering in `draw_captcha` function (line 398) |
| `src/main.rs` | Initialize `Picker` after `ratatui::init()` and pass to app |

### 7. Code Patterns in Existing Codebase

- **Async pattern**: The app uses `tokio::spawn` with `mpsc::UnboundedSender<AppEvent>` for all async operations (see `src/app.rs` lines 294-465). Image downloading should follow this same pattern.
- **UI rendering**: All UI rendering is in `src/ui/quiz.rs`, using Ratatui's `Frame::render_widget` pattern.
- **Captcha flow**: `spawn_fetch_captcha` (line 428) fetches captcha data including URL and token. The image download should be added here or as a separate spawn.

## Caveats / Not Found

- **ratatui-image 9.0.0 vs 11.0.2 API differences**: Version 9.0.0 uses `sixel-bytes` for Sixel encoding while 11.0.2 uses `icy_sixel`. The 9.0.0 Sixel module description says "Uses sixel-bytes to draw image pixels, if the terminal supports the Sixel protocol. Needs the sixel feature." -- need to verify if the `sixel` feature flag is needed in 9.0.0 or if it's included by default.
- **No testing in WSL**: The project is running in WSL2 (Linux 6.6.114.1-microsoft-standard-WSL2). Windows Terminal is the likely host terminal. Windows Terminal does NOT support Sixel. The fallback path (Halfblocks or URL display) must work correctly.
- **Feature flags for 9.0.0**: The exact feature flags for ratatui-image 9.0.0 (whether `sixel` is a separate feature) could not be confirmed via docs.rs. The crate should be tested to verify Sixel works when the terminal supports it.
