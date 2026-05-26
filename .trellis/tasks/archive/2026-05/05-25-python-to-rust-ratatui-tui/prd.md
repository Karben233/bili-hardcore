# Python → Rust 重构：Ratatui TUI

## Goal

将 bili-hardcore 项目从 Python 完全重构为 Rust，使用 Ratatui 构建 TUI 界面，提供三个页面（首页、配置页、开始答题页），保持所有 B站 API 调用参数和签名逻辑完全一致，最终可跨平台打包为单一可执行文件。

## What I already know

- 项目是一个 B站硬核会员自动答题工具，通过 LLM（OpenAI 兼容 API）自动回答100道题
- 认证流程：QR码扫码登录 → 保存 token 7天 → 自动加载缓存
- API 签名：appkey + 时间戳 + 参数排序 + MD5 生成 sign
- Ticket 生成：HMAC-SHA256 密钥 "XgwSnGZ1p"，消息格式 "ts{timestamp}"
- 当前打包：PyInstaller 打单文件 exe，GitHub Actions 5平台构建（Win/Mac/Linux x arm64）
- 配置管理：`~/.bili-hardcore/openai_config.json` + `auth.json`
- LLM 请求：POST `{base_url}/chat/completions`，含 `enable_thinking: false` 和 `thinking: {type: disabled}` 参数

## Requirements

### 功能需求（对齐 Python 版本）

- [x] QR 码扫码登录（终端显示 QR 码 + 轮询验证）
- [x] 登录信息缓存（7天有效，存储在 `~/.bili-hardcore/auth.json`）
- [x] 用户等级验证（必须6级）
- [x] 题目获取、验证码处理、分类选择
- [x] LLM 自动答题（100题循环）
- [x] 答案解析（从 LLM 回复提取1-4数字）
- [x] 答题结果展示（总分、分类得分）
- [x] OpenAI 兼容 API 配置（base_url, model, api_key）
- [x] 命令行参数传入配置：`program [url] [model] [apikey]`
- [x] 配置持久化到 `~/.bili-hardcore/openai_config.json`
- [x] 答题完成后可选清除配置（安全性）

### TUI 界面需求（新增）

#### 导航模型：二级页面

- 所有子页面通过 ESC 键返回上一级
- 首页为顶级页面，子页面由首页进入

#### 首页（一级）

- 居中显示项目名称和简介
- 两个操作按钮：`开始答题` / `配置`
- 上下方向键选择，回车确认

#### 配置页（二级，从首页进入）

- 三个输入框：`API URL`、`模型名称`、`API Key`
- 预填充已保存的配置（如有）
- `保存` 按钮：保存配置到 `~/.bili-hardcore/openai_config.json`，返回首页
- `取消` 按钮：放弃修改，返回首页
- ESC 键等同于取消，返回首页

#### 答题页（二级，从首页进入）

- **未配置时**：显示提示 "未配置 AI API，请先完成配置"，提供 `确认` 按钮，确认后跳转到配置页
- **已配置时**：自动进入答题流程，登录作为答题的前置步骤自动触发：
  1. 检查登录缓存（`~/.bili-hardcore/auth.json`，7天有效）
  2. 缓存有效 → 跳过登录
  3. 缓存无效/不存在 → 自动弹出登录页面（显示 QR 码 + 轮询状态 + 倒计时）
  4. 登录成功 → 验证用户等级（6级）→ 开始答题
  5. 登录超时 → 提示超时，提供 `重试` / `返回首页` 按钮
- 答题流程中显示：当前题号/100、题目内容、四个选项、AI 给出的答案、实时分数/正确率
- 等待 LLM 回复时：显示旋转动画 `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` + "AI 思考中..." 文字
- 验证码触发时：答题区域内嵌切换为验证码模式 — 上方显示分类列表（空格键多选，最多3个），中间显示验证码 URL（提示浏览器打开），下方输入框填写验证码，底部提交按钮。完成后自动切回答题模式。
- 答题结束后显示：总分、分类得分明细
- 答题完成后可选清除配置（安全性）
- ESC 键：答题中可退出，确认后返回首页

### API 调用要求（严格保持一致）

- 所有 API 参数必须与 Python 版本完全一致，不得删除任何参数
- 签名算法（MD5）必须产生与 Python 完全相同的结果
- Ticket 生成（HMAC-SHA256）必须产生与 Python 完全相同的结果
- 请求头必须与 Python 版本一致（User-Agent、Content-Type 等）
- 重试策略：3次重试，backoff_factor=1，状态码 500/502/503/504

### 打包要求

- 跨平台单一可执行文件（Windows x86_64, macOS arm64/x86_64, Linux x86_64/arm64）
- 使用静态链接（rustls-tls，不依赖系统 OpenSSL）
- GitHub Actions CI/CD 自动构建发布

## Assumptions (temporary)

- 使用 `reqwest` + `rustls-tls` 作为 HTTP 客户端
- 使用 `ratatui` 0.30+ 作为 TUI 框架
- 使用 `tokio` 作为异步运行时
- 使用 `cargo-dist` 进行跨平台发布
- 使用 `thiserror` + `anyhow` 进行错误处理
- 使用 `serde` + `serde_json` 进行配置序列化
- QR 码使用 `qrcode` crate 的字符串渲染功能
- 配置目录保持 `~/.bili-hardcore/` 不变，兼容旧版本

## Acceptance Criteria

- [ ] 所有 B站 API 调用参数与 Python 版本完全一致
- [ ] 签名算法在相同输入下产生与 Python 完全相同的输出
- [ ] TUI 三个页面（首页、配置页、答题页）功能完整
- [ ] QR 码可在终端正确显示和扫描
- [ ] 配置可持久化并从缓存加载
- [ ] 登录信息7天缓存有效
- [ ] 可通过命令行参数传入配置
- [ ] 5个平台的可执行文件可正常构建
- [ ] 答题流程端到端可用（登录→验证→答题→结果）

## Definition of Done

- 所有 `cargo test` 通过
- `cargo clippy` 无警告
- `cargo fmt` 格式化通过
- 5个平台的 GitHub Actions 构建成功
- 手动测试答题流程完整可用

## Out of Scope

- 不添加新的 API 提供商 SDK（保持直接 HTTP 调用）
- 不改变 B站 API 的调用参数
- 不增加新的答题策略（仅 LLM 答题）
- 不支持 GUI（仅 TUI）
- 不做数据库存储（保持 JSON 文件配置）

## Technical Approach

### 项目结构

```
bili-hardcore/
├── Cargo.toml
├── .github/workflows/release.yml
├── src/
│   ├── main.rs              # 入口：解析命令行参数，启动 TUI
│   ├── app.rs               # AppState：全局状态管理
│   ├── ui/                  # TUI 渲染层
│   │   ├── mod.rs
│   │   ├── home.rs          # 首页
│   │   ├── config.rs        # 配置页
│   │   └── quiz.rs          # 答题页
│   ├── api/                 # B站 API 客户端
│   │   ├── mod.rs
│   │   ├── sign.rs          # appsign 签名逻辑
│   │   ├── auth.rs          # QR 码登录、token 轮询
│   │   ├── senior.rs        # 题目获取、答案提交、验证码
│   │   ├── ticket.rs        # bili_ticket 生成
│   │   └── user.rs          # 用户信息
│   ├── llm/                 # LLM 集成
│   │   ├── mod.rs
│   │   └── openai.rs        # OpenAI 兼容 API
│   ├── config.rs            # 配置管理（加载/保存/校验）
│   ├── crypto.rs            # MD5 签名 + HMAC-SHA256
│   └── error.rs             # 错误类型定义
```

### 核心依赖

- `ratatui` 0.30+ — TUI 框架
- `crossterm` — 终端后端
- `tokio` — 异步运行时
- `reqwest` + `rustls-tls` — HTTP 客户端（静态链接）
- `serde` + `serde_json` — 序列化
- `clap` — 命令行参数
- `qrcode` — QR 码生成
- `md-5` + `hmac` + `sha2` — 加密
- `thiserror` + `anyhow` — 错误处理
- `tracing` + `tracing-appender` — 日志（仅写文件，不干扰 TUI）
- `dirs` — 跨平台路径

### 异步架构

```
tokio::select! {
    event = terminal.read() => handle_input(event)
    msg = rx.recv() => update_state(msg)   // 来自 async API 任务
}
```

- API 调用和 LLM 请求在 tokio 任务中执行
- 通过 channel（mpsc）将结果发送回主事件循环
- TUI 渲染在主循环中，60fps 渲染间隔

### 打包方案

使用 `cargo-dist` 配合 GitHub Actions：
- 触发条件：tag push（`v*`）
- 构建矩阵：5个目标平台
- 产物：单一可执行文件 + SHA256 校验
- 自动创建 GitHub Release

## Decision (ADR-lite)

**Context**: 需要选择 TUI 异步架构——纯同步阻塞 vs tokio 异步
**Decision**: 使用 tokio 异步架构
**Consequences**:
- 优势：API 调用不阻塞 UI 渲染，用户体验更好
- 代价：架构复杂度略高，需要 channel 通信
- 缓解：AppState + message passing 模式成熟可靠

## Research References

- [research/ratatui-tui-patterns.md](research/ratatui-tui-patterns.md) — Ratatui 架构、异步集成、QR码、跨平台构建
- [research/rust-project-structure.md](research/rust-project-structure.md) — 模块设计、错误处理、加密、CI/CD

## Open Questions

1. ~~TUI 页面之间的导航方式？~~ → 已决定：二级页面，ESC返回
2. ~~QR码登录流程在 TUI 中的交互设计？~~ → 已决定：点击"开始答题"自动触发，登录作为答题前置步骤
3. ~~验证码输入在 TUI 中的交互设计？~~ → 已决定：答题页内嵌显示
4. ~~答题过程中 LLM 请求等待时的 UI 表现？~~ → 已决定：旋转动画 + "AI 思考中..."
