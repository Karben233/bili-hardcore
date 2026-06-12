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


## Session 6: Config Buttons Vertical

**Date**: 2026-05-26
**Task**: Config Buttons Vertical
**Branch**: `refactor`

### Summary

配置页和确认弹窗按钮从水平排列改为竖向排列

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `1040088` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 7: 清除 Python 遗留代码，项目完全迁移到 Rust

**Date**: 2026-05-26
**Task**: 清除 Python 遗留代码，项目完全迁移到 Rust
**Branch**: `refactor`

### Summary

删除 Python 源码目录、requirements.txt、PyInstaller spec，清除 Rust 代码中引用 Python 的注释，更新 .gitignore 和 README.md，重写 .trellis/spec 全部文档以反映 Rust 架构

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `108a9b4` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 8: fix: ESC 退出答题后后台任务不再继续运行

**Date**: 2026-05-27
**Task**: fix: ESC 退出答题后后台任务不再继续运行
**Branch**: `refactor`

### Summary

在 App::process() 添加页面守卫，self.page != Page::Quiz 时丢弃所有答题事件，防止 ESC 退出后 tokio 后台任务继续驱动答题循环。同时增加答题历史持久化功能。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `95beda3` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 9: 移除 Tab 键切换操作

**Date**: 2026-05-27
**Task**: 移除 Tab 键切换操作
**Branch**: `refactor`

### Summary

移除配置页和验证码页的 Tab 键焦点切换逻辑，统一使用 ↑↓ 箭头导航，同步更新帮助文字

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `72f6360` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 10: Spec 更新 + 键盘操作一致性修复

**Date**: 2026-05-27
**Task**: Spec 更新 + 键盘操作一致性修复
**Branch**: `refactor`

### Summary

更新 4 个 code-spec（键盘处理、后台任务、UI 帮助栏、LLM 重试），移除 vim 键和左右箭头导航，修复 Captcha 焦点循环，补充 UI 提示文字

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `607d709` | (see git log) |
| `85ee723` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 11: 修复高/中优先级代码问题

**Date**: 2026-05-27
**Task**: 修复高/中优先级代码问题
**Branch**: `refactor`

### Summary

修复4个代码问题：日志WorkerGuard提前释放、config_page UTF-8光标偏移panic、BiliClient连接池丢失、ShowingQuestion死代码；额外清理clippy needless_return警告。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `14aac1e` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 12: 修复日志ANSI转义码与截断

**Date**: 2026-05-27
**Task**: 修复日志ANSI转义码与截断
**Branch**: `refactor`

### Summary

日志写文件禁用ANSI颜色，移除API响应截断，删除truncate_str函数

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `87dbc13` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 13: 日志时区与release禁用

**Date**: 2026-05-27
**Task**: 日志时区与release禁用
**Branch**: `refactor`

### Summary

日志使用本地时区(LocalTime)，release构建通过cfg(debug_assertions)跳过日志初始化

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `a654682` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 14: 压缩release包体积

**Date**: 2026-05-27
**Task**: 压缩release包体积
**Branch**: `refactor`

### Summary

精简image依赖只保留png+jpeg，release profile加panic=abort和opt-level=z，体积从4.9MB降到3.6MB

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `c37f50c` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 15: 流式输出 + 思考模式 toggle

**Date**: 2026-05-28
**Task**: 流式输出 + 思考模式 toggle
**Branch**: `refactor`

### Summary

将 LLM 请求改为 SSE streaming，新增思考模式 toggle 配置，答题 UI 实时展示推理过程和答案。兼容硅基流动/百炼/DeepSeek 三家服务商。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `a4472a2` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 16: 优化答题界面UI

**Date**: 2026-05-28
**Task**: 优化答题界面UI
**Branch**: `refactor`

### Summary

左侧选项移至思考内容上方、添加题号显示、新增ShowingResult阶段展示答题结果1秒后自动进入下一题

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `cd7987e` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 17: 增加LLM配置预设模板功能

**Date**: 2026-06-12
**Task**: 增加LLM配置预设模板功能
**Branch**: `main`

### Summary

新增预设模板选择功能：presets.json 编译期嵌入、首次进入自动弹出模板浮层、配置页模板按钮、统一按钮样式、思考模式默认勾选、按键提示固定底部、spec 更新

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `9749e30` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete
