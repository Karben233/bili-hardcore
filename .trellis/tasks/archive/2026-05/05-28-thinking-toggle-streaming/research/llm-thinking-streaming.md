# Research: LLM Thinking/Reasoning Content in Streaming APIs

- **Query**: How do different LLM providers handle "thinking"/"reasoning" content in their OpenAI-compatible chat completion streaming APIs?
- **Scope**: External (API documentation research)
- **Date**: 2026-05-28

## Findings

### 1. SiliconFlow (硅基流动)

**Base URL**: `https://api.siliconflow.cn/v1`
**Endpoint**: `POST /chat/completions`

#### Thinking Content Format

- Uses `reasoning_content` field at the same level as `content` in the response
- Streaming: `chunk.choices[0].delta.reasoning_content` contains thinking tokens
- Streaming: `chunk.choices[0].delta.content` contains final answer tokens
- Both fields can be `null` depending on the phase (thinking vs answering)

#### How to Enable/Disable Thinking

**Request parameter: `enable_thinking`** (boolean)
- Set `true` to enable thinking mode
- Set `false` to disable thinking mode
- Supported models include: `Pro/zai-org/GLM-5`, `Pro/zai-org/GLM-4.7`, `deepseek-ai/DeepSeek-V3.2`, `Qwen/Qwen3-*` series, etc.

**Request parameter: `thinking_budget`** (integer, 128-32768)
- Maximum number of tokens for chain-of-thought output
- Controls how long the model thinks before answering

**Request parameter: `reasoning_effort`** (string: "high" | "max")
- Only for `deepseek-ai/DeepSeek-V4-Flash`
- Default is "high" for regular requests

#### Streaming SSE Format (example)

```
data: {"id":"019bdaabd0514ee04b65607601e48651","object":"chat.completion.chunk","created":1768900251,"model":"Pro/zai-org/GLM-4.7","choices":[{"index":0,"delta":{"content":"","reasoning_content":null,"role":"assistant"},"finish_reason":null}],"system_fingerprint":"","usage":{"prompt_tokens":15,"completion_tokens":0,"total_tokens":15}}
```

When thinking content is present:
```
data: {"id":"...","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":null,"reasoning_content":"thinking text here"},"finish_reason":null}]}
```

When answer content is present:
```
data: {"id":"...","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"answer text here","reasoning_content":null},"finish_reason":null}]}
```

Termination: `data: [DONE]`

#### Non-streaming response

```json
{
  "choices": [{
    "message": {
      "role": "assistant",
      "content": "final answer...",
      "reasoning_content": "thinking process..."
    }
  }],
  "usage": {
    "completion_tokens_details": {
      "reasoning_tokens": 1190
    }
  }
}
```

#### Python streaming example (from SiliconFlow docs)

```python
for chunk in response:
    if not chunk.choices:
        continue
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="", flush=True)
    if chunk.choices[0].delta.reasoning_content:
        print(chunk.choices[0].delta.reasoning_content, end="", flush=True)
```

#### Manual SSE parsing (from SiliconFlow docs, using requests library)

```python
for chunk in response.iter_lines():
    if chunk:
        chunk_str = chunk.decode('utf-8').replace('data: ', '')
        if chunk_str != "[DONE]":
            chunk_data = json.loads(chunk_str)
            delta = chunk_data['choices'][0].get('delta', {})
            content = delta.get('content', '')
            reasoning_content = delta.get('reasoning_content', '')
```

#### Supported reasoning models on SiliconFlow

- `Pro/deepseek-ai/DeepSeek-R1` (always thinks, no enable_thinking needed)
- `deepseek-ai/DeepSeek-R1` (always thinks)
- `Qwen/QwQ-32B` (always thinks)
- Models with optional thinking: GLM-4.7, DeepSeek-V3.2, Qwen3 series, etc. (use `enable_thinking`)

**Note**: For DeepSeek-R1 models on SiliconFlow, `reasoning_content` is always returned. For optional-thinking models, set `enable_thinking: true` to get `reasoning_content`.

**Reference**: https://docs.siliconflow.cn/cn/userguide/capabilities/reasoning

---

### 2. Alibaba Cloud DashScope (阿里云百炼)

**Base URL**: `https://dashscope.aliyuncs.com/compatible-mode/v1`
**Endpoint**: `POST /chat/completions`

#### Thinking Content Format

Based on community documentation and the OpenAI-compatible endpoint pattern:

- For **QwQ** models: thinking content comes as `reasoning_content` in the response (same format as DeepSeek)
- For **Qwen3** series with `enable_thinking: true`: thinking content comes as `reasoning_content`
- The DashScope OpenAI-compatible endpoint follows the same `reasoning_content` / `content` split pattern as SiliconFlow and DeepSeek

#### How to Enable/Disable Thinking

**Request parameter: `enable_thinking`** (boolean)
- Only applicable to Qwen3 series models (Qwen3-8B, Qwen3-14B, Qwen3-32B, etc.)
- QwQ models always think (no parameter needed)
- Qwen3 models default to thinking mode; set `enable_thinking: false` to disable

**Chat template-level control** (for local models):
- `/think` and `/no_think` instructions in system/user messages
- `enable_thinking=True/False` in `tokenizer.apply_chat_template`

#### Streaming SSE Format

Same as OpenAI standard with `reasoning_content` extension:

```
data: {"id":"...","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"role":"assistant","reasoning_content":"thinking text"},"finish_reason":null}]}

data: {"id":"...","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"answer text"},"finish_reason":null}]}

data: [DONE]
```

#### Notes

- DashScope's documentation site requires JavaScript rendering, so direct HTML scraping fails
- The official documentation URLs (`help.aliyun.com/zh/model-studio/developer-reference/*`) return 404 for thinking-specific pages via curl
- The `base_url` in the project is configured for `dashscope.aliyuncs.com`, and the code already has a specific error message for it (line 64 of `openai.rs`)

**Reference**: Community examples and OpenAI-compatible endpoint documentation pattern

---

### 3. DeepSeek Official API

**Base URL**: `https://api.deepseek.com`
**Endpoint**: `POST /chat/completions`

#### Thinking Content Format

- Uses `reasoning_content` field, which is **the same level as `content`**
- This is the **original source** of the `reasoning_content` convention that SiliconFlow and DashScope follow
- Model: `deepseek-reasoner`

#### Streaming SSE Format (from official DeepSeek docs)

The streaming response uses the standard SSE format with `reasoning_content` in the delta:

```python
# From DeepSeek official docs
response = client.chat.completions.create(
    model="deepseek-reasoner",
    messages=messages,
    stream=True
)
reasoning_content = ""
content = ""
for chunk in response:
    if chunk.choices[0].delta.reasoning_content:
        reasoning_content += chunk.choices[0].delta.reasoning_content
    else:
        content += chunk.choices[0].delta.content
```

**Key behavior**: In streaming mode, `reasoning_content` chunks come first, then `content` chunks. The `reasoning_content` and `content` fields are mutually exclusive in each chunk -- when one is present, the other is typically `None`/empty.

#### Non-streaming response

```python
response = client.chat.completions.create(
    model="deepseek-reasoner",
    messages=messages
)
reasoning_content = response.choices[0].message.reasoning_content
content = response.choices[0].message.content
```

#### How to Enable/Disable Thinking

- The `deepseek-reasoner` model **always thinks** -- there is no `enable_thinking` parameter
- There is no way to disable thinking for this model
- For non-thinking mode, use other DeepSeek models (e.g., `deepseek-chat`)

#### API Parameters

- `max_tokens`: Maximum output length (including thinking), default 32K, max 64K
- **Unsupported parameters**: `temperature`, `top_p`, `presence_penalty`, `frequency_penalty`, `logprobs`, `top_logprobs`
- Supported: JSON Output, chat completions, prefix continuation (Beta)
- Not supported: Function Calling, FIM (Beta)

#### Multi-turn Context

- Thinking content (`reasoning_content`) must be **removed** before sending in the next round
- If `reasoning_content` is included in input messages, the API returns a 400 error
- Only `content` should be appended to the conversation history

**Reference**: https://api-docs.deepseek.com/zh-cn/guides/reasoning_model

---

### 4. OpenAI o-series

**Base URL**: `https://api.openai.com/v1`
**Endpoint**: `POST /chat/completions`

#### Thinking Content Format

**OpenAI o-series models do NOT expose reasoning/thinking content to the user.**

- The reasoning process is internal and not returned in the API response
- Only the final `content` is available
- Usage statistics include `completion_tokens_details.reasoning_tokens` which counts internal reasoning tokens

#### How to Control Reasoning

**Request parameter: `reasoning_effort`** (string)
- Values: `"low"`, `"medium"`, `"high"` (default)
- Controls how much computation the model spends on reasoning
- Higher effort = more reasoning tokens used (but not visible to user)
- Supported by: `o1`, `o1-mini`, `o3-mini`, `o4-mini`

#### Response format

```json
{
  "choices": [{
    "message": {
      "role": "assistant",
      "content": "final answer only"
    }
  }],
  "usage": {
    "prompt_tokens": 100,
    "completion_tokens": 500,
    "completion_tokens_details": {
      "reasoning_tokens": 300
    }
  }
}
```

#### Key Differences from Other Providers

- No `reasoning_content` field is ever returned
- Thinking content is completely hidden from the API consumer
- Only the reasoning token count is visible in usage stats
- This is a **fundamentally different approach** from DeepSeek/SiliconFlow/DashScope

---

### 5. Rust SSE Streaming with reqwest

#### Current Project State

The project uses `reqwest = { version = "0.12", features = ["rustls-tls", "json", "gzip", "deflate", "brotli"] }`.

**The `stream` feature is NOT currently enabled.**

#### reqwest `bytes_stream()` Method

- Available in reqwest 0.12.x when the `stream` feature is enabled
- Returns `impl futures_core::Stream<Item = Result<Bytes>>`
- Located at `reqwest::Response::bytes_stream()` (requires `#[cfg(feature = "stream")]`)
- Requires adding `"stream"` to reqwest features in Cargo.toml

#### SSE Parsing Options

**Option A: `eventsource-stream` crate (v0.2.3)**
- License: MIT OR Apache-2.0
- Dependencies: `futures-core ^0.3`, `nom ^7.1`, `pin-project-lite ^0.2.8`
- Last updated: 2022-02-17 (stable, no active development needed)
- Usage pattern:
  ```rust
  use eventsource_stream::Eventsource;
  
  let mut stream = client
      .post(&url)
      .json(&body)
      .send()
      .await?
      .bytes_stream()
      .eventsource();
  
  while let Some(event) = stream.next().await {
      match event {
          Ok(event) => {
              // event.event: event type (usually empty for SSE)
              // event.data: the data payload
              if event.data == "[DONE]" { break; }
              let chunk: serde_json::Value = serde_json::from_str(&event.data)?;
          }
          Err(e) => { /* handle error */ }
      }
  }
  ```
- **Pros**: Clean API, well-tested, handles SSE protocol correctly (reconnects, event types, etc.)
- **Cons**: Dev dependency on reqwest ^0.11 (but only for tests, not runtime)

**Option B: Manual SSE parsing**
- Use `bytes_stream()` directly and parse `data: ` prefix manually
- More control but more error-prone
- Need to handle: line splitting, `data: ` prefix stripping, `[DONE]` termination, empty lines

**Option C: `futures::StreamExt` + manual line parsing**
- Already have `futures = "0.3"` in the project
- Use `response.bytes_stream()` and process each chunk

#### Required Changes for SSE Streaming

1. Add `"stream"` feature to reqwest in `Cargo.toml`:
   ```toml
   reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "gzip", "deflate", "brotli", "stream"] }
   ```

2. Add `eventsource-stream` dependency:
   ```toml
   eventsource-stream = "0.2"
   ```

3. The `futures` crate (already in project) provides `StreamExt` for `.next()` on streams

---

### 6. Summary: Universal Streaming Response Format

All three Chinese providers (SiliconFlow, DashScope, DeepSeek) follow the **same pattern**:

```
SSE line: data: {"id":"...","object":"chat.completion.chunk","choices":[{"index":0,"delta":{...}}]}
```

Where `delta` contains:
- **During thinking phase**: `{"reasoning_content": "some thinking text", "content": null}`
- **During answer phase**: `{"content": "some answer text", "reasoning_content": null}`
- **Role chunk** (first): `{"role": "assistant", "content": "", "reasoning_content": null}`

Termination: `data: [DONE]`

This means **a single unified parser** can handle all three providers.

#### Key behavioral notes for implementation:

1. `reasoning_content` and `content` are mutually exclusive per chunk -- when one has content, the other is null/empty
2. Thinking always comes before the answer
3. For optional-thinking models, `reasoning_content` is simply absent when thinking is disabled
4. The `[DONE]` marker terminates the stream
5. Empty `data: ` lines should be ignored (SSE keep-alive)

---

### Related Specs

- `.trellis/spec/` directory exists but contains only `backend/` and `guides/` subdirectories
- No existing spec documents about LLM streaming were found

## Caveats / Not Found

1. **DashScope documentation** could not be fetched directly because their help site requires JavaScript rendering. The information about DashScope is based on community examples and the fact that they follow the same OpenAI-compatible convention with `reasoning_content`. The project already has DashScope-specific error handling (line 64 of `src/llm/openai.rs`).

2. **OpenAI o-series** does not expose thinking content -- this is architecturally different from the Chinese providers. If the project needs to show thinking content, OpenAI o-series cannot be used.

3. **eventsource-stream** crate was last updated in 2022 but is stable and widely used. Its dev dependency on reqwest ^0.11 is only for tests and does not affect runtime compatibility with reqwest 0.12.

4. The `reqwest` `stream` feature MUST be added to enable `bytes_stream()`. Without it, streaming is not possible.

5. **DeepSeek-R1 via SiliconFlow** always thinks and returns `reasoning_content` without needing `enable_thinking`. Other models on SiliconFlow (like DeepSeek-V3.2, Qwen3) require `enable_thinking: true` to activate thinking mode.
