---
id: task-0008
title: "同步 note embed render 文档与回归测试"
status: completed
exec-plan: exec-002
phase: 3
boundaries:
  allowed:
    - "README.md"
    - "tests/cli_note.rs"
    - "docs/design-docs/implemented/design-002-render.md"
    - "docs/design-docs/implemented/design-001-links-and-embeds.md"
  forbidden_patterns:
    - "specs/**"
    - "src/query/**"
completion_criteria:
  - id: "cc-001"
    scenario: "README 对 note embed 的用户可见行为与实现一致"
    test: "test_render_note_embed_behavior_matches_readme_examples"
  - id: "cc-002"
    scenario: "render 设计文档中的 note embed 合同由回归测试覆盖"
    test: "test_render_note_embed_behavior_matches_design_contract"
---

## Intent

在 renderer 实现完成后，同步 README 与回归测试，使 note embed render 的用户可见行为、active 设计文档和可执行验收保持一致。

由于本次设计文档已经先行更新，这个任务的重点不是重新定义语义，而是确认 README 与最终实现收敛到同一份合同上。

## Decisions

- `design-002-render.md` 是 render 行为的主设计文档
- `design-001-links-and-embeds.md` 记录 shared link/embed 视角下的 render-time 边界
- README 只记录用户可见行为与示例，不重复 renderer 内部实现结构
- 如果实现与当前设计文档冲突，先根据通过测试的最终实现收敛，再回写文档或测试
- note embed 相关 README 内容必须明确四点：只展开 body、不输出 frontmatter；inline embed 拆行；循环嵌入是 warning + placeholder 的软失败；nested `.base` 使用当前 embedded note 作为 `this`

## Boundaries

### Allowed Changes

- README.md
- tests/cli_note.rs
- docs/design-docs/implemented/design-002-render.md
- docs/design-docs/implemented/design-001-links-and-embeds.md

### Forbidden

- 不得在 README 中承诺尚未实现的 selector note embed 能力
- 不得通过弱化 README 表述来掩盖实现偏差
- 不得删除关键回归测试来制造“文档一致”

## Completion Criteria

场景: README 对 note embed 的用户可见行为与实现一致
测试: test_render_note_embed_behavior_matches_readme_examples
假设 README 已记录 note embed 展开、inline 拆行和循环软失败行为
当   执行对应 CLI 回归测试
那么 行为与 README 约定一致

场景: render 设计文档中的 note embed 合同由回归测试覆盖
测试: test_render_note_embed_behavior_matches_design_contract
假设 active 设计文档已记录 body-only、recursive expansion、cycle guard 合同
当   执行 render 回归测试
那么 至少存在覆盖这些行为的测试，并且实现与文档一致
