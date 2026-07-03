---
name: version-release
description: "版本发布流程：提交未暂存内容、自动判定版本号（需用户确认）、生成更新日志、创建 git tag。直接 /release 即可，版本号由 AI 基于提交记录推断，不满意可自行指定"
---

# 版本发布

自动化版本发布流程。根据提交记录自动推断版本号，**经用户确认后**，完成提交、更新版本、生成更新日志、打 tag 等操作。

---

## 使用方式

```
/release
```

直接运行即可。AI 会分析自上一个 tag 以来的提交记录，自动推断版本号，**在执行任何操作前会先展示建议版本号并等待你确认**。

如果对建议的版本号不满意，也可以直接指定：

```
/release 1.0.0
```

---

## Step 1: 确定版本号（需用户确认）

> ⚠️ 本步骤必须拿到用户明确同意后才能进入 Step 2。在用户确认版本号之前，**不要**执行任何 git 提交、不要修改任何文件。

### 1.1 获取当前版本

```bash
grep '^version' Cargo.toml | head -1
```

### 1.2 获取自上一个 tag 以来的提交

```bash
git describe --tags --abbrev=0 2>/dev/null
```

- 存在上一个 tag：`git log <上一个tag>..HEAD --oneline`
- 没有上一个 tag：`git log --oneline`（首次发布）

### 1.3 推断版本号

按 Conventional Commits 前缀判定 bump 类型，取所有提交中的**最高级别**：

| 提交前缀 | bump 类型 |
|---------|----------|
| `feat!`、`BREAKING CHANGE`、标记为破坏性变更 | **major**（X+1.0.0） |
| `feat:` | **minor**（x.Y+1.0） |
| `fix:`、`refactor:`、`perf:`、`chore:`、`docs:` 等 | **patch**（x.y.Z+1） |

在当前版本基础上按 bump 类型递增，得到**建议版本号**。

- 如果用户在命令中显式指定了版本号（如 `/release 1.2.0`），以用户指定的版本号为候选，但仍需走下面的确认流程。
- 若推断结果与常理明显不符（如仍在 0.x 阶段、或提交均为内部重构），可在确认时附带说明，交由用户定夺。

### 1.4 向用户确认版本号（使用 AskUserQuestion 工具）

调用 `AskUserQuestion` 工具，以可点选的选项形式让用户确认。上下文写在 `question` 里，候选版本作为 `options`：

- **header**：`版本号`
- **question**：简述上下文，例如
  `当前版本 1.2.3，自上个 tag 以来 feat×2 / fix×3，建议 minor bump。使用哪个版本号发布？`
- **options**（按推荐度排序）：
  1. **推荐选项放第一个**，label 为建议版本号（如 `1.3.0`），description 注明 bump 类型与原因（如 `minor bump：含 2 个新功能`）。label 末尾追加 `（推荐）`。
  2. 视情况追加 1 个备选版本（如更保守的 patch `1.2.4`，或更激进的 major `2.0.0`），仅在确有合理空间时才加。
- **不要单独列"自定义"选项**：界面会自动提供 "Other" 入口，用户不满意时直接在那里输入想要的版本号即可。

根据用户选择：

- 选中某个版本号选项 → 使用该版本号，进入 Step 2
- 通过 "Other" 输入了自定义版本号 → 使用用户输入的版本号（做基本合法性校验，异常则提示后再次确认），进入 Step 2
- 用户要求中止 → 终止流程

**未拿到用户明确选择前，不得继续后续步骤。**

---

## Step 2: 检查并提交未提交的内容

检查 git 工作区是否有未提交的变更：

```bash
git status --porcelain
```

- 如果有未提交的内容，**先暂存并提交**：
  - 查看 `git diff` 了解变更内容，生成合适的 commit message
  - 执行 `git add -A && git commit -m "<commit message>"`
  - commit message 使用中文，遵循 conventional commits 风格（如 `feat: xxx`、`fix: xxx`）
- 如果工作区干净，跳过此步骤

---

## Step 3: 更新 Cargo.toml 版本号

将 `Cargo.toml` 中的 `version` 字段更新为目标版本号。

编辑 `Cargo.toml` 第 3 行的 `version` 值：

```
version = "<目标版本号>"
```

更新完成后，同步更新 `Cargo.lock`：

```bash
cargo generate-lockfile
```

---

## Step 4: 生成版本更新日志

基于 git log 生成更新日志。获取自上一个 tag 以来的所有提交：

```bash
git describe --tags --abbrev=0 2>/dev/null
```

- 如果存在上一个 tag，用 `git log <上一个tag>..HEAD --oneline` 获取变更
- 如果没有上一个 tag，用 `git log --oneline` 获取所有变更

根据 commit message 自动分类到以下章节（都是可选的，没有内容的章节省略）：

| 章节 | 匹配的 commit 前缀 |
|------|-------------------|
| 🔥 重点内容 | `feat!`、`feat:` 且为重大功能 |
| ✨ 新功能 | `feat:` |
| 🔄 变更 | `refactor:`、`chore:`、`ci:`、`perf:` |
| 🐛 修复 | `fix:` |
| 📝 文档 | `docs:` |

更新日志是给用户看的，所以无关于发布流程的技术细节（如版本号更新、tag 创建等）不需要包含在日志中。只关注功能变更和修复内容。不一定要把所有的提交信息都包含在日志里，适当合并或省略一些细节，保持日志简洁易读，要让用户能看懂。

### 更新日志格式

文件路径：`docs/release-notes/v<版本号>.md`

```markdown
### 🔥 重点内容

- <重点功能描述>

### ✨ 新功能

- <功能描述>

#### 🔄 变更

- <变更描述>

### 🐛 修复

- <修复描述>
```

**注意**：
- 确保先创建 `docs/release-notes/` 目录（如不存在）
- 没有内容的章节直接省略，不要保留空标题
- commit hash 使用 7 位短格式
- 描述部分使用中文，基于 commit message 润色

---

## Step 5: 提交版本发布内容

将 Cargo.toml、Cargo.lock 和更新日志一起提交：

```bash
git add Cargo.toml Cargo.lock docs/release-notes/
git commit -m "release: <版本号>版本发布"
```

---

## Step 6: 创建 Git Tag

以版本号创建 annotated tag：

```bash
git tag -a "<版本号>" -m "<聚焦于重点内容的简短描述>"
```

---

## Step 7: 完成提示

向用户报告发布完成的状态，包含：

```
✅ 版本 <版本号> 发布完成！

下一步操作：
git push origin main --tags  # 推送代码和 tag 到远程
```

**到此停止，等待用户决定下一步操作。** 不要自动推送或执行任何进一步操作。
