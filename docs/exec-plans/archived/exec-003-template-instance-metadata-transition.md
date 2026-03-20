---
id: exec-003
title: "Template Instance Metadata Transition"
status: completed
design-doc: design-006
parallel_safe_verified: false
---

## Goal

完成模板创建 metadata transition，把模板创建默认值从 legacy outer frontmatter 迁移到 `_schema.create`，并让 `note new --template` 与 `note verify` 对这一模型达成一致。

这个计划的目标不是只让 `_schema.create` 能被读取，而是完成一次完整的模板语义切换：

- `note new --template` 从 `_schema.create` 物化实例 frontmatter
- `templates` 变为系统自动注入字段，而不是模板作者手写字段
- `note verify` 不再把模板 outer frontmatter 当作实例字面约束来源
- stable identity 字段例如 `type` 能被持续验证，而 mutable seed 字段例如 `status` 不会被错误冻结
- patch 设计在实现完成后并回正式设计文档与 README

## Phases

### Phase 1: 模板读模型与实例创建

- [x] task-0009: 为模板归一化、`template describe` 与 `note new --template` 引入 schema-owned create surface，并自动注入 `templates`

### Phase 2: 验证语义切换

- [x] task-0010: 将 `note verify` 从 outer frontmatter literal-match 语义切换到 `_schema.required` / `_schema.properties` 语义，并落实 stable identity rule

### Phase 3: 文档收口与兼容清理

- [x] task-0011: 将 patch 设计并回正式文档，更新 README，并明确兼容行为的最终状态

## Execution Mode

按顺序串行执行，不并行。

原因：

- `task-0009` 定义新的模板读模型和实例物化路径，是 `task-0010` 的前置条件
- `task-0010` 必须在 `task-0009` 稳定后再切换 verifier，否则创建语义和验证语义会短暂分叉
- `task-0011` 必须在实现收敛后再更新正式对外合同，避免过早锁死兼容细节

## Dependencies

task-0009 -> task-0010 -> task-0011

## Decision Log

### 2026-03-18: 使用 exec plan，而不是单一 task spec

原因：这次变更跨越模板归一化、实例创建、验证逻辑、测试和正式文档，而且有明确阶段依赖，不适合压缩成一个 task contract。

### 2026-03-18: 将 `note verify` 语义切换独立成单独 task

原因：schema-owned create surface 的最大风险不是读取实现，而是 verifier 是否会错误地把 seed literal 当作持续约束。把验证切换单列，便于审查 stable identity rule 是否被正确落地。

### 2026-03-18: 将正式文档合并与 patch 清理放到最后

原因：`design-007` 当前是 patch contract。只有实现、兼容策略和测试都稳定后，才能安全并回 `design-006` 与 `design-004`，避免正式文档反复改写。

## Progress Notes

- 2026-03-18: `docs/design-docs/obsolete/design-006-patch-01-template-instance-metadata-transition.md` 的前身已建立，并明确 schema-owned create surface、system-derived `templates`、stable identity rule 与 verifier 语义切换
- 2026-03-18: 本计划与 `task-0009` 至 `task-0011` 作为该 patch 的实现入口
- 2026-03-19: `_schema.create` 创建模型、schema-first verifier 语义、README 与正式设计文档已完成收口，`design-007` 已归档并注明 merged ownership

## Definition of Done

`exec-003` 只有在以下条件全部满足时才算完成：

1. 模板归一化层支持 `_schema.create`，并且不再把 arbitrary outer frontmatter 当作实例骨架
2. `note new --template` 基于 `_schema.create` 创建实例 note
3. `note new --template <name>` 总是自动注入 `templates: ["[[<name>]]"]`
4. `note verify` 不再对模板 outer frontmatter 执行 legacy literal-match 校验
5. stable identity 字段例如 `type` 能通过 `_schema.required` 与 `_schema.properties` 被持续验证
6. mutable seed 字段例如 `status` 不会因为 `_schema.create.status: Lead` 而被永久要求等于 `Lead`
7. template describe、create、verify 的共享模板语义在测试中得到覆盖
8. `design-006`、`design-004`、`README.md` 与最终实现一致
9. `design-007` 被移除或归档，不再作为 active patch contract
10. `cargo test`、`cargo clippy -- -D warnings`、`cargo fmt --check` 通过

## Blocking Rules

执行过程中如果遇到以下情况，不要自行扩展语义，必须先回到 `design-007` 对齐：

- 想把 `_schema.properties.default` 自动升级为实例物化值
- 想让 `_schema.create` 重新承担 generic exact-match verify 语义
- 想把 `templates` 重新暴露为模板作者必须手写的实例字段
- 想保留 outer frontmatter literal-match 校验作为新旧并存的长期语义
- 想在未明确文档合同前临时发明 `const`、`seed_only` 等新 schema 关键字
