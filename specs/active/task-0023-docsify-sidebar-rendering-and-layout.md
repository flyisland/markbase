---
id: task-0023
title: "实现 docsify sidebar 渲染与布局"
status: draft
design: design-014
exec-plan: exec-006
phase: 2
boundaries:
  allowed:
    - "src/web/mod.rs"
    - "src/web/templates/docsify_index.html"
    - "src/web/templates/docsify_shell.css"
    - "src/web/templates/docsify_sidebar.js"
    - "docs/design-docs/candidate/design-014-docsify-note-sidebar-ui.md"
    - "docs/design-docs/implemented/design-014-docsify-note-sidebar-ui.md"
    - "specs/active/task-0023-docsify-sidebar-rendering-and-layout.md"
    - "tests/cli_docsify.rs"
  forbidden_patterns:
    - "specs/**"
    - "src/renderer/**"
    - "src/query/**"
    - "src/db.rs"
    - "src/template.rs"
    - "docs/design-docs/implemented/design-013-web-note-metadata-mode.md"
completion_criteria:
  - id: "cc-001"
    scenario: "desktop note page 渲染正文 + 右侧 sidebar 的两栏布局"
    test: "test_web_init_docsify_sidebar_includes_desktop_two_column_layout_contract"
  - id: "cc-002"
    scenario: "narrow viewport 下 sidebar 堆叠到正文下方"
    test: "test_web_init_docsify_sidebar_includes_mobile_stack_layout_contract"
  - id: "cc-003"
    scenario: "`Properties` section 按 semantic value kinds 渲染而不重解析 frontmatter Markdown"
    test: "test_web_init_docsify_sidebar_renders_property_semantic_value_kinds"
  - id: "cc-004"
    scenario: "`rich_text` 中 resolved / unresolved wikilink 有区分明确的渲染"
    test: "test_web_init_docsify_sidebar_renders_rich_text_wikilink_segments"
  - id: "cc-005"
    scenario: "`Links` section 只消费 backend 已返回的 link fields"
    test: "test_web_init_docsify_sidebar_links_section_uses_current_metadata_contract_only"
  - id: "cc-006"
    scenario: "empty / loading / ready 状态有明确 DOM contract"
    test: "test_web_init_docsify_sidebar_includes_state_dom_contract"
  - id: "cc-007"
    scenario: "`design-014` 在 sidebar 合同实现后完成内容收口并迁移到 implemented"
    test: "doc review"
---

## Intent

落地 `design-014` 中与 sidebar 视觉输出相关的第二阶段实现。

这个任务负责把已经稳定的 metadata state 渲染为 `Properties` / `Links` UI，并给 note page 提供 responsive sidebar layout。

这个任务不负责改变 route eligibility 或后端 metadata contract。
实现收口后，本任务同时负责把 `design-014` 的最终实现合同折回正式设计文档并完成生命周期迁移。

## Decisions

- 正文仍是主区域；sidebar 是辅助阅读 chrome，不得喧宾夺主
- desktop 采用两栏布局；mobile / narrow viewport 下改为正文后堆叠
- `Properties` 渲染只消费 `design-013` 的 semantic value nodes；前端不得把 `raw` string 当 Markdown 再 parse
- `rich_text` segment 中 resolved `wikilink` 渲染为内部可点击链接；unresolved 维持非 clickable 且视觉上可区分
- `Links` section 只能依赖 `target`、`kind`、`href`、`exists`，不得假设 alias、source attribution 或附加 schema
- schema hints 属于次级信息，只允许轻量显示，不得把 sidebar 变成冗长的 schema inspector
- loading / empty / error / ready state 都需要稳定 DOM contract，便于测试与后续维护
- renderer / layout 资产应从 shell lifecycle 脚本中分离为独立模板资产；本任务负责其模板装配与样式合同
- 当 sidebar 合同已经落地时，`design-014` 不应继续停留在 candidate；本任务负责把最终合同写实并迁移到 implemented

## Boundaries

### Allowed Changes

- src/web/mod.rs
- src/web/templates/docsify_index.html
- src/web/templates/docsify_shell.css
- src/web/templates/docsify_sidebar.js
- docs/design-docs/candidate/design-014-docsify-note-sidebar-ui.md
- docs/design-docs/implemented/design-014-docsify-note-sidebar-ui.md
- specs/active/task-0023-docsify-sidebar-rendering-and-layout.md
- tests/cli_docsify.rs

### Forbidden

- 不得修改 metadata route eligibility 或 fetch URL contract
- 不得引入 frontmatter Markdown 二次解析
- 不得为 `Links` section 补造 backend 未提供的 alias / source-location metadata
- 不得顺手把 sidebar 扩展成 backlinks、search、edit UI 或全局导航系统
- 不得修改 `src/renderer/**`、`src/query/**`、`src/db.rs` 或 `src/template.rs`

## Completion Criteria

场景: desktop note page 渲染正文 + 右侧 sidebar 的两栏布局
测试: test_web_init_docsify_sidebar_includes_desktop_two_column_layout_contract
假设 当前 docsify shell 打开一个 eligible note route
当   页面进入 ready state
那么 note page DOM 和样式合同体现正文主列与右侧 sidebar 的两栏结构
并且 正文仍保持视觉主导

场景: narrow viewport 下 sidebar 堆叠到正文下方
测试: test_web_init_docsify_sidebar_includes_mobile_stack_layout_contract
假设 当前 viewport 宽度进入 narrow/mobile 区间
当   shell 应用 sidebar layout
那么 sidebar 堆叠到正文下方
并且 不要求额外 drawer / toggle 交互

场景: `Properties` section 按 semantic value kinds 渲染而不重解析 frontmatter Markdown
测试: test_web_init_docsify_sidebar_renders_property_semantic_value_kinds
假设 当前 metadata 同时包含 `null`、`scalar`、`rich_text`、`list` 与 `object`
当   shell 渲染 `Properties`
那么 每种 kind 都按前端合同渲染为对应 DOM
并且 实现不依赖重新解析 frontmatter Markdown string

场景: `rich_text` 中 resolved / unresolved wikilink 有区分明确的渲染
测试: test_web_init_docsify_sidebar_renders_rich_text_wikilink_segments
假设 `rich_text` segments 同时包含普通 text、resolved wikilink 与 unresolved wikilink
当   shell 渲染 property value
那么 resolved wikilink 为可点击内部链接
并且 unresolved wikilink 为非 clickable 表达
并且 两者不会只靠颜色一个信号来区分

场景: `Links` section 只消费 backend 已返回的 link fields
测试: test_web_init_docsify_sidebar_links_section_uses_current_metadata_contract_only
假设 `links` metadata 仅包含 `target`、`kind`、`href`、`exists`
当   shell 渲染 `Links` section
那么 row label 与行为只依赖这些字段
并且 不假设 alias、source attribution 或其他额外字段

场景: empty / loading / ready 状态有明确 DOM contract
测试: test_web_init_docsify_sidebar_includes_state_dom_contract
假设 当前 note route 分别处于 loading、ready、empty 与 error 情况
当   shell 渲染 sidebar
那么 每种状态都有稳定、可识别的 DOM 表达
并且 不会把 empty 与 error 混为同一种模糊状态

场景: `design-014` 在 sidebar 合同实现后完成内容收口并迁移到 implemented
测试: doc review
假设 sidebar route lifecycle、rendering 与 layout 合同都已实现
当   检查设计文档目录与 `design-014` 内容
那么 `design-014` 已迁移到 `docs/design-docs/implemented/`
并且 文档反映最终的 note-only sidebar、route eligibility、TOC anchor handling 与 rendering contract
