---
id: exec-007
title: "Docsify Sidebar Tabs And Routing"
status: completed
design-doc: design-014
parallel_safe_verified: false
---

## Goal

按 [design-014-docsify-note-sidebar-ui.md](../../design-docs/candidate/design-014-docsify-note-sidebar-ui.md)
继续收口 docsify note metadata sidebar 的增量合同，把上一轮 v1 实现中仍不符合当前设计的部分补齐：

- `Properties`、`Links` 与未来 tab 共享同一个 sidebar slot，而不是纵向堆叠
- sidebar 自己拥有 scroll container；长 `Properties` 不应把其他 panel 挤到页面下方
- sidebar 内 note/base links 必须导航到 docsify shell route，而不是直接打开 backend raw Markdown URL

这个计划不是重做 `exec-006`，而是承接 `design-014` 追加明确化后的 follow-up 实现。

## Phases

### Phase 1: Sidebar Structure, Scroll, And Link Routing

- [x] task-0025: 实现 tab strip + active panel、sidebar own scroll，以及 sidebar note/base links 的 docsify route adaptation

### Phase 2: Docs And Acceptance Closure

- [x] task-0026: 更新 README / ARCHITECTURE / AGENTS / acceptance tests，明确替换旧的 stacked-section 假设

## Execution Mode

按顺序串行执行。

原因：

- `task-0025` 同时改变 sidebar DOM contract 与 link href contract，必须作为一个完整实现任务交付，否则会留下半 tabs、半旧链接行为的中间态
- `task-0026` 最后收口，用来把新合同写回仓库级文档并替换旧测试假设

## Dependencies

task-0025 -> task-0026

## Decision Log

### 2026-03-23: tabs/scroll/link adaptation 以 follow-up exec 承接，而不是回改 `exec-006`

原因：`exec-006` 及其 task 已归档完成，代表上一阶段在当时设计理解下的交付历史。现在 `design-014` 明确新增了：

- tab-strip 而不是 stacked sections
- sidebar own scroll container
- sidebar note/base link 必须适配到 docsify route

这些是新增明确化后的合同，不应通过回写归档任务来改写历史。

### 2026-03-23: 需要显式替换旧的 stacked-section 测试假设

原因：当前实现测试把 `Properties` 与 `Links` 视为同时可见的两个 section，这与更新后的 `design-014` 已冲突。

新的执行计划必须明确：

- 哪些旧测试语义不再成立
- 哪些测试需要被替换为 tabs contract / scroll contract / docsify route adaptation contract

否则后续 review 无法区分“历史测试覆盖”与“当前设计覆盖”。

## Definition of Done

`exec-007` 只有在以下条件全部满足时才算完成：

1. eligible note route 的 sidebar 以 tab strip + 单一 active panel 渲染，而不是 stacked sections
2. 默认 active tab 为 `Properties`
3. tab order 稳定为 `Properties`、`Links`，并为未来 tab 扩展预留同一 sidebar slot
4. sidebar 自身拥有 scroll container；长 `Properties` 内容不会把 `Links` panel 挤到页面下方
5. sidebar 内 resolved note/base links 会导航到 docsify shell route，例如 `#/entities/company/acme.md`
6. sidebar 内 resolved resource links 保持直接资源 URL 行为
7. unresolved links 仍保持非 clickable 表达
8. README、ARCHITECTURE 与 AGENTS 反映 tabs / sidebar scroll / docsify route adaptation 的 frontend ownership
9. 回归测试替换旧的 stacked-section 假设，并覆盖 tabs、scroll、docsify route adaptation 与浏览器验收
10. `cargo test`、`cargo clippy -- -D warnings`、`cargo fmt --check` 通过
11. `specmate check` 通过

## Blocking Rules

执行过程中如果遇到以下情况，不要自行扩展语义，必须先回到本计划和 `design-014` 对齐：

- 想把 `.base` route 也纳入 metadata sidebar，而不是先遵守 `design-013`
- 想保留 stacked sections，仅通过视觉手段把它“看起来像 tabs”
- 想让 sidebar 内部 note/base link 直接打开 `/foo.md` raw Markdown 页面
- 想顺手加入 backlinks 数据获取，而不是先只预留 future tab slot
- 想在未替换旧测试语义的情况下保留“Properties 与 Links 同时可见”的断言

## Progress Notes

- 2026-03-23: 建立 `exec-007`，承接 `design-014` 对 tabs、sidebar scroll 与 docsify route adaptation 的增量合同
- 2026-03-23: 完成 `task-0025`，将 sidebar 从 stacked sections 改为 tab strip + active panel，补齐 sidebar own scroll 与 docsify route adaptation
- 2026-03-23: 完成 `task-0026`，收口 README / ARCHITECTURE / acceptance，并通过浏览器验收与仓库级验证
