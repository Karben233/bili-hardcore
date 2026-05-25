# Quality Guidelines

> Code quality standards for backend development.

---

## Overview

This is a small Python project with no automated test suite, no CI/CD, and no linter configuration. Quality is enforced through consistent patterns and code review.

---

## Required Patterns

### Import structure

```python
# 1. stdlib
import os
import json
import time

# 2. third-party
import requests
from qrcode.main import QRCode

# 3. local package
from tools.request_b import get, post
from config import config
from tools.logger import logger
```

### Function signatures

All Bilibili API wrappers follow the same pattern:

```python
def api_call():
    '''Chinese docstring describing the function'''
    res = get_or_post(url, params)
    if res and res.get('code') == 0:
        return res.get('data')
    else:
        raise Exception('Error description{}'.format(res))
```

### LLM client interface

All LLM classes follow the same interface:

```python
class ProviderAPI:
    def __init__(self):
        self.base_url = "..."
        self.model = "..."
        self.api_key = ...

    def ask(self, question: str, timeout: Optional[int] = 30) -> Dict[str, Any]:
        # Build request, call API, return content string
        # Wrap requests.exceptions.RequestException in Exception
```

---

## Forbidden Patterns

- **Don't create `requests.Session()` instances** — use `tools/request_b.session`
- **Don't skip `appsign()`** — all Bilibili API calls must go through `request_b.get()` or `request_b.post()` which handle signing
- **Don't catch exceptions without logging** — every `except` block must have a `logger.error()` or `logger.warning()` call
- **Don't store secrets in the repository** — user credentials and API keys go to `~/.bili-hardcore/`

---

## Testing Requirements

No test suite exists. When adding tests:

- Place tests in a top-level `tests/` directory (to be created)
- Use `pytest` as the test framework
- Mock HTTP calls — don't hit real Bilibili APIs in tests
- Test LLM answer parsing logic in `QuizSession.parse_answer()`

---

## Code Review Checklist

- [ ] New Bilibili API calls go through `request_b.get()`/`post()` (not raw `requests`)
- [ ] Error messages are in Chinese with raw response included via `.format(res)`
- [ ] `logger` is used instead of `print()` (except for interactive prompts in config)
- [ ] New LLM providers follow the `ask(question, timeout)` interface
- [ ] No secrets or tokens committed to the repo

---

## Known Tech Debt

- `config/config.py` executes interactive I/O at module import time (prints, input()) — importing this module triggers the LLM selection prompt
- Global mutable state: `config.access_token` and `config.csrf` are set by side effect
- `client/ziantt.py` has dead code (commented-out response handling)
- No `pyproject.toml` — project uses bare `requirements.txt`
- No type checking or linting configured

---

## Common Mistakes

- Importing `config/config.py` from a test or utility that shouldn't trigger interactive prompts — the module has side effects at import time
- Creating a new `requests.Session()` instead of using the shared one from `request_b.py` — loses retry configuration and shared headers
