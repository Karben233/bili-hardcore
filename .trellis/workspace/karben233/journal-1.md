# Journal - karben233 (Part 1)

> AI development session journal
> Started: 2026-05-25

---



## Session 1: Bootstrap spec guidelines

**Date**: 2026-05-25
**Task**: Bootstrap spec guidelines
**Branch**: `refactor`

### Summary

Populated all 5 backend spec files (directory-structure, storage, error-handling, logging, quality) with real codebase patterns from bili-hardcore. Added trellis workspace config. Archived 00-bootstrap-guidelines task.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `78f954a` | (see git log) |
| `23fba70` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 2: Refactor: 简化 API provider 为仅 OpenAI 兼容格式

**Date**: 2026-05-25
**Task**: Refactor: 简化 API provider 为仅 OpenAI 兼容格式
**Branch**: `refactor`

### Summary

移除 DeepSeek/Gemini 独立 provider，只保留 OpenAI 兼容 API（自定义 base_url + model + api_key）。删除 deepseek.py、gemini.py、CONFIG_EXAMPLE.md，重构 config.py 移除 provider 选择菜单，start_senior.py 直接使用 OpenAIAPI，更新 spec 文件。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `3d11eb5` | (see git log) |
| `6b267ef` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 3: Python → Rust 重构：Ratatui TUI

**Date**: 2026-05-26
**Task**: Python → Rust 重构：Ratatui TUI
**Branch**: `refactor`

### Summary

将 bili-hardcore 项目从 Python 完全重构为 Rust，使用 Ratatui 构建二级页面 TUI（首页/配置/答题），答题页左右分栏显示当前题目与历史记录，实现 QR 码登录、验证码处理、LLM 自动答题完整流程，修复 API 签名/Ticket/URL/UTF-8 截断等关键 bug，跨平台 GitHub Actions 构建配置。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `0b840af` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 4: Captcha Sixel Image + Submit Button

**Date**: 2026-05-26
**Task**: Captcha Sixel Image + Submit Button
**Branch**: `refactor`

### Summary

验证码页面支持 Sixel/Kitty/iTerm2 图片渲染，三焦点导航（分类/输入框/提交按钮），R 键刷新，URL 降级显示

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `1232719` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 5: Config Reset Button

**Date**: 2026-05-26
**Task**: Config Reset Button
**Branch**: `refactor`

### Summary

配置页添加重置按钮，五焦点循环导航，确认弹窗防误操作，重置后清空配置和登录状态返回首页

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `2bfaaac` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete
