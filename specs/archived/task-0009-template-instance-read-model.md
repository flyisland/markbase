---
id: task-0009
title: "实现 `_schema.create` 模板读模型与实例创建"
status: completed
exec-plan: exec-003
phase: 1
boundaries:
  allowed:
    - "src/template.rs"
    - "src/creator.rs"
    - "src/describe.rs"
    - "tests/cli_note.rs"
    - "tests/e2e_complete.rs"
  forbidden_patterns:
    - "src/verifier.rs"
    - "docs/design-docs/implemented/design-006-template-system.md"
    - "docs/design-docs/implemented/design-004-note-verify.md"
completion_criteria:
  - id: "cc-001"
    scenario: "template normalization creates `_schema.create` and does not treat arbitrary outer frontmatter as instance defaults"
    test: "template::tests::test_from_content_normalizes_create_block"
  - id: "cc-002"
    scenario: "`note new --template` materializes frontmatter from `_schema.create`"
    test: "test_note_create_with_template_uses_schema_create"
  - id: "cc-003"
    scenario: "`note new --template` auto-injects `templates: [[template-name]]`"
    test: "test_note_create_with_template_auto_injects_templates_field"
  - id: "cc-004"
    scenario: "legacy outer frontmatter `templates` is not copied into created instances during compatibility fallback"
    test: "test_note_create_with_template_ignores_legacy_outer_templates_field"
  - id: "cc-005"
    scenario: "`template describe` shows normalized `_schema.create` content used by creation paths"
    test: "describe::tests::test_describe_template_shows_normalized_create_block"
---

## Intent

建立 `design-007` 定义的新模板读模型，让 `_schema.create` 成为 `note new --template` 的实例 frontmatter 来源，并把 `templates` 切换为系统自动注入字段。

这个任务只负责模板归一化、实例创建和 describe 读取一致性，不负责 verifier 的语义切换；那部分由后续任务单独处理。

## Decisions

- `TemplateDocument` 必须显式归一化 `_schema.create` 为 object；缺失或非 object 时归一化为空 object
- `render_for_create()` 必须基于 `_schema.create` 物化实例 frontmatter，而不是 clone outer frontmatter 后删 `_schema`
- `description` 仍然是创建时必须存在的实例字段；如果 `_schema.create.description` 缺失或非 string，创建路径必须补成空字符串
- `note new --template <name>` 必须自动注入 `templates: ["[[<name>]]"]`
- 模板 outer frontmatter `templates` 不再具有实例创建语义
- 兼容期内若保留 legacy fallback，则 `_schema.create` 优先于 outer frontmatter seed fields
- `template describe` 必须展示用于创建语义的归一化 `_schema.create`，避免 describe 和 create 看见不同模板视图

## Boundaries

### Allowed Changes

- `src/template.rs`
- `src/creator.rs`
- `src/describe.rs`
- 与模板创建相关的 CLI / e2e 测试

### Forbidden

- 不得在本任务中修改 `src/verifier.rs` 的验证语义
- 不得提前合并 `design-007` 到 `design-006`
- 不得在本任务中发明新的 schema 关键字来表达 identity / mutability
- 不得让 `templates` 继续依赖模板作者在模板文件中手写

## Completion Criteria

场景: template normalization creates `_schema.create` and does not treat arbitrary outer frontmatter as instance defaults
测试: template::tests::test_from_content_normalizes_create_block
假设 模板缺失 `_schema.create`，并且 outer frontmatter 含有 legacy instance-like fields
当   模板被 `TemplateDocument::from_content()` 归一化
那么 `_schema.create` 存在且为 object
并且 实例物化路径不会把 arbitrary outer frontmatter 自动视为实例默认值

场景: `note new --template` materializes frontmatter from `_schema.create`
测试: test_note_create_with_template_uses_schema_create
假设 模板在 `_schema.create` 中声明 `type: company` 与 `tags: []`
当   执行 `markbase note new acme --template company_customer`
那么 生成的实例 frontmatter 包含这些字段
并且 字段来源是 `_schema.create` 而不是 outer frontmatter clone

场景: `note new --template` auto-injects `templates: [[template-name]]`
测试: test_note_create_with_template_auto_injects_templates_field
假设 模板没有在 `_schema.create` 中声明 `templates`
当   执行 `markbase note new acme --template company_customer`
那么 生成的实例 frontmatter 仍然包含 `templates: ["[[company_customer]]"]`

场景: legacy outer frontmatter `templates` is not copied into created instances during compatibility fallback
测试: test_note_create_with_template_ignores_legacy_outer_templates_field
假设 旧模板仍在 outer frontmatter 中携带 `templates`
当   执行 `note new --template`
那么 创建结果中的 `templates` 只来自系统自动注入
并且 不会把 legacy outer value 当作作者输入再次复制

场景: `template describe` shows normalized `_schema.create` content used by creation paths
测试: describe::tests::test_describe_template_shows_normalized_create_block
假设 模板经过归一化后 `_schema.create` 含默认 object 与 description 补齐行为
当   执行 `template describe <name>`
那么 输出反映的正是创建路径使用的归一化模板视图
