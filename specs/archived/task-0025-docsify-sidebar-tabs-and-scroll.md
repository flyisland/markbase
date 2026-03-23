---
id: task-0025
title: "实现 docsify sidebar tabs、独立滚动与 link 路由适配"
status: completed
design: design-014
exec-plan: exec-007
phase: 1
boundaries:
  allowed:
    - "src/web/mod.rs"
    - "src/web/templates/docsify_shell.css"
    - "src/web/templates/docsify_sidebar.js"
    - "tests/cli_docsify.rs"
    - "tests/cli_web.rs"
    - "specs/archived/task-0025-docsify-sidebar-tabs-and-scroll.md"
  forbidden_patterns:
    - "specs/**"
    - "src/renderer/**"
    - "src/query/**"
    - "src/db.rs"
    - "src/template.rs"
    - "docs/design-docs/implemented/design-013-web-note-metadata-mode.md"
completion_criteria:
  - id: "cc-001"
    scenario: "sidebar 渲染为 tab strip + 单一 active panel，而不是 stacked sections"
    test: "test_docsify_sidebar_uses_tabbed_panel_contract"
  - id: "cc-002"
    scenario: "默认 active tab 为 `Properties`，并可切换到 `Links`"
    test: "test_docsify_sidebar_defaults_to_properties_tab"
  - id: "cc-003"
    scenario: "sidebar 自身拥有独立 scroll container，tab strip 保持可见"
    test: "test_docsify_sidebar_uses_independent_scroll_container"
  - id: "cc-004"
    scenario: "resolved note/base link 在 sidebar 中被适配为 docsify shell route"
    test: "test_docsify_sidebar_adapts_note_and_base_links_to_docsify_routes"
  - id: "cc-005"
    scenario: "resolved resource link 保持直接资源 URL，unresolved link 仍为非 clickable 表达"
    test: "test_docsify_sidebar_preserves_resource_and_unresolved_link_behavior"
  - id: "cc-006"
    scenario: "rich_text 中 resolved wikilink segment 同样走 docsify shell route"
    test: "test_docsify_sidebar_adapts_rich_text_wikilinks_to_docsify_routes"
  - id: "cc-007"
    scenario: "mobile / narrow viewport 下仍保持单一 active panel，而不是恢复 stacked sections"
    test: "test_docsify_sidebar_mobile_layout_preserves_tabbed_panels"
---

## Intent

把当前 sidebar 从“Properties 与 Links 同时纵向堆叠”改为当前 `design-014` 要求的 tabbed sidebar。

这个任务负责 DOM 结构、active-tab state、desktop/mobile layout、sidebar own scroll contract，以及 sidebar 内 note/base links 的 docsify route adaptation。

这个任务不负责 metadata fetch lifecycle，也不负责仓库级文档收口。

## Decisions

- `Properties` 与 `Links` 共享同一个 sidebar content slot，只显示一个 active panel
- tab strip 是 sidebar 的稳定顶层 chrome，不随 panel 内容滚出可视区域
- sidebar scroll 只作用于 sidebar 自身，不应迫使整页布局替代 sidebar panel 滚动
- 未来 tab（例如 `Backlinks`）应可复用现有 tab strip / active panel 结构
- sidebar 内 note/base href 必须适配成 docsify shell route；resource href 保持直接资源 URL
- 现有测试中“Properties 与 Links 同时显示”的假设需要被显式替换，而不是继续保留

## Boundaries

### Allowed Changes

- src/web/mod.rs
- src/web/templates/docsify_shell.css
- src/web/templates/docsify_sidebar.js
- tests/cli_docsify.rs
- tests/cli_web.rs
- specs/archived/task-0025-docsify-sidebar-tabs-and-scroll.md

### Forbidden

- 不得修改 metadata request lifecycle 或 route eligibility
- 不得修改 backend metadata route / JSON schema
- 不得引入新的 metadata field，例如 backlinks fetch
- 不得修改 `src/renderer/**`、`src/query/**`、`src/db.rs` 或 `src/template.rs`

## Completion Criteria

场景: sidebar 渲染为 tab strip + 单一 active panel，而不是 stacked sections
测试: test_docsify_sidebar_uses_tabbed_panel_contract
假设 当前 sidebar 已进入 ready state
当   shell 渲染 metadata sidebar
那么 sidebar 包含稳定 tab strip
并且 同一时刻只显示一个 active panel
并且 不再要求 `Properties` 与 `Links` 同时出现在 DOM 可见区

场景: 默认 active tab 为 `Properties`，并可切换到 `Links`
测试: test_docsify_sidebar_defaults_to_properties_tab
假设 当前 note 同时包含 properties 与 links metadata
当   sidebar 首次完成 ready render
那么 默认 active tab 为 `Properties`
并且 切换 tab 后 `Links` panel 成为当前可见 panel

场景: sidebar 自身拥有独立 scroll container，tab strip 保持可见
测试: test_docsify_sidebar_uses_independent_scroll_container
假设 `Properties` panel 内容长度超过 sidebar 可用高度
当   用户浏览 sidebar 内容
那么 sidebar content 区域可以独立滚动
并且 tab strip 保持在 sidebar 顶部可见
并且 长内容不会把其他 panel 变成页面下方的 stacked block

场景: resolved note/base link 在 sidebar 中被适配为 docsify shell route
测试: test_docsify_sidebar_adapts_note_and_base_links_to_docsify_routes
假设 backend metadata 返回 resolved note/base href，例如 `/entities/company/acme.md`
当   sidebar 生成 clickable link
那么 最终 href 为 docsify shell route，例如 `#/entities/company/acme.md`
并且 点击会留在 docsify app 内导航

场景: resolved resource link 保持直接资源 URL，unresolved link 仍为非 clickable 表达
测试: test_docsify_sidebar_preserves_resource_and_unresolved_link_behavior
假设 sidebar 同时包含 resolved resource link 与 unresolved link
当   sidebar 渲染 link row
那么 resource link 仍为直接资源 URL
并且 unresolved link 不会伪造 docsify hash route 或 backend href

场景: rich_text 中 resolved wikilink segment 同样走 docsify shell route
测试: test_docsify_sidebar_adapts_rich_text_wikilinks_to_docsify_routes
假设 property `rich_text` segment 中包含 resolved wikilink
当   sidebar 渲染该 value
那么 该 segment 的 clickable href 也被适配为 docsify shell route
并且 unresolved segment 继续保持非 clickable

场景: mobile / narrow viewport 下仍保持单一 active panel，而不是恢复 stacked sections
测试: test_docsify_sidebar_mobile_layout_preserves_tabbed_panels
假设 当前 viewport 进入 narrow/mobile 区间
当   shell 应用 sidebar layout
那么 sidebar 仍保持 tab strip + active panel 结构
并且 只改变整体堆叠位置，而不是把 tabbed panels 重新展开成纵向 sections
