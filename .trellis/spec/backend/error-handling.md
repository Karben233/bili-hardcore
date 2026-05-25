# Error Handling

> How errors are handled in this project.

---

## Overview

Errors propagate as Python `Exception` instances. Functions raise on failure; callers catch when they can recover. No custom exception classes are used.

---

## Error Types

The project uses plain `Exception` throughout — no custom exception hierarchy.

```python
# Raise pattern (all client/ functions)
if res and res.get('code') == 0:
    return res.get('data')
else:
    raise Exception('获取分类失败{}'.format(res))
```

```python
# LLM wrapper pattern (tools/LLM/openai.py)
except requests.exceptions.RequestException as e:
    raise Exception(f"OpenAI API request failed: {str(e)}")
```

---

## Error Handling Patterns

### Pattern 1: Raise on failure (client/ layer)

All Bilibili API wrapper functions in `client/` raise `Exception` with a Chinese error message including the raw response:

```python
# client/senior.py
if res and res.get('code') == 0:
    return res.get('data')
elif res and res.get('code') == 41099:
    raise Exception('获取分类失败，可能是已经达到答题限制(B站每日限制3次)...')
else:
    raise Exception('获取分类失败，请前往B站APP确认...{}'.format(res))
```

### Pattern 2: Catch + return False (scripts/ layer)

High-level workflow functions in `scripts/` catch exceptions and return `False` instead of propagating:

```python
# scripts/start_senior.py
except Exception as e:
    logger.error(f"获取题目失败: {str(e)}")
    return False
```

### Pattern 3: Exponential backoff retry

```python
# scripts/start_senior.py — QuizSession.start()
sleep_time = math.pow(2, retry_count + 1)
logger.info(f"正在重试({sleep_time:.0f}s)...")
sleep(sleep_time)
retry_count += 1
if retry_count > 7:
    logger.error("重试次数过多，程序终止")
    return
```

### Pattern 4: Top-level catch-all

```python
# main.py
try:
    auth()
    validate()
    start()
except Exception as e:
    logger.error(f"程序运行出错: {str(e)}")
    input("按回车键退出程序...")
```

---

## Bilibili API Error Codes

| Code | Meaning | Handling |
|------|---------|----------|
| `0` | Success | Return `data` field |
| `41099` | Daily quiz limit reached (3 attempts/day) | Raise with message |
| `41103` | Submission error (possibly already hardcore member) | Log error, stop |
| Other | Unknown error | Raise with raw response |

---

## HTTP Retry Strategy

Configured in `tools/request_b.py`:

```python
retry_strategy = Retry(
    total=3,
    backoff_factor=1,
    status_forcelist=[500, 502, 503, 504]
)
```

Only retries on server errors (5xx). Client errors (4xx) propagate immediately.

---

## Forbidden Patterns

- **Don't silently swallow exceptions** — always log the error
- **Don't use bare `except:`** — catch `Exception` at minimum
- **Don't create custom exception classes** — the project doesn't use them

---

## Common Mistakes

- Forgetting to check `res.get('code') == 0` before accessing `res.get('data')` — non-zero responses may lack the `data` field
- Not distinguishing between network errors and API business errors — `request_b.py` raises on HTTP errors, but API-level errors (code != 0) need separate checking in `client/` functions
