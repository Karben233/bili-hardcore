# Captcha Sixel Image Display

## Goal

在终端 TUI 界面中直接显示验证码图片（Sixel 协议），无需切换到浏览器查看。不支持 Sixel 的终端自动降级为现有 URL 提示方式。支持刷新验证码。

## What I already know

* 当前验证码流程：检测 → 获取分类+URL → 显示 URL 链接 → 用户输入 → 提交
* `CaptchaState` 持有 `captcha_url`（图片链接）、`captcha_token`、`categories`、`input`
* 项目使用 `ratatui = "0.29"`，必须使用 `ratatui-image = "9"`（v11 需要 ratatui 0.30）
* `ratatui-image` 的 `Picker::from_query_stdio()` 自动检测终端协议并降级
* 当前运行环境为 WSL2 + Windows Terminal（不支持 Sixel），降级路径必须可靠
* `reqwest` 已在项目中，可用于下载图片

## Requirements

* 下载验证码图片并解码为 `DynamicImage`
* 使用 `ratatui-image` 的 Sixel/Kitty/iTerm2 协议在终端内显示验证码
* 自动检测终端是否支持图形协议，不支持时降级显示 URL 链接
* 按 R 键刷新验证码（重新获取图片）
* `Picker` 在 `ratatui::init()` 之后初始化，传递给 App

## Acceptance Criteria

- [ ] Sixel 支持的终端中，验证码图片内联显示在 TUI 界面
- [ ] 不支持 Sixel 的终端中，显示 "请打开链接查看验证码: URL"（现有行为）
- [ ] 按 R 键刷新验证码，重新获取并显示新图片
- [ ] 图片下载失败时，降级到 URL 显示方式
- [ ] 分类选择、输入、提交等现有功能不受影响

## Definition of Done

* `cargo build` 通过
* 在支持的终端和不支持的终端中分别测试
* 无 unsafe 代码
* 现有验证码流程不受影响

## Technical Approach

### 依赖

```toml
ratatui-image = { version = "9", default-features = false, features = ["image-defaults", "crossterm"] }
image = "0.25"
```

禁用 `chafa-dyn` 避免 libchafa 运行时依赖，保留 Sixel/Kitty/iTerm2/Halfblocks 协议支持。

### 核心变更

1. **Picker 初始化**：`main.rs` 中 `ratatui::init()` 后调用 `Picker::from_query_stdio()`
2. **CaptchaState 扩展**：增加 `captcha_image: Option<ImageProtocol>` 和 `supports_image: bool`
3. **图片下载**：`spawn_fetch_captcha` 中增加图片下载逻辑，通过新事件 `CaptchaImage` 回传
4. **UI 渲染**：`draw_captcha` 中根据 `supports_image` 条件渲染图片或 URL
5. **刷新支持**：按 R 键触发重新获取验证码

### Widget 选择

使用 `Image`（stateless）而非 `StatefulImage`，因为验证码图片小且不频繁更新，避免阻塞渲染。

## Out of Scope

* Halfblocks 降级渲染（仅 Sixel → URL fallback，不做中间态）
* 支持其他图形协议的 UI 优化
* 验证码 OCR 自动识别

## Technical Notes

* `ratatui-image = "9"` 是唯一兼容 ratatui 0.29 的版本
* `Picker::from_query_stdio()` 需要在 alternate screen 启用后调用
* Sixel 是 "immediate-mode" 协议，ratatui-image 内部处理跳过绘制覆盖区
* `Image` widget 使用 `picker.new_protocol()` 预处理图片，不阻塞渲染

## Research References

* [`research/sixel-rust-terminal.md`](research/sixel-rust-terminal.md) — ratatui-image 版本兼容性、API、终端支持矩阵
