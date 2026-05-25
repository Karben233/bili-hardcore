# Storage Guidelines

> Data persistence patterns for this project.

---

## Overview

This project has **no database**. All persistent data is stored as JSON files in the user's home directory under `~/.bili-hardcore/`. Bilibili API is the only external data source.

---

## Storage Pattern

### Config directory

```
~/.bili-hardcore/
├── auth.json              # Login credentials (access_token, csrf, mid, cookie)
└── openai_config.json     # OpenAI-compatible config (base_url, model, api_key)
```

### Read pattern

```python
# From config/config.py — load_openai_config()
config_file = os.path.join(os.path.expanduser('~'), '.bili-hardcore', 'openai_config.json')
if os.path.exists(config_file):
    with open(config_file, 'r') as f:
        data = json.load(f)
        return data.get('base_url', ''), data.get('model', ''), data.get('api_key', '')
```

### Write pattern

```python
# From config/config.py — save_openai_config()
os.makedirs(os.path.dirname(config_file), exist_ok=True)
with open(config_file, 'w') as f:
    json.dump({'base_url': base_url, 'model': model, 'api_key': api_key}, f)
```

---

## Auth Token Lifecycle

- **Storage**: `~/.bili-hardcore/auth.json` with fields: `access_token`, `csrf`, `mid`, `cookie`
- **Expiration**: 7 days (checked via file mtime, not token content)
- **Refresh**: Re-login via QR code when expired or missing
- **Cleanup**: `clear_config()` deletes entire `~/.bili-hardcore/` directory

---

## Forbidden Patterns

- **Don't store credentials in the repo** — all user-specific data goes to `~/.bili-hardcore/`
- **Don't read/write config files without `try/except`** — file may be missing or corrupted
- **Don't use pickle or binary formats** — JSON only for human readability

---

## Common Mistakes

- Forgetting `os.makedirs(exist_ok=True)` before writing — will crash on first run
- Checking auth token age by token content instead of file mtime — the current code checks file modification time
