# PRD: 压缩 release 包体积

## 目标
将 release 二进制从 4.9MB 进一步压缩

## 措施
1. `Cargo.toml` profile 加 `panic = "abort"`
2. `opt-level` 改为 `"z"`
3. `ratatui-image` 的 `image-defaults` feature 换成只启用 PNG + JPEG（验证码图片只需要这两种）

## 涉及文件
- `Cargo.toml`

## 验收
- cargo build --release 成功
- 对比压缩前后体积
