---
id: task-007
title: "实现 note embed 的递归展开与循环保护"
status: completed
exec-plan: exec-002
phase: 2
boundaries:
  allowed:
    - "src/renderer/mod.rs"
    - "tests/cli_note.rs"
  forbidden_patterns:
    - "specs/**"
    - "src/query/**"
    - "src/db.rs"
completion_criteria:
  - id: "cc-001"
    scenario: "嵌入 note 内的 note embed 会继续递归展开"
    test: "test_note_render_recursive_note_embeds_are_expanded"
  - id: "cc-002"
    scenario: "嵌入 note 内的 `.base` embed 也会继续展开"
    test: "test_note_render_recursive_note_embed_expands_nested_base_embed"
  - id: "cc-003"
    scenario: "循环嵌入会输出 warning 和 placeholder，但不会中止整个 render"
    test: "test_note_render_recursive_note_embed_cycle_is_soft_failure"
---

## Intent

在基础 note embed 展开已经建立后，为 renderer 增加递归 render 能力，使 embedded note 中出现的 note embed 和 `.base` embed 都按照同一套 contract 继续生效。

这个任务同时负责让递归执行具备可预期的安全边界，避免循环嵌入导致无限递归或栈溢出。

## Decisions

- 递归展开采用 depth-first 顺序
- 递归 note render 继续复用顶层 note render 的 `MarkdownBody` 扫描规则
- cycle guard 以当前 active note render stack 为准，键使用 normalized note name
- 当目标 note 已经出现在 active stack 中时，不再继续递归
- 该情况属于 soft failure：stderr 输出 warning，stdout 在原位输出 placeholder comment，然后继续渲染后续内容
- cycle guard 只针对 Markdown note embed；`.base` 视图执行不加入 note stack
- recursive note render 中遇到缺失 note 或 note 读取失败，也按 soft failure 处理，而不是中止整个 render
- recursive note render 会在每个 embedded note 边界重建当前 note 上下文：embedded note 内部触发的 `.base` 以该 embedded note 作为 `this`
- 递归过程不得改变既有 `.base` render wrapper、`--dry-run` 行为或输出格式语义
- 本任务不改变 direct `.base` render 行为

## Boundaries

### Allowed Changes

- src/renderer/mod.rs
- tests/cli_note.rs

### Forbidden

- 不得把循环嵌入升级为 hard error 并中止整个命令
- 不得通过全局深度上限替代显式 cycle guard
- 不得改变 `.base#View` 的既有选择器语义
- 不得把 cycle guard 键改成 path-based 规则，偏离 note-name identity
- 不得顺手修改 direct `.base` render 语义

## Completion Criteria

场景: 嵌入 note 内的 note embed 会继续递归展开
测试: test_note_render_recursive_note_embeds_are_expanded
假设 `note-a` embed `note-b`，`note-b` embed `note-c`
当   渲染包含 `![[note-a]]` 的顶层 note
那么 输出中会按递归顺序展开 `note-a`、`note-b`、`note-c` 的 body 内容

场景: 嵌入 note 内的 `.base` embed 也会继续展开
测试: test_note_render_recursive_note_embed_expands_nested_base_embed
假设 `note-a` 的 body 中包含 `![[tasks.base]]`
当   另一个 note 通过 `![[note-a]]` 触发其展开
那么 `tasks.base` 的 render 结果也会出现在最终输出中
并且 该 `.base` 执行时使用 `note-a` 的 `this` 上下文，而不是 top-level render target

场景: 循环嵌入会输出 warning 和 placeholder，但不会中止整个 render
测试: test_note_render_recursive_note_embed_cycle_is_soft_failure
假设 `note-a` embed `note-b`，且 `note-b` 再 embed `note-a`
当   渲染包含 `![[note-a]]` 的顶层 note
那么 stderr 包含 `WARN: recursive note embed skipped for 'note-a' to avoid cycle.`
并且 stdout 在循环位置输出 `<!-- [markbase] recursive note embed skipped for 'note-a' -->`
并且 render 仍继续输出循环点之后的其余内容
