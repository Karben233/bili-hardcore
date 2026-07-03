# 全项目 rustfmt 格式化

## Goal

一次性修复全项目积累的 rustfmt 格式违规，建立干净基线，使 `cargo fmt --check` 在 CI 中可零成本通过。纯机械操作，不改任何逻辑。

## What I already know

- 全项目共 **40 处** fmt 违规，分布在 7 个文件（`cargo fmt -- --check` 调研）：
  - `src/app.rs`（4 处）
  - `src/input.rs`（4 处）
  - `src/llm/mod.rs`（1 处）
  - `src/llm/openai.rs`（3 处）
  - `src/main.rs`（11 处）
  - `src/ui/config_page.rs`（11 处）
  - `src/ui/quiz.rs`（8 处，均为既有代码，非上一任务新增）
- 项目使用默认 rustfmt 配置（无 `rustfmt.toml`）。
- 上一任务（feat(ui) 思考滚动）已确认其新增代码本身格式正确。

## Requirements

- 运行 `cargo fmt`（全项目，默认配置）一次性修复全部 40 处违规。
- 格式化后行为不变：`cargo clippy` 零警告、`cargo test` 通过。

## Acceptance Criteria

- [ ] `cargo fmt -- --check` 无 diff（退出码 0）。
- [ ] `cargo clippy --all-targets` 无警告。
- [ ] `cargo test` 通过（含上一任务新增的 wrapped_text_height 单测）。

## Definition of Done

- fmt / clippy / test 三项全绿。
- 单次提交，message 表明为格式化（无行为变更）。

## Out of Scope

- 不修改任何业务逻辑。
- 不引入 / 修改 `rustfmt.toml` 配置（保持默认）。
- 不处理 fmt 之外的 lint（clippy 已绿）。

## Technical Approach

`cargo fmt`（不带 `--check` 即原地格式化整个 crate）。无需逐文件手动改。

## Decision (ADR-lite)

**Context**: 全项目 40 处 fmt 违规散落 7 文件，阻碍 `cargo fmt --check` 作为质量门。

**Decision**: 一次性 `cargo fmt` 全项目，建立干净基线。

**Consequences**: 单次提交 diff 较大（纯空白 / 换行调整），但从此 fmt --check 可作为 CI 硬门；后续提交若再引入违规可一眼识别。无运行时影响。

## Technical Notes

- 涉及文件：app.rs / input.rs / llm/mod.rs / llm/openai.rs / main.rs / ui/config_page.rs / ui/quiz.rs。
- 复杂度分类：**Trivial**（机械格式化），按 brainstorm skill 跳过问答直接实现。
