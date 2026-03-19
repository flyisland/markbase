---
id: task-010
title: "切换 `note verify` 到 `_schema.create` 时代的模板语义"
status: completed
exec-plan: exec-003
phase: 2
boundaries:
  allowed:
    - "src/verifier.rs"
    - "tests/cli_note.rs"
    - "tests/e2e_complete.rs"
  forbidden_patterns:
    - "src/creator.rs"
    - "docs/design-docs/design-006-template-system.md"
    - "README.md"
completion_criteria:
  - id: "cc-001"
    scenario: "`note verify` stops using template outer frontmatter as literal-match constraints"
    test: "test_note_verify_ignores_template_outer_frontmatter_seed_fields"
  - id: "cc-002"
    scenario: "stable identity field `type` is enforced through `_schema.required` and `_schema.properties`, not by seed literal equality alone"
    test: "test_note_verify_type_identity_enforced_via_schema"
  - id: "cc-003"
    scenario: "mutable seed field `status` may evolve after creation without verify forcing the seed literal"
    test: "test_note_verify_mutable_seed_status_not_frozen_to_initial_value"
  - id: "cc-004"
    scenario: "template files are not treated as ordinary self-verifying instances under the new semantics"
    test: "test_note_verify_template_file_is_not_a_valid_instance_target"
  - id: "cc-005"
    scenario: "location, required, property, link, and embedded `.base` checks continue to work after the transition"
    test: "test_note_verify_template_semantics_preserved_after_instance_transition"
---

## Intent

把 verifier 从 legacy outer-frontmatter literal-match 模式迁移到 `design-007` 规定的 schema-first 模式。

这个任务的关键不是简单删除几条校验，而是明确 `_schema.create` 在 verify 中只承担 creation blueprint 角色，持续约束必须来自 `_schema.required` 与 `_schema.properties`。

## Decisions

- verifier 继续从实例 note 的 `templates` 字段解析模板名
- verifier 继续检查 `_schema.location`、`_schema.required`、`_schema.properties`、link target 和 template body `.base` embed 约束
- verifier 必须移除对模板 outer frontmatter non-`_schema` 字段的 missing / scalar mismatch / list containment 校验
- `_schema.create` 不是 generic exact-match verify surface
- stable identity 字段例如 `type` 必须通过 `_schema.required` 和 `_schema.properties` 建模后才能被持续验证
- mutable seed 字段例如 `status` 只因为存在于 `_schema.create`，不能被 verifier 永久要求等于初始值
- 如果 `note verify <template-name>` 的命令可见行为改变，测试必须明确覆盖新的拒绝或新消息

## Boundaries

### Allowed Changes

- `src/verifier.rs`
- 与 verify 行为相关的 CLI / e2e 测试

### Forbidden

- 不得在本任务中改写 `note new --template` 创建逻辑
- 不得把 `_schema.properties.default` 当作 verify literal source
- 不得为解决 identity 建模临时发明未写入设计的 schema key
- 不得在 README 和正式设计文档中提前收口最终合同

## Completion Criteria

场景: `note verify` stops using template outer frontmatter as literal-match constraints
测试: test_note_verify_ignores_template_outer_frontmatter_seed_fields
假设 模板 outer frontmatter 含有 legacy seed fields
当   验证由该模板创建的实例 note
那么 verifier 不会再因为这些 outer frontmatter seed fields 缺失或值不同而报错

场景: stable identity field `type` is enforced through `_schema.required` and `_schema.properties`, not by seed literal equality alone
测试: test_note_verify_type_identity_enforced_via_schema
假设 模板声明 `_schema.create.type: company`，并通过 `_schema.required` 与 `_schema.properties.type.enum: [company]` 建模
当   实例 note 的 `type` 被改为非 `company`
那么 verifier 报错
并且 该错误来源于 schema constraint，而不是 legacy outer-frontmatter literal match

场景: mutable seed field `status` may evolve after creation without verify forcing the seed literal
测试: test_note_verify_mutable_seed_status_not_frozen_to_initial_value
假设 模板声明 `_schema.create.status: Lead`，同时 `_schema.properties.status.enum` 允许 `Lead`、`Active`、`Closed Won`
当   实例 note 的 `status` 变为 `Active`
那么 verifier 通过
并且 不会因为初始 seed 是 `Lead` 而要求 literal equality

场景: template files are not treated as ordinary self-verifying instances under the new semantics
测试: test_note_verify_template_file_is_not_a_valid_instance_target
假设 用户直接执行 `markbase note verify company_customer`
当   `company_customer` 命中的是 `templates/company_customer.md`
那么 verifier 不应把该模板文件当作普通实例进行 schema 验证
并且 输出应符合最终实现约定的拒绝或专用错误语义

场景: location, required, property, link, and embedded `.base` checks continue to work after the transition
测试: test_note_verify_template_semantics_preserved_after_instance_transition
假设 模板已迁移到 `_schema.create`
当   验证包含 location、required、enum、link target 与 embedded `.base` 约束的实例 note
那么 这些既有 schema-driven 检查继续正常工作
