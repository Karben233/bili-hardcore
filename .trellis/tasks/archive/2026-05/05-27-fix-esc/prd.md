# Fix: ESC 退出答题后后台任务继续运行

## Problem
在答题页面按 ESC 返回主界面后，已 spawn 的 tokio 后台任务（获取题目、LLM 请求、提交答案等）仍在运行。这些任务通过 mpsc channel 发送 AppEvent，main loop 无条件处理所有事件，process() 也不检查当前页面，导致答题流程在后台自动继续。

## Root Cause
1. `back()` 只改变 `self.page`，不取消 spawn 的任务
2. main.rs 事件循环 `while let Ok(ev) = app.rx.try_recv()` 无条件处理所有事件
3. `process()` 中 `SubmitOk → spawn_fetch_question`、`LlmOk → spawn_submit` 等链式调用不检查当前页面

## Solution
在 `App::process()` 中，对答题相关事件增加 `self.page == Page::Quiz` 守卫。当不在答题页面时丢弃这些事件，打破自动答题链。

受影响的事件（需要守卫的）：
- `QuestionReady`
- `NeedCaptcha`
- `CaptchaData`
- `LlmOk`
- `LlmErr`
- `SubmitOk`
- `SubmitFail`
- `QuizDone`

## Scope
- 仅修改 `src/app.rs` 的 `process()` 方法
- 添加页面守卫，无其他行为变更
