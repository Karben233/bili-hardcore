# 答题界面 AI 思考内容自动滚动

## Goal

答题界面左侧栏的「AI 思考内容」是流式追加的。当内容超出可见区域时，当前渲染从顶部开始、无滚动偏移，导致**最新追加的思考内容被截断看不到**。需要让思考区域自动跟随到最新内容（底部），让用户实时看到 AI 的思考进展。

## What I already know

- 左侧栏渲染位于 [quiz.rs:234-345](src/ui/quiz.rs#L234-L345)：`left[1]` 是一个 `Paragraph`，依次包含 选项 → 空行 → 状态行（如 "● AI 思考中..."）→ 空行 → `app.thinking_text`，使用 `Wrap { trim: true }`，**无 `.scroll()`**。
- `thinking_text` 流式追加（[app.rs:870](src/app.rs#L870) `push_str`），每题开始时 `clear()`（[app.rs:825](src/app.rs#L825)、[app.rs:919](src/app.rs#L919)）。
- 右侧「已答题目」历史栏已有手动滚动参考实现：`.scroll((app.history_scroll as u16, 0))`（[quiz.rs:411](src/ui/quiz.rs#L411)），由 ↑↓ 键控制（[input.rs:237-240](src/input.rs#L237-L240)）。
- 思考内容渲染在 `WaitingLlm | WaitingRetry | Submitting | ShowingResult` 四个阶段共享（[quiz.rs:170](src/ui/quiz.rs#L170)）。
- 依赖可用：`ratatui = "0.29"`、`unicode-width = "0.2"`（直接依赖，可直接用于中文/Emoji 宽度计算）。
- ratatui `Paragraph` 默认从顶部渲染，超出区域底部的部分被丢弃 → 这正是最新思考看不到的根因。

## Assumptions (temporary)

- 自动滚动即「跟随最新」：scroll offset = `max(0, wrap后总行数 - 可见高度)`。
- 不需要给思考区加手动 ↑↓ 滚动（↑↓ 已分配给右侧历史栏，冲突；且自动跟随已满足核心诉求）。

## Open Questions

- ~~[Preference] 选项 + 状态行是否需要「始终固定可见」？~~ → 已决定：**是，采用方案 B**。

## Requirements (evolving)

- 思考内容超出可见区域时，自动滚动使**最新内容（底部）可见**。
- 流式追加过程中视图持续跟随到底部。
- 思考内容较短（未超出）时，正常从顶部显示，无空滚动。
- 不破坏现有布局：题目区、选项、状态行、右侧历史栏（含其手动滚动）行为不变。
- 覆盖全部四个渲染阶段（WaitingLlm / WaitingRetry / Submitting / ShowingResult）。

## Acceptance Criteria (evolving)

- [ ] 深度思考模型产出超长 thinking 时，最新思考行始终可见（不被截断）。
- [ ] 短 thinking 不滚动，选项与状态行正常显示。
- [ ] 右侧历史栏 ↑↓ 滚动不受影响。
- [ ] `cargo build` / `cargo clippy` / `cargo fmt` 通过。

## Definition of Done (team quality bar)

- Lint / typecheck / fmt 绿色（本项目无单元测试覆盖 UI 渲染，以编译 + clippy + 手动验证为准）。
- 行为变更无需单独文档（TUI 内部交互）。

## Out of Scope (explicit)

- 思考内容的手动滚动 / 暂停跟随 / 折叠展开（自动跟随已满足诉求）。
- 右侧历史栏改动。
- `answer_text` 的展示（当前未在 UI 渲染）。

## Technical Approach

### 核心工具：wrap 后行数计算

中文 / Emoji 在终端占 2 列，不能按字符数估算。新增工具函数，基于 `unicode-width` 计算给定文本在指定列宽下折行后的实际行数：

```rust
// 思考内容是纯文本 String，按 .lines() 分段，每段按 unicode 宽度 / area.width 向上取整
fn wrapped_text_height(text: &str, width: u16) -> usize { /* ... */ }
```

### 方案 A：整个 `left[1]` 自动滚到底（简单）

- 给现有 `Paragraph::new(lines)` 追加 `.scroll((offset, 0))`，offset = `max(0, 总行数 - left[1].height)`。
- 改动最小。
- 缺点：thinking 很长时，**选项与状态行会被滚出视野**；ShowingResult 阶段看不到选项高亮（✓/✗）。

### 方案 B：选项/状态固定 + 思考区独立自动滚动（推荐）

- 把 `left[1]` 再纵向切分：上部 `Constraint::Length(opt_height)` 放 选项+状态行（不滚动），下部 `Constraint::Min(0)` 只放 `thinking_text` 并自动滚动。
- `opt_height` 用同样的 wrap 计算得出（先取 `left[1].width`，再算行数，再 split）。
- 优点：选项始终可见，体验最佳。
- 代价：多一个工具函数 + 两步 layout。

## Decision (ADR-lite)

**Context**: 思考内容自动滚动到底部时，需决定选项 + 状态行是否固定可见——影响实现复杂度与 ShowingResult 阶段体验。

**Decision**: 采用 **方案 B** —— 左侧 `left[1]` 纵向切分为两块：上部 `Constraint::Length(opt_height)` 固定显示「选项 + 空行 + 状态行 + 空行」（不滚动），下部 `Constraint::Min(0)` 只渲染 `thinking_text` 并自动滚动到底。

**Consequences**:
- 体验最佳：选项始终可见，ShowingResult 阶段 ✓/✗ 高亮不被滚走。
- 代价：新增一个基于 `unicode-width` 的折行行数计算函数；`opt_height` 需先取 `left[1].width` 再 wrap 估算（两步 split，避免 layout 死循环）。
- 折行行数为估算（按 unicode 宽度 / 列宽 向上取整），与 ratatui 实际 wrap 在中英混排边界可能差 1 行；用于 scroll offset 足够（最多多滚 1 行，仍保证最新内容可见）。

## Technical Notes

- 涉及文件：`src/ui/quiz.rs`（主），可能新增 `src/ui/mod.rs` 或 quiz.rs 内的私有工具函数。
- `unicode-width` 已是直接依赖，无需新增。
- ratatui `Line::width()` 也基于 unicode-width，可用于 Vec<Line> 的高度估算。
