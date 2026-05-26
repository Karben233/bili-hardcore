# Config Page Reset Button

## Goal

在配置页添加一个重置按钮，点击后弹出确认弹窗，确认后清除所有配置项和登录状态。

## What I already know

* ConfigFocus 枚举有 BaseUrl/Model/ApiKey/SaveBtn 四个焦点项
* 配置存储在 `~/.bili-hardcore/openai_config.json`，登录状态在 `~/.bili-hardcore/auth.json`
* 已有确认弹窗模式：LoginTimeout 的 retry bool + 左右切换选项
* key_config 处理配置页键盘输入，Tab/Up/Down 循环焦点
* save_config() 保存配置并返回

## Requirements

* ConfigFocus 增加 ResetBtn 变体
* Tab/Up/Down 导航包含重置按钮（保存按钮之后）
* 选中重置按钮按 Enter 弹出确认弹窗
* 确认弹窗提示"将会重置所有配置项以及登录状态"，两个选项：确认重置 / 取消
* 确认后：删除配置文件和登录文件，清空 App 中的 config/auth/bili 状态，返回首页
* 取消：关闭弹窗，回到配置页

## Acceptance Criteria

- [ ] 配置页有"重置"按钮，可导航选中
- [ ] 按 Enter 弹出确认弹窗
- [ ] 确认后清除配置文件和登录状态，返回首页
- [ ] 取消后关闭弹窗，恢复配置页

## Definition of Done

* `cargo build` 通过
* 重置流程可正常操作

## Technical Approach

### 新增状态

App 增加 `config_confirm_reset: bool` 字段。当为 true 时，配置页渲染确认弹窗而非表单。

### ConfigFocus 扩展

```rust
enum ConfigFocus {
    BaseUrl, Model, ApiKey, SaveBtn, ResetBtn,
}
```

### 焦点循环

Tab: BaseUrl → Model → ApiKey → SaveBtn → ResetBtn → BaseUrl
Down: 同 Tab 方向
Up: 反方向

### 确认弹窗

复用 LoginTimeout 模式：
- config_confirm_reset = false: 默认选中"取消"
- 左右切换"确认重置"和"取消"
- Enter 执行选择
- ESC 关闭弹窗

### 重置操作

```rust
fn reset_all(&mut self) {
    let _ = std::fs::remove_file(config::openai_config_path());
    let _ = std::fs::remove_file(config::auth_path());
    self.config = None;
    self.auth = None;
    self.bili = BiliClient::new();
    self.cfg_fields = [String::new(), String::new(), String::new()];
    self.back(); // 返回首页
}
```

### UI 渲染

确认弹窗覆盖配置页：居中显示警告文字 + 两个按钮，类似 LoginTimeout 的样式。

## Out of Scope

* 不支持单独重置配置或登录状态
* 不支持撤销重置操作

## Technical Notes

* 涉及文件：`src/app.rs`、`src/input.rs`、`src/ui/config_page.rs`、`src/config.rs`
* 需要暴露配置文件路径的函数（如 openai_config_path/auth_path）或在 config 模块添加 delete 函数
