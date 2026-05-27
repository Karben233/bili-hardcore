# Keyboard Event Handling

> Unified keyboard navigation conventions for this TUI application.

---

## Overview

This is a ratatui + crossterm TUI app. Keyboard events flow through `input.rs` which dispatches to page-specific handlers based on `self.page`. All navigation uses **arrow keys** (↑↓), never Tab.

---

## Design Decision: Arrow-Only Navigation

**Context**: Previously Tab and Shift+Tab were used alongside arrows for focus switching.

**Options Considered**:
1. Tab + arrows (dual system)
2. Arrow keys only (↑↓)

**Decision**: ↑↓ only for navigation. Tab conflicted with terminal input behavior. ←→ is reserved for text cursor movement only.

**How to apply**: When adding focus navigation to any screen, use `KeyCode::Up`/`KeyCode::Down`. Never bind Tab, vim keys, or ←→ for navigation. ←→ is only for text input cursor movement.

---

## Key Binding Reference

### Global Bindings (all pages)

| Key | Action |
|-----|--------|
| `Esc` | Navigate back / close dialog |
| `Enter` | Confirm / activate focused element |

### Home Page

| Key | Action |
|-----|--------|
| `↑` | Previous menu item |
| `↓` | Next menu item |

### Config Page

| Key | Action |
|-----|--------|
| `↑` / `↓` | Cycle through fields (BaseUrl → Model → ApiKey → SaveBtn → ResetBtn → wraps) |
| `←` / `→` | Move cursor within text field |
| `Backspace` | Delete character before cursor |
| `Char(c)` | Insert character at cursor position |

### Quiz Page

| Key | Action | Phase |
|-----|--------|-------|
| `Esc` | Exit quiz (stops background tasks) | All phases |
| `↑` / `↓` | Scroll answer history | Answer display |
| `Ctrl+R` | Refresh captcha (preserves selections) | Captcha phase |
| `Space` | Toggle category checkbox | Captcha > Categories focus |
| `B` | Open QR code in browser | WaitingScan phase |

### Captcha Sub-page Focus Cycle

```
Submit ←→ Input ←→ OpenBrowser ←→ Categories
    ↑ wraps to                        ↑ wraps to
    └─────────────────────────────────┘
```

---

## Architecture

### Event Dispatch

```rust
// src/input.rs
impl App {
    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.page {
            Page::Home => self.key_home(key.code),
            Page::Config => self.key_config(key.code),
            Page::Quiz => self.key_quiz(key),  // Note: passes full KeyEvent for modifier checks
        }
    }
}
```

### Captcha State Extraction Pattern

The captcha handler uses `std::mem::replace` to extract and rebuild state:

```rust
fn key_captcha(&mut self, key: KeyEvent) {
    let cs = match std::mem::replace(&mut self.phase, QuizPhase::NotConfigured) {
        QuizPhase::Captcha(cs) => cs,
        other => { self.phase = other; return; }
    };
    let cs = match code {
        KeyCode::Up if /* condition */ => CaptchaState { ..cs },
        // ...
        _ => cs,
    };
    self.phase = QuizPhase::Captcha(cs);
}
```

**Why**: Borrow checker requires exclusive access to `self.phase` and `self` methods simultaneously. The replace-then-rebuild pattern avoids this.

---

## Forbidden Patterns

- **Don't use Tab for focus navigation** — conflicts with terminal behavior
- **Don't use F5 for refresh** — use `Ctrl+R` with `key.modifiers.contains(KeyModifiers::CONTROL)` check
- **Don't bind keys without checking phase** — quiz page has many sub-phases; always match on `self.phase` first

---

## Common Mistakes

- Forgetting to check `key.modifiers` for Ctrl+R — must match `KeyCode::Char('r')` AND `KeyModifiers::CONTROL`
- Using `key.code` when modifiers are needed — `key_quiz` receives the full `KeyEvent`, not just `KeyCode`
- Not wrapping focus at boundaries — captcha focus cycles (Submit → Categories), config fields wrap (ResetBtn → BaseUrl)
