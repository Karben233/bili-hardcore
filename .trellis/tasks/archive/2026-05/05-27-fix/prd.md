# PRD: 修复高/中优先级代码问题

## 背景
代码审查发现4个高/中优先级问题需要修复。

## 修复清单

### 高优先级
1. **`_guard` 提前 drop 导致日志失效** (main.rs:37)
   - `setup_logging()` 中 `tracing_appender::non_blocking` 返回的 guard 在函数返回时 drop，导致非阻塞 writer 失效
   - 修复：将 guard 返回给 main()，在整个程序生命周期持有

2. **`String::insert` 用字符偏移当字节偏移** (ui/config_page.rs:85)
   - `text.insert(pos, '|')` 中 `pos` 是字符偏移，但 `String::insert` 需要字节偏移
   - 非ASCII 输入时 panic
   - 修复：用字符级操作替代，遍历 chars 找到正确的插入位置

### 中优先级
3. **`async_clone` 丢失连接池** (app.rs:866-873)
   - 每次 spawn 都 `BiliClient::new()` 创建新 `reqwest::Client`，丢失连接复用
   - 修复：让 `BiliClient.http` 字段 `Clone`，直接 clone 共享连接池

4. **`ShowingQuestion` 死代码** (app.rs:737)
   - `QuestionReady` 处理中先设 `ShowingQuestion` 后立即覆盖为 `WaitingLlm`
   - 修复：删除无效的 `ShowingQuestion` 赋值

## 验收标准
- `cargo check` 无错误
- `cargo clippy` 无新增 warning
- 所有修改点与上述清单一一对应
