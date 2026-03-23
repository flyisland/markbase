---
id: exec-006
title: "Docsify Note Sidebar UI"
status: active
design-doc: design-014
parallel_safe_verified: false
---

## Goal

按 [design-014-docsify-note-sidebar-ui.md](../../design-docs/candidate/design-014-docsify-note-sidebar-ui.md)
为 markbase 的 docsify shell 交付第一版 note metadata sidebar UI，并把这次已暴露的
前端路由边界明确收口：

- sidebar 只在 canonical Markdown note route 上出现
- docsify 的 `?id=...` TOC 锚点跳转被视为页内导航，而不是新的 metadata route
- metadata request 始终由前端按 canonical note pathname 单独构造
- `.base` 页面、root shell 路由和其他非 note route 不发 metadata request
- sidebar 渲染只消费 `design-013` 已实现的 semantic metadata contract

这个计划的目标不是笼统地“做个 sidebar”，而是分阶段交付以下能力：

- 先修正 docsify shell 对 route identity、request gating、stale response 与 TOC 锚点跳转的处理
- 再交付 `Properties` / `Links` 的 sidebar 渲染、responsive layout、最小可用视觉层，以及 `design-014` 的最终实现文档收口
- 最后把 README、ARCHITECTURE、AGENTS 和验收测试一起收口，避免实现与仓库级文档再次漂移

## Phases

### Phase 1: Route Eligibility And Fetch Lifecycle

- [ ] task-0022: 建立 docsify metadata sidebar 的 route eligibility、metadata request 构造、`?id=...` 锚点忽略、error/loading state 与 stale-response 防护

### Phase 2: Sidebar Rendering And Layout

- [ ] task-0023: 实现 `Properties` / `Links` 的 semantic rendering、desktop/mobile layout 与最小可用样式

### Phase 3: Documentation Closure And Acceptance

- [ ] task-0024: 收口 README / ARCHITECTURE / AGENTS / browser acceptance，并把最终 sidebar contract 固定下来

## Execution Mode

按顺序串行执行，不并行。

原因：

- `task-0022` 负责修正这次 bug 暴露出的根问题，也就是 docsify route state 与后端 metadata route contract 的边界；不先收敛这层，后续 UI 只会建立在错误的 fetch 行为之上
- `task-0023` 的渲染和布局必须建立在稳定的 sidebar state machine 之上，否则会把 route eligibility、loading、error 和 stale-response 逻辑散落到视觉层
- `task-0024` 负责把最终实现反写到 README、ARCHITECTURE 与 AGENTS，并完成最终验收；必须等前两阶段的行为收敛后再锁定

## Dependencies

task-0022 -> task-0023 -> task-0024

## Decision Log

### 2026-03-23: 将 route / fetch lifecycle 与 UI rendering 拆成两个实现任务

原因：这次故障的根源不是“sidebar 长什么样”，而是 docsify shell 把页内 TOC 导航与 metadata route 混在了一起。

- `design-013` 已明确 metadata mode 只支持 canonical Markdown note route
- `design-014` 现在已明确 `.base`、`/` 和 `?id=...` 锚点变化不应触发 metadata request
- 如果把 route eligibility、request construction 和 visual rendering 混在一个 task 中，review 很容易只看到 DOM/UI，却漏掉真正的 contract boundary

因此必须先单独收敛 route identity 与 request lifecycle，再进入渲染层。

### 2026-03-23: 明确 `.base` 页面不属于 sidebar v1

原因：`.base` route 当前没有 metadata mode，前端若仍试图在 `.base` 页面发 `?fields=properties,links` 请求，只会把后端 `400` 当成 UI 错误。

这不是“错误态要不要好看”的问题，而是请求本身不该发生。v1 sidebar 因此定义为 note-only page chrome。

### 2026-03-23: 将 docsify `?id=...` 视为前端页内导航状态，而不是后端 contract 输入

原因：当前 bug 已证明，若前端直接复用 docsify 当前 URL 去构造 metadata request，就会把 `note.md?id=header-id` 错误地下沉为后端请求目标。

正确边界应当是：

- docsify 可以自由维护自己的 anchor/query state
- metadata fetch 只能由 canonical note pathname 加固定 `fields` query 构造
- 前端不得把 docsify 的页内导航 query 透传给后端 metadata mode

## Progress Notes

- 2026-03-23: 建立 `exec-006` 初稿，按 route lifecycle -> rendering/layout -> docs/acceptance 三阶段拆分 sidebar UI 交付

## Definition of Done

`exec-006` 只有在以下条件全部满足时才算完成：

1. docsify shell 只在 canonical Markdown note route 上显示 metadata sidebar
2. `.base` route、root shell route 与其他非 note route 不会发出 metadata request
3. docsify TOC 触发的 `?id=...` 页内跳转不会触发新的 metadata request，也不会导致对 `/<note>.md?id=...` 的错误后端访问
4. metadata request 始终由 canonical note pathname 加 `?fields=properties,links` 构造，而不是直接复用 docsify 当前 URL
5. sidebar route change handling 对快速导航具备 stale-response overwrite 防护
6. metadata request 失败不会阻塞 note body；error state 被限制在 sidebar 区域
7. `Properties` section 按 `design-013` 的 `null` / `scalar` / `rich_text` / `list` / `object` semantic node 渲染，不重新解析 frontmatter Markdown
8. resolved frontmatter `wikilink` segment 在 sidebar 中可点击；unresolved segment 维持显式 unresolved 表达且不伪造 href
9. `Links` section 只依赖 `design-013` 当前已提供的 `target` / `kind` / `href` / `exists` 字段，不假设 alias 或 source attribution
10. desktop 采用正文 + 右侧 sidebar 的两栏布局，mobile / narrow viewport 下 sidebar 堆叠到正文下方
11. README、ARCHITECTURE 和最终 design doc 与实现一致，且 `design-014` 生命周期状态完成收口
12. 回归测试覆盖 route eligibility、TOC anchor behavior、sidebar rendering 和核心浏览器验收场景
13. `cargo test`、`cargo clippy -- -D warnings`、`cargo fmt --check` 通过
14. `specmate check` 通过

## Blocking Rules

执行过程中如果遇到以下情况，不要自行扩展语义，必须先回到本计划和设计文档对齐：

- 想让 `.base` route 也顺手支持 metadata mode，而不是先遵守 `design-013`
- 想让前端直接复用 docsify 当前 URL 作为 metadata fetch URL
- 想在前端重新解析 frontmatter wiki-link 或补造 backend 未返回的 link metadata
- 想把 sidebar 渲染成注入正文的 Markdown/HTML，而不是 shell page chrome
- 想顺手加入 search、Mermaid、backlinks、inline property edit 或全站导航重构
- 想让 README / ARCHITECTURE / design lifecycle 等文档收口延后到实现之外
