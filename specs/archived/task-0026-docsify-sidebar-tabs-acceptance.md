---
id: task-0026
title: "收口 docsify sidebar tabs 文档与验收"
status: completed
design: design-014
exec-plan: exec-007
phase: 2
boundaries:
  allowed:
    - "AGENTS.md"
    - "README.md"
    - "ARCHITECTURE.md"
    - "specs/archived/task-0026-docsify-sidebar-tabs-acceptance.md"
  forbidden_patterns:
    - "specs/**"
    - "src/**"
completion_criteria:
  - id: "cc-001"
    scenario: "README 与 ARCHITECTURE 记录 tabs / sidebar scroll / docsify route adaptation 的 frontend ownership"
    test: "doc review"
  - id: "cc-002"
    scenario: "AGENTS.md 导航在 `design-014` 仍为 candidate 时保持正确"
    test: "doc review"
  - id: "cc-003"
    scenario: "浏览器验收覆盖 tabs 切换、长 sidebar 滚动与 docsify link navigation"
    test: "manual browser acceptance"
  - id: "cc-004"
    scenario: "新测试假设替换旧 stacked-section 假设后仓库级验证通过"
    test: "cargo test && cargo clippy -- -D warnings && cargo fmt --check && specmate check"
---

## Intent

在 tabs/scroll/link adaptation 实现完成后，收口仓库级文档与最终验收。

这个任务负责把新设计合同写回 README、ARCHITECTURE 与 AGENTS，并明确旧 stacked-section 假设已被替换。

## Decisions

- tabs / scroll / docsify route adaptation 都属于 docsify frontend ownership，不是 backend metadata contract 的一部分
- 仓库级文档需要明确 sidebar 现在是 tabbed sidebar，而不是 stacked sections
- 浏览器验收必须覆盖 sidebar 长内容滚动与 sidebar 内部 link 保持 docsify 内导航
- 旧测试若体现“Properties 与 Links 同时显示”，必须被替换或重写，不得与当前设计并存

## Boundaries

### Allowed Changes

- AGENTS.md
- README.md
- ARCHITECTURE.md
- specs/archived/task-0026-docsify-sidebar-tabs-acceptance.md

### Forbidden

- 不得借文档收口之名继续扩展 sidebar scope
- 不得修改 backend metadata contract 来迁就 tabs/link 行为
- 不得回到实现代码层继续修改 `src/**`

## Completion Criteria

场景: README 与 ARCHITECTURE 记录 tabs / sidebar scroll / docsify route adaptation 的 frontend ownership
测试: doc review
假设 docsify sidebar tabs 与 routing 行为已实现
当   检查 README 与 ARCHITECTURE
那么 文档明确说明 sidebar 使用 tabs
并且 明确说明 sidebar 拥有自己的 scroll container
并且 明确说明 sidebar note/base links 在 docsify shell 内导航

场景: AGENTS.md 导航在 `design-014` 仍为 candidate 时保持正确
测试: doc review
假设 `design-014` 仍为 candidate design doc
当   检查 AGENTS.md
那么 docsify sidebar 相关导航继续指向正确的 candidate design doc
并且 不会把它误写成 implemented contract

场景: 浏览器验收覆盖 tabs 切换、长 sidebar 滚动与 docsify link navigation
测试: manual browser acceptance
假设 docsify shell 已可运行
当   浏览器打开 eligible `.md` note route、切换 `Properties` / `Links` tab、查看长 properties 内容、并点击 sidebar note/base link
那么 sidebar 以 tabs 呈现
并且 sidebar 内容区可以独立滚动
并且 点击 sidebar note/base link 后继续停留在 docsify app 内

场景: 新测试假设替换旧 stacked-section 假设后仓库级验证通过
测试: cargo test && cargo clippy -- -D warnings && cargo fmt --check && specmate check
假设 所有实现与文档改动已完成
当   运行仓库级验证
那么 所有命令通过
并且 不留与旧 stacked-section 假设冲突的测试或文档
