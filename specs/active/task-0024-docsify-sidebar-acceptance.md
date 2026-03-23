---
id: task-0024
title: "收口 docsify sidebar 文档与验收"
status: draft
design: design-014
exec-plan: exec-006
phase: 3
boundaries:
  allowed:
    - "AGENTS.md"
    - "README.md"
    - "ARCHITECTURE.md"
    - "specs/active/task-0024-docsify-sidebar-acceptance.md"
  forbidden_patterns:
    - "specs/**"
    - "src/**"
completion_criteria:
  - id: "cc-001"
    scenario: "README 与 ARCHITECTURE 记录 sidebar 的 note-only 适用范围与 frontend ownership"
    test: "doc review"
  - id: "cc-002"
    scenario: "AGENTS.md 的阅读导航在 `design-014` implemented 后保持正确"
    test: "doc review"
  - id: "cc-003"
    scenario: "浏览器验收覆盖 note route、TOC 锚点跳转、unsupported route 与 sidebar state"
    test: "manual browser acceptance"
  - id: "cc-004"
    scenario: "实现完成后仓库级验证通过"
    test: "cargo test && cargo clippy -- -D warnings && cargo fmt --check && specmate check"
---

## Intent

落地 sidebar UI 后的最终文档收口与验收。

这个任务的重点不是再发明新 UI，而是把已经实现的 sidebar contract 固定到 README、ARCHITECTURE 与 AGENTS 中，并通过浏览器验收把这次 bug class 封住。
这个任务假定 `task-0023` 已经完成实现代码与 `design-014` 的生命周期迁移；它只负责仓库级文档收口和最终验收。

## Decisions

- sidebar 是 docsify frontend behavior，不是后端 metadata contract 的一部分
- README 与 ARCHITECTURE 必须明确 sidebar 只对 canonical `.md` note route 生效
- AGENTS.md 的 “Read First” / “Task Navigation” 需要在 `design-014` implemented 后反映新的 active doc
- 浏览器验收必须显式覆盖 TOC `?id=...` 锚点跳转不触发错误 metadata request 这一回归场景

## Boundaries

### Allowed Changes

- AGENTS.md
- README.md
- ARCHITECTURE.md
- specs/active/task-0024-docsify-sidebar-acceptance.md

### Forbidden

- 不得借文档收口之名继续扩展 sidebar scope
- 不得修改 `design-013` 的 backend metadata contract 来掩盖前端问题
- 不得回到实现代码层继续修改 `src/**`

## Completion Criteria

场景: README 与 ARCHITECTURE 记录 sidebar 的 note-only 适用范围与 frontend ownership
测试: doc review
假设 docsify sidebar UI 已实现
当   检查 README 与 ARCHITECTURE
那么 文档明确说明 sidebar 属于 docsify shell 前端能力
并且 明确说明只有 canonical `.md` note route 使用 metadata sidebar

场景: AGENTS.md 的阅读导航在 `design-014` implemented 后保持正确
测试: doc review
假设 `design-014` 已成为 implemented design doc
当   检查 AGENTS.md
那么 Web note view / docsify frontend 相关导航包含正确的 implemented `design-014` 路径
并且 不再把旧 draft 路径当作 active contract

场景: 浏览器验收覆盖 note route、TOC 锚点跳转、unsupported route 与 sidebar state
测试: manual browser acceptance
假设 docsify shell 已可运行
当   浏览器分别打开 eligible `.md` note route、点击左侧 TOC、切到 `.base` route、并制造 metadata error / empty state
那么 `.md` note route 正常显示 sidebar
并且 TOC 锚点跳转不会触发新的 metadata request 或 404
并且 `.base` route 不会显示伪 error sidebar
并且 loading / empty / error state 都符合设计合同

场景: 实现完成后仓库级验证通过
测试: cargo test && cargo clippy -- -D warnings && cargo fmt --check && specmate check
假设 所有实现与文档改动已完成
当   运行仓库级验证
那么 所有命令通过
并且 不留未解释的 managed doc 校验问题
