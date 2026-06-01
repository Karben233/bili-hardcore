---
name: version-release
description: "版本发布流程：提交未暂存内容、更新版本号、生成更新日志、创建 git tag。使用时提供目标版本号，如 /release 1.2.0"
---

# 版本发布

自动化版本发布流程。根据提供的版本号，完成提交、更新版本、生成更新日志、打 tag 等操作。

---

## 使用方式

```
/release <版本号>
```

示例：`/release 1.0.0`、`/release 2.1.3-beta`

---

## Step 1: 解析版本号

从用户输入中提取版本号。如果没有提供版本号，询问用户提供目标版本号。

当前版本可通过以下命令获取：

```bash
grep '^version' Cargo.toml | head -1
```

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
