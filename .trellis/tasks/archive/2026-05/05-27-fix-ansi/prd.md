# PRD: 修复日志截断与 ANSI 转义码

## 修复清单

1. **日志 ANSI 转义码** — `setup_logging()` 写文件时带了颜色控制字符，需加 `.with_ansi(false)`
2. **日志截断** — `client.rs` 中所有 `tracing::info!` 使用 `truncate_str(&text, 500)` 截断响应，改为直接输出完整内容，删除不再需要的 `truncate_str` 函数

## 涉及文件
- `src/main.rs` — setup_logging 加 with_ansi(false)
- `src/api/client.rs` — 删除 truncate_str 调用及函数定义

## 验收标准
- cargo check + clippy 通过
