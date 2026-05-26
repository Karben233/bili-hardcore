# Code Reuse Thinking Guide

> **Purpose**: Stop and think before creating new code — does it already exist?

---

## The Problem

**Duplicated code is the #1 source of inconsistency bugs.**

When you copy-paste or rewrite existing logic:
- Bug fixes don't propagate
- Behavior diverges over time
- Codebase becomes harder to understand

---

## Before Writing New Code

### Step 1: Search First

```bash
# Search for similar function names
grep -rn "function_name" src/

# Search for similar logic
grep -rn "keyword" src/
```

### Step 2: Ask These Questions

| Question | If Yes... |
|----------|-----------|
| Does a similar function exist? | Use or extend it |
| Is this pattern used elsewhere? | Follow the existing pattern |
| Could this be a shared utility? | Create it in the right module |
| Am I copying code from another file? | **STOP** — extract to shared |

---

## Common Duplication Patterns in This Project

### Pattern 1: Bilibili API calls

All API calls follow the same pattern: sign params → HTTP request → check code → extract data.

If adding a new endpoint, follow the existing pattern in `api/client.rs`.

### Pattern 2: Error conversion

All Bilibili API responses convert to `AppError::Api`. Don't create new error variants when `AppError::Api` suffices.

---

## When to Abstract

**Abstract when**:
- Same code appears 3+ times
- Logic is complex enough to have bugs
- Multiple modules need this

**Don't abstract when**:
- Only used once
- Trivial one-liner
- Abstraction would be more complex than duplication

---

## Checklist Before Commit

- [ ] Searched for existing similar code
- [ ] No copy-pasted logic that should be shared
- [ ] Constants defined in one place
- [ ] Similar patterns follow same structure
