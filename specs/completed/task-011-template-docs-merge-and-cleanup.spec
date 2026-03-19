---
id: task-011
title: "合并模板实例 metadata patch 到正式文档"
status: completed
exec-plan: exec-003
phase: 3
boundaries:
  allowed:
    - "docs/design-docs/design-006-template-system.md"
    - "docs/design-docs/design-004-note-verify.md"
    - "docs/design-docs/legacy/design-007-template-instance-metadata-patch.md"
    - "docs/DESIGN.md"
    - "README.md"
    - "tests/cli_note.rs"
  forbidden_patterns:
    - "src/**"
completion_criteria:
  - id: "cc-001"
    scenario: "`design-006` reflects `_schema.create` as the active template creation model"
    test: "doc review"
  - id: "cc-002"
    scenario: "`design-004` reflects the new verifier semantics for seed values and stable identity fields"
    test: "doc review"
  - id: "cc-003"
    scenario: "README template examples no longer instruct users to author legacy outer frontmatter seed fields"
    test: "doc review"
  - id: "cc-004"
    scenario: "`design-007` is removed or archived once its content is absorbed"
    test: "doc review"
---

## Intent

在实现完成后，把 patch contract 收口到正式文档体系中，避免 `design-007` 长期悬挂为第二套 active source of truth。

这个任务只处理对外合同与文档归并，不再修改实现代码。

## Decisions

- `design-006` 必须接收 `_schema.create`、system-derived `templates` 与新 creation contract
- `design-004` 必须接收 verifier 侧的语义切换，包括 stable identity rule 的落地结果
- README 中所有模板示例都必须改为新作者面，不再要求在模板 outer frontmatter 中手写 `type`、`templates` 等 seed fields
- `design-007` 完成使命后必须被删除或移入 legacy，不能长期与 `design-006` 共同作为 active contract

## Boundaries

### Allowed Changes

- 正式设计文档
- README
- 如有必要的文档对齐测试

### Forbidden

- 不得在本任务中继续改动 Rust 实现逻辑
- 不得在实现未完成前提前移除 `design-007`
- 不得保留与实现不一致的双重合同

## Completion Criteria

场景: `design-006` reflects `_schema.create` as the active template creation model
测试: doc review
假设 前两阶段实现已完成
当   阅读 `design-006`
那么 其创建语义、归一化规则和 example 都以 `_schema.create` 为正式 active contract

场景: `design-004` reflects the new verifier semantics for seed values and stable identity fields
测试: doc review
假设 verifier 语义已切换
当   阅读 `design-004`
那么 文档明确 outer frontmatter literal-match 已废止
并且 明确 stable identity 字段与 mutable seed 字段的建模差异

场景: README template examples no longer instruct users to author legacy outer frontmatter seed fields
测试: doc review
假设 用户查看 `template describe` / `note new --template` 相关说明
当   阅读 README 示例
那么 示例只展示 `_schema.create` 作者面与 system-derived `templates`

场景: `design-007` is removed or archived once its content is absorbed
测试: doc review
假设 正式文档已经完成吸收
当   检查 active design docs
那么 `design-007` 不再作为 active patch contract 留存
