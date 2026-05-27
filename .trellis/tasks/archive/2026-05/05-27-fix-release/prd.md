# PRD: 修复日志时区与 release 禁用日志

## 修复清单

1. **日志时间使用本地时区** — 当前 tracing-subscriber 默认 UTC，差8小时
   - 加 `time` crate 并启用 `local-offset` feature
   - 使用 `LocalTime::new()` 作为 timer
2. **Release 构建不输出日志** — 用 `cfg(debug_assertions)` 条件编译，release 跳过 setup_logging

## 涉及文件
- `Cargo.toml` — 加 time 依赖
- `src/main.rs` — setup_logging 改用本地时间 + 条件编译

## 验收标准
- cargo check + clippy 通过
- debug 模式日志时间显示本地时间
- release 模式不生成日志文件
