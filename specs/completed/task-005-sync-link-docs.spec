---
id: task-005
title: "同步 link 设计文档与回归测试"
status: completed
exec-plan: exec-001
phase: 3
boundaries:
  allowed:
    - "docs/design-docs/design-001-links-and-embeds.md"
    - "README.md"
    - "AGENTS.md"
    - "ARCHITECTURE.md"
    - "tests/cli_note.rs"
    - "tests/cli_index.rs"
    - "tests/e2e_complete.rs"
  forbidden_patterns:
    - "specs/**"
    - "src/query/**"
completion_criteria:
  - id: "cc-001"
    scenario: "索引、rename、render 对 escaped pipe 与 `.md#anchor` 语义保持一致"
    test: "test_link_semantics_are_consistent_across_index_rename_and_render"
  - id: "cc-002"
    scenario: "code context 中的 link/embed 语法不会进入索引，也不会被 rename 或 render 当作真实语义执行"
    test: "test_code_context_links_are_ignored_across_features"
  - id: "cc-003"
    scenario: "`.base#View` 渲染行为在文档与实现之间一致"
    test: "test_render_view_selector_matches_documented_behavior"
---

## Intent

在共享解析层和消费方迁移完成后，同步 markbase 对外文档与跨模块回归测试，确保新的 link/embed 行为成为显式合约，而不是只存在于实现细节中。

## Decisions

- `design-001-links-and-embeds.md` 是 link/embed 行为的主设计文档
- README 只记录用户可见行为，不重复实现细节
- AGENTS 与 ARCHITECTURE 只更新入口路径和 ownership，不重复设计全文
- 需要把 `design-001` 中所有“当前 regex 限制”改写为最终共享解析层 contract
- 文档与测试冲突时，以通过测试的最终实现为准，然后同步回文档

## Boundaries

### Allowed Changes

- docs/design-docs/design-001-links-and-embeds.md
- README.md
- AGENTS.md
- ARCHITECTURE.md
- tests/cli_note.rs
- tests/cli_index.rs
- tests/e2e_complete.rs

### Forbidden

- 不得新增未在 design-001 中说明的 link/embed 语义
- 不得通过删减测试来规避跨模块不一致
- 不得修改其他无关设计文档来掩盖行为变化

## Completion Criteria

场景: 索引、rename、render 对 escaped pipe 与 `.md#anchor` 语义保持一致
测试: test_link_semantics_are_consistent_across_index_rename_and_render
假设 同一组示例同时经过索引、rename、render
当   执行端到端验证
那么 三条路径对 target 的识别与保留后缀规则一致

场景: code context 中的 link/embed 语法不会进入索引，也不会被 rename 或 render 当作真实语义执行
测试: test_code_context_links_are_ignored_across_features
假设 样例文件同时包含正文、fenced code、inline code 三类上下文
当   执行索引、rename 和 render
那么 只有正文中的真实链接或 `.base` embed 生效

场景: `.base#View` 渲染行为在文档与实现之间一致
测试: test_render_view_selector_matches_documented_behavior
假设 README 和 design-001 已记录 `.base#View` 语义
当   执行渲染测试
那么 结果与文档约定一致
