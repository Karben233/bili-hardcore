# bili-hardcore (opencode skill)

把 [bili-hardcore](https://github.com/Karben233/bili-hardcore) 的 B 站硬核会员自动答题能力做成 opencode skill。装上后对任意 agent 说"帮我答硬核会员"即可触发，agent **自身作答**，无需自备 LLM API Key。

## 与原 Rust 程序的区别

| 项 | 原 Rust 程序 | 本 skill |
|----|------------|---------|
| 答题 LLM | 调外部 OpenAI 兼容 API（需 base_url/model/key） | **agent 自身作答**，零配置 |
| 交互 | TUI 终端界面 | 对话式，agent 编排 |
| 依赖 | Rust 二进制 | Python 3 标准库 |
| 登录态 | `~/.bili-hardcore/auth.json` | **相同，双向兼容** |
| B 站 API | `src/crypto.rs` + `src/api/client.rs` | 移植到 `bili_quiz.py`（签名逐字节一致） |

## 安装

### 项目内（随仓库走）

已位于 `.agents/skills/bili-hardcore/`，无需额外操作。仓库内任何 opencode 会话自动可用。

### 全局（所有项目可用）

拷贝到以下任一位置：

```bash
# 方式一：opencode 全局 skills
cp -r .agents/skills/bili-hardcore ~/.config/opencode/skills/bili-hardcore

# 方式二：通用 agents skills
cp -r .agents/skills/bili-hardcore ~/.agents/skills/bili-hardcore
```

## 依赖

- **Python 3.7+**（仅标准库：hashlib/hmac/json/urllib）
- agent 有视觉能力时优先用 `see_image` 识别验证码；无视觉时回退让用户看图输入

## 使用

在任意 opencode 会话里对 agent 说：

- "帮我答 B 站硬核会员"
- "开始硬核会员答题"
- "查一下我的硬核会员答题得分"

agent 会自动加载本 skill 并按 `SKILL.md` 编排执行。首次使用需用 B 站 APP 扫码登录，登录态 7 天有效。

## 文件

- `SKILL.md` — 答题流程编排指令（agent 读取）
- `bili_quiz.py` — B 站 API CLI（appsign MD5 签名 + 全端点，移植自原 Rust 项目）
- `README.md` — 本文件

## 已验证

- `appsign` / `gen_ticket_params` 签名与原 Rust 实现**逐字节一致**（固定时间戳对照测试）
- `status` / `level` / `category` / `qrcode` 真实端点连通
- 登录态与原 Rust 程序**双向兼容**（同一 `~/.bili-hardcore/auth.json`）

## 限制

- 每日仅 3 次答题机会（B 站限制，skill 无法绕过）
- 答题账号需 6 级
- 验证码识别依赖 agent 视觉能力，可能需用户介入
