---
name: bili-hardcore
description: "B站硬核会员自动答题。Use when the user wants to complete bilibili 硬核会员试炼/六级答题、查询答题得分，或提到 bili-hardcore / 硬核会员 / B站答题 / 哔哩哔哩答题。zero-config，agent 自身作答，无需 LLM API Key。"
---

# bili-hardcore

B 站硬核会员试炼自动答题。agent 读题后**自身作答**（不再调用外部 LLM），仅依赖同目录的 `bili_quiz.py`（Python 标准库）与 B 站 API 交互。

> 仅 python3 标准库，无第三方依赖。登录态复用 `~/.bili-hardcore/auth.json`（7 天有效），与原 Rust 程序双向兼容。

---

## 脚本定位

`bili_quiz.py` 与本 SKILL.md 同目录。安装位置可能为项目内或全局，执行前先定位：

```bash
for d in ".agents/skills/bili-hardcore" "$HOME/.config/opencode/skills/bili-hardcore" "$HOME/.agents/skills/bili-hardcore"; do
  [ -f "$d/bili_quiz.py" ] && SKILL_DIR="$d" && break
done
[ -z "$SKILL_DIR" ] && { echo "未找到 bili_quiz.py"; exit 1; }
```

后续所有命令以 `python3 "$SKILL_DIR/bili_quiz.py" <子命令>` 调用。每条命令输出 JSON 到 stdout，诊断走 stderr。判断成败看 JSON 的 `ok` 字段。

子命令速查：

| 子命令 | 作用 |
|--------|------|
| `status` | 检查本地登录态（含 7 天过期判断） |
| `ticket` | 获取 web ticket（HMAC 签名，登录前用） |
| `qrcode` | 取 TV 登录二维码 `url` + `auth_code` |
| `poll <auth_code>` | 轮询登录，成功存 `~/.bili-hardcore/auth.json` |
| `level` | 查询账号等级（需 6 级才能答题） |
| `category` | 取答题分区分类（验证码流程用） |
| `captcha` | 取验证码 token + 下载图片到本地 |
| `captcha-submit <code> <token> <ids>` | 提交验证码，成功直接返回下一题 |
| `question` | 取一道题（题目 + 选项 text/hash） |
| `submit <id> <hash> <text>` | 提交答案，返回累计 score |
| `result` | 查询最终分类得分 |
| `logout` | 删除本地登录态 |

---

## 全局执行原则

**一旦用户明确要求"开始答题"，你必须自动连贯执行 Step 0 → Step 5 全流程，中途任何一步成功后都立即进入下一步，不得停下来等待用户确认或汇报。** 仅在以下情况暂停向用户提问：
- 需要用户扫码（Step 1 展示二维码）
- 需要用户选分区或识别验证码（Step 3b，且你无视觉能力时）
- 遇到报错需要用户决策（见异常处理）
- 流程结束（Step 5 展示结果）

等级检查、取题、作答、提交等步骤**全部自动连续执行**，无需逐步征求用户同意。每 10 题或用户主动询问时才汇报进度。

### 交互约定

凡需用户决策处，统一以**编号列表**形式呈现可选项，让用户选择序号。规则：

- 列出每个选项一行，格式为 `1) xxx  2) yyy  3) zzz`，末尾提示用户回复序号。
- 用户回复序号后，执行对应分支；回复无效则重新展示列表。
- **不要用开放式提问**让用户自由发挥，除非是验证码文字这类必须自由输入的场景。
- 各 agent 平台若有原生结构化选项能力（如 opencode 的 question 工具），可优先用以呈现该编号列表，提升体验；但 SKILL.md 本身不绑定任何平台专属工具，以保证跨平台可移植。

---

## Step 0：检查登录态

```bash
python3 "$SKILL_DIR/bili_quiz.py" status
```

- `logged_in: true` → **立即**跳 Step 2，不要询问用户
- `logged_in: false` → Step 1

---

## Step 1：扫码登录

### 1.1 取二维码

```bash
python3 "$SKILL_DIR/bili_quiz.py" qrcode
```

得到 `url` 和 `auth_code`。

### 1.2 展示二维码给用户

`url` 是二维码内容，需转成图片让用户用 **B 站 APP** 扫码。把下述在线二维码链接给用户在浏览器打开扫描（对齐原项目 Ctrl+B 行为）：

```
https://api.cl2wm.cn/api/qrcode/code?text=<URL编码后的 url>
```

也可直接把 `url` 给用户，让其在 B 站 APP 内打开。提醒用户：二维码约 60 秒有效。

### 1.3 轮询登录

每 ~2 秒执行一次，最多 30 次（约 60 秒）：

```bash
python3 "$SKILL_DIR/bili_quiz.py" poll <auth_code>
```

- `ok: true` → 登录成功，**立即**进入 Step 2，不要停下来汇报或等待
- `ok: false, pending: true` → 继续轮询
- 超时仍 pending → 向用户展示选项：
  ```
  二维码已过期，请选择：
  1) 重新扫码
  2) 取消
  请回复序号：
  ```
  用户选 1 → 回到 1.1 取新二维码；用户选 2 → 结束流程。

---

## Step 2：等级检查

```bash
python3 "$SKILL_DIR/bili_quiz.py" level
```

- `level: 6` → **立即**进入 Step 3 取题，不要询问用户
- 其它 → 告知用户等级不足（硬核会员试炼需 6 级账号，当前等级 N），并展示选项：
  ```
  1) 换个账号登录
  2) 取消
  请回复序号：
  ```
  用户选 1 → 回到 Step 1 重新扫码登录（先 `logout` 清旧态）；用户选 2 → 结束流程。

---

## Step 3：取题（含验证码分支）

```bash
python3 "$SKILL_DIR/bili_quiz.py" question
```

### 3a. 正常取题（`ok: true, need_captcha: false`）

得到 `id` / `question_num` / `question` / `answers[]`。进入 Step 4。

### 3b. 需要验证码（`ok: false, need_captcha: true`）

B 站每次答题会弹一次图块/数字验证码。分两步处理：

**① 选分区 + 取验证码图**

```bash
python3 "$SKILL_DIR/bili_quiz.py" category   # 取分区列表
python3 "$SKILL_DIR/bili_quiz.py" captcha    # 取 token + 下载图片到 image_path
```

`category` 返回 `categories: [{id, name}, ...]`。将分区以编号列表展示给用户选择：

```
请选择答题分区（1-3 个，至少选 1 个）：
1) 游戏
2) 动画
3) 科技
...
请回复序号（多个用逗号分隔，如 1,3）：
```

- 把 `categories` 每项作为一行，`label` = name，并附上其 id。
- **必须至少选 1 个，最多 3 个**。用户回复序号后，映射回对应 id，逗号拼接成 `ids`（如 `1,3`）。
- 若用户选超 3 个，提示上限并重新展示列表；若回复为空或无效，提示至少选 1 个并重新展示。

> 若 `category` 返回空列表（偶发于尚未进入答题流程），无需询问用户，直接用空 `ids` 继续，B 站会自动分配。

**② 识别验证码**

`captcha` 返回 `image_path`。你（agent）用 `see_image` 工具看图识别验证码文字 `bili_code`：

```
see_image(filePath=<image_path>, question="识别图中验证码文字，只返回文字本身")
```

**若你没有视觉能力 / see_image 失败**，回退让用户看图：把 `captcha` 返回的 `url` 或 `image_path` 展示给用户，并提示：
```
请查看上方验证码图片，回复图中显示的验证码文字：
```
用户回复的文字即 `bili_code`（这是少数允许自由输入的场景）。

**③ 提交验证码**

```bash
python3 "$SKILL_DIR/bili_quiz.py" captcha-submit <bili_code> <token> "<ids>"
```

成功后该命令**直接返回下一题**（等同 Step 3a 的输出）。失败则 `ok: false`，看 `raw` 里的 B 站错误；常见是验证码错，重新 `captcha` 取新图再试。

---

## Step 4：循环答题

每题流程：取题 → agent 作答 → 提交。

### 4.1 作答协议

收到题目 JSON 后，你在内心按以下模板推理（确定答案即可，不必向用户复述过程）：

> 你是资深 B 站用户，正在完成硬核会员试炼，涉及分区：[<本批所选分区，或"未知">]。
> 根据问题和选项判断正确答案，返回对应选项的序号（1, 2, 3, 4）。
> 示例：问题"大的反义词是什么？" 选项 ['长','宽','小','热'] → 回答 3。
> 不确定时选最接近的，不要解释，只给序号 1-4。

题目含百科/常识/分区相关知识，发挥你的知识库作答。确定序号 N（1-based）后，取 `answers[N-1].hash` 与 `answers[N-1].text`。

### 4.2 提交

```bash
python3 "$SKILL_DIR/bili_quiz.py" submit <id> <hash> "<text>"
```

返回 `ok: true, score: <累计答对数>`。

### 4.3 循环

- submit 成功 → **立即**回 Step 3 取下一题，重复，不要停顿汇报（除非到 10 题节点）
- `question` 返回 `ok: false` 且 `need_captcha: false`：通常表示 100 题已答完，**立即**进入 Step 5
- `question_num` 达到 100：答完该题后**立即**进入 Step 5

每 10 题或用户主动询问时汇报一次进度（如"第 N/100 题，当前答对 M"），其余时间保持静默连续执行。

---

## Step 5：查询最终得分

```bash
python3 "$SKILL_DIR/bili_quiz.py" result
```

返回 `score`（总得分）和 `scores[]`（各分区得分/总数）。向用户展示完整结果。

---

## 异常处理

- **每日 3 次限制**：`category` 返回 `code: 41099` 表示今日答题次数已用完。告知用户次日再试（可在 B 站 APP 答题页查看解锁时间），并展示选项：
  ```
  1) 结束流程
  2) 查询当前得分
  请回复序号：
  ```
  用户选 2 → 执行 Step 5；否则结束。
- **answer 作答失败**：若你对某题无把握，仍必须选一个序号提交（B 站答题不能跳题），选最接近的。
- **网络错误**：命令返回 `ok: false, error: "..."`，可自动重试 1-2 次；持续失败则向用户展示选项：
  ```
  网络异常，请选择：
  1) 重试
  2) 取消
  请回复序号：
  ```
- **登录过期**：`status` 返回 `logged_in: false` 或 API 报登录失效，回到 Step 1 重新扫码。
- **已是硬核会员**：`submit` 返回 `error: "请检查是否已经是硬核会员"`，直接结束。

---

## 注意事项

- 每日仅 3 次答题机会，每次 100 题，谨慎触发。
- 答题需账号 **6 级**。
- 登录态文件 `~/.bili-hardcore/auth.json` 含敏感凭证，不要展示给用户或写入代码。
- 验证码图片仅当次有效，识别失败就重新 `captcha` 取新图。
