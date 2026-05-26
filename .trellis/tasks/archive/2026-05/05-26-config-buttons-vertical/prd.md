# Config Buttons Vertical Layout

## Goal
配置页的保存和重置按钮改为竖向排列。

## Requirements
- 保存按钮在上，重置按钮在下
- 确认弹窗的取消和确认重置按钮也改为竖向排列

## Technical Notes
- 仅修改 `src/ui/config_page.rs`
- 将 horizontal Layout 改为 vertical，每个按钮独占一行
