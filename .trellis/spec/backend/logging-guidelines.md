# Logging Guidelines

> How logging is done in this project.

---

## Overview

Logging uses Python's built-in `logging` module. A single global `logger` instance is created in `tools/logger.py` and imported everywhere.

---

## Logger Setup

```python
# tools/logger.py — imported as `from tools.logger import logger`
logger = setup_logger()  # name='bili-hardcore', level=INFO
```

- **Library**: `logging` (stdlib)
- **Logger name**: `'bili-hardcore'`
- **Default level**: `logging.INFO` (DEBUG commented out)
- **Log directory**: `bili-hardcore/logs/` (auto-created)

---

## Log Levels

| Level | When to use | Example |
|-------|-------------|---------|
| `DEBUG` | Request/response details | `logger.debug(f'发送GET请求: {url}, 参数: {signed_params}')` |
| `INFO` | Normal flow progress | `logger.info("答案提交成功")`, `logger.info(f"第{num}题:{question}")` |
| `WARNING` | Unexpected but recoverable | `logger.warning(f"AI回复了无关内容:[{answer}]")` |
| `ERROR` | Operation failures | `logger.error(f"获取题目失败: {str(e)}")` |

---

## Log Format

```
%(asctime)s - %(name)s - %(levelname)s - %(message)s
```

Example output:
```
2026-05-25 10:30:15,123 - bili-hardcore - INFO - 第1题:以下哪个是...
2026-05-25 10:30:16,456 - bili-hardcore - ERROR - 获取题目失败: ...
```

---

## Output Handlers

- **File handler**: Timestamped files at `logs/YYYY-MM-DD_HH-MM-SS.log`, UTF-8 encoding
- **Console handler**: `StreamHandler()` to stdout/stderr

Both handlers use the same format and level.

---

## What to Log

- Quiz progress: question number, question text, options displayed
- Answer results: AI answer, correct/wrong, running score and accuracy
- API interactions: request URLs and params (DEBUG level only)
- Login status: cache loaded, QR code prompt, login success/failure
- Config actions: key saved, config cleared

---

## What NOT to Log

- **API keys** — never log `api_key` values
- **Full auth tokens** — log only "登录成功", not the token value
- **User cookies** — referenced but not logged

---

## Usage Pattern

```python
from tools.logger import logger

# Module-level import, then use throughout
logger.info(f"当前得分:{score}, 正确率:{accuracy:.1f}%")
logger.error(f"提交失败: {result}")
logger.debug(f'请求成功: {data}')
```

---

## Forbidden Patterns

- **Don't use `print()` for errors** — use `logger.error()` (note: `config.py` uses `print()` for interactive prompts, which is intentional)
- **Don't create additional loggers** — import the global `logger`
- **Don't log at DEBUG level for normal flow** — DEBUG is for request/response details only

---

## Common Mistakes

- Using `print()` instead of `logger` for status messages — `config.py` does this for interactive I/O, but operational messages should use `logger`
- Forgetting to format floats with precision specifier — use `f"{value:.1f}%"` for percentages
