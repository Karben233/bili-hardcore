# Cross-Layer Thinking Guide

> **Purpose**: Think through data flow across layers before implementing.

---

## The Problem

**Most bugs happen at layer boundaries**, not within layers.

In this project, the main layers are:
- **API layer** (`api/client.rs`): Bilibili HTTP calls
- **Crypto layer** (`crypto.rs`): Request signing
- **LLM layer** (`llm/openai.rs`): AI answer generation
- **App layer** (`app.rs`): State machine and orchestration
- **UI layer** (`ui/*.rs`): Terminal rendering

---

## Before Implementing Cross-Layer Features

### Step 1: Map the Data Flow

For example, the quiz answer flow:

```
UI (user clicks) → App (state machine) → API (fetch question) → Crypto (sign request)
     → API (HTTP call) → App (receive data) → LLM (get AI answer) → API (submit answer)
     → App (update state) → UI (re-render)
```

For each arrow, ask:
- What format is the data in?
- What could go wrong?
- Who is responsible for validation?

### Step 2: Identify Boundaries

| Boundary | Common Issues |
|----------|---------------|
| API ↔ Crypto | Params must be sorted before signing |
| API ↔ App | Bilibili error codes → AppError conversion |
| App ↔ LLM | Prompt template formatting, answer parsing |
| App ↔ UI | State changes must trigger re-render |
| Config ↔ Storage | JSON serialization, file mtime for expiry |

### Step 3: Define Contracts

For each boundary:
- What is the exact input format?
- What is the exact output format?
- What errors can occur?

---

## Checklist for Cross-Layer Features

Before implementation:
- [ ] Mapped the complete data flow
- [ ] Identified all layer boundaries
- [ ] Defined format at each boundary
- [ ] Decided where validation happens

After implementation:
- [ ] Tested with edge cases (null, empty, invalid)
- [ ] Verified error handling at each boundary
- [ ] Checked data survives round-trip
