# Directory Structure

> How backend code is organized in this project.

---

## Overview

Single-package Python project. The `bili-hardcore/` directory is the sole source package. No separate test, docs, or infra directories exist.

---

## Directory Layout

```
bili-hardcore/
├── main.py                    # Entry point: auth → validate → start
├── config/
│   └── config.py              # All configuration, API keys, prompts (module-level I/O)
├── tools/
│   ├── logger.py              # Global logger instance
│   ├── request_b.py           # Shared requests.Session with retry + app signing
│   ├── bili_ticket.py         # HMAC-SHA256 ticket generation
│   └── LLM/
│       └── openai.py          # OpenAI-compatible API client (base_url, model, api_key)
├── client/
│   ├── login.py               # QR code login (qrcode_get, qrcode_poll)
│   ├── senior.py              # Quiz APIs (category, captcha, question, answer)
│   ├── user_info.py           # Account info lookup
│   └── ziantt.py              # External question DB submission
├── scripts/
│   ├── login.py               # Auth flow (load/save/cache credentials)
│   ├── start_senior.py        # QuizSession class + main quiz loop
│   ├── check_config.py        # Config file check/clear
│   └── validate.py            # User level validation (must be level 6)
├── logs/                      # Runtime log files (timestamped, auto-created)
└── __init__.py
```

---

## Module Organization

### Where to put new code

| Type | Location | Example |
|------|----------|---------|
| New Bilibili API endpoint | `client/<domain>.py` | `client/senior.py` for quiz APIs |
| New LLM provider | Configure in `tools/LLM/openai.py` | Add base_url + model via OpenAI-compatible format |
| Utility / shared function | `tools/<name>.py` | `tools/bili_ticket.py` |
| High-level workflow script | `scripts/<name>.py` | `scripts/start_senior.py` |
| Config constants | `config/config.py` | `API_CONFIG`, `HEADERS`, `PROMPT` |

### Import conventions

```python
# Relative imports within the package
from tools.request_b import get, post
from config import config
from tools.logger import logger
from client.senior import question_get, question_submit
```

Imports use the package-relative form (`from tools.X import Y`), not absolute (`from bili_hardcore.tools.X import Y`).

---

## Naming Conventions

- **Files**: `snake_case.py`
- **Functions**: `snake_case()` — e.g., `question_get()`, `captcha_submit()`
- **Classes**: `PascalCase` — e.g., `QuizSession`, `OpenAIAPI`
- **Constants**: `UPPER_SNAKE_CASE` at module level — e.g., `API_CONFIG`, `HEADERS`, `PROMPT`
- **Config files** (JSON): `~/.bili-hardcore/<name>.json` — e.g., `openai_config.json`, `auth.json`

---

## Common Mistakes

- **Don't put business logic in `config/config.py`** — it runs interactive I/O (input/print) at module import time. Any import triggers the API config prompt.
- **Don't create new `requests.Session()` instances** — use the shared `session` from `tools/request_b.py` which has retry configured.
- **Don't hardcode Bilibili API URLs** — keep them in the `client/` layer functions where they already live.
