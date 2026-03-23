---
id: task-0027
title: "实现 docsify sidebar 路由与请求生命周期"
status: completed
design: design-014
exec-plan: exec-006
phase: 1
boundaries:
  allowed:
    - "src/web/templates/docsify_shell.js"
    - "tests/cli_web.rs"
    - "specs/archived/task-0027-docsify-sidebar-fetch-lifecycle.md"
  forbidden_patterns:
    - "specs/**"
    - "src/renderer/**"
    - "src/query/**"
    - "src/db.rs"
    - "src/template.rs"
    - "docs/design-docs/implemented/design-013-web-note-metadata-mode.md"
completion_criteria:
  - id: "cc-001"
    scenario: "sidebar 只在 canonical Markdown note route 上启用"
    test: "test_web_init_docsify_sidebar_only_targets_markdown_note_routes"
  - id: "cc-002"
    scenario: "metadata request 总是由 canonical note pathname 加固定 `fields` query 构造"
    test: "test_web_init_docsify_sidebar_metadata_request_uses_canonical_note_path_only"
  - id: "cc-003"
    scenario: "docsify `?id=...` TOC 锚点变化不会触发新的 metadata request"
    test: "test_web_init_docsify_sidebar_ignores_same_note_section_anchor_navigation"
  - id: "cc-004"
    scenario: "非 note route 不会进入 sidebar error state，而是直接隐藏/省略 sidebar"
    test: "test_web_init_docsify_sidebar_skips_unsupported_routes_without_metadata_errors"
  - id: "cc-005"
    scenario: "快速 route 切换时旧 metadata response 不会覆盖新 route 的 sidebar"
    test: "test_web_init_docsify_sidebar_prevents_stale_response_overwrite"
  - id: "cc-006"
    scenario: "metadata request 失败不会阻塞 note body 渲染"
    test: "test_web_init_docsify_sidebar_metadata_failure_does_not_block_note_body"
---

## Intent

落地 `design-014` 中与 docsify shell route identity 和 metadata fetch lifecycle 相关的第一阶段实现。

这个任务只负责前端 shell 何时显示 sidebar、何时发 request、request 发向哪里，以及 route change 期间如何避免旧请求污染新页面。

这个任务不负责 `Properties` / `Links` 的最终视觉布局，也不负责修改后端 metadata contract。

## Decisions

- sidebar 只对 canonical `.md` note route 生效；`.base`、`/` 与其他 shell route 不发 metadata request
- docsify route 的逻辑文档 identity 由 pathname 决定，不由 `?id=...` 这类页内锚点 query 决定
- metadata request 必须由 canonical note pathname 单独构造为 `/<file.path>.md?fields=properties,links`
- docsify 自己用于 TOC / section jump 的 query parameter 不得透传给后端 metadata mode
- 对 unsupported route，前端应直接省略/隐藏 sidebar，而不是故意请求后再把 `400` 渲染成 error state
- note body 渲染继续独立于 sidebar；metadata failure 只能影响 sidebar 自身
- stale-response protection 属于 shell lifecycle contract，不得留给视觉层临时兜底

## Boundaries

### Allowed Changes

- src/web/templates/docsify_shell.js
- tests/cli_web.rs
- specs/archived/task-0027-docsify-sidebar-fetch-lifecycle.md

### Forbidden

- 不得修改 `design-013` 已锁定的 metadata route / JSON schema
- 不得通过修改后端去容忍 docsify `?id=...` 被错误透传的请求
- 不得在本任务中实现完整 sidebar visual layout
- 不得修改 `src/renderer/**`、`src/query/**`、`src/db.rs` 或 `src/template.rs`
- 不得把 `.base` route 纳入 sidebar v1 适用范围

## Completion Criteria

场景: sidebar 只在 canonical Markdown note route 上启用
测试: test_web_init_docsify_sidebar_only_targets_markdown_note_routes
假设 docsify shell 当前可访问 `.md` note route、`.base` route 与 root route
当   前端根据当前 route 决定是否启用 sidebar
那么 只有 `.md` note route 启用 metadata sidebar
并且 `.base` route 与 root route 不发 metadata request

场景: metadata request 总是由 canonical note pathname 加固定 `fields` query 构造
测试: test_web_init_docsify_sidebar_metadata_request_uses_canonical_note_path_only
假设 当前 docsify route 为 `#/logs/example.md?id=heading-one`
当   前端构造 sidebar metadata request
那么 请求目标只包含 canonical note pathname 和 `?fields=properties,links`
并且 不会透传 `id=heading-one`

场景: docsify `?id=...` TOC 锚点变化不会触发新的 metadata request
测试: test_web_init_docsify_sidebar_ignores_same_note_section_anchor_navigation
假设 当前 note 已完成 sidebar metadata 加载
当   用户点击同一 note 左侧 TOC，导致 docsify route 仅改变 `?id=...`
那么 shell 将其视为页内导航
并且 当前 sidebar state 保持不变
并且 不会再发起新的 metadata request

场景: 非 note route 不会进入 sidebar error state，而是直接隐藏/省略 sidebar
测试: test_web_init_docsify_sidebar_skips_unsupported_routes_without_metadata_errors
假设 当前 docsify route 为 `.base` 或 root shell route
当   前端处理该 route
那么 sidebar 不会渲染 metadata error state
并且 不会尝试把 unsupported route 的 `400` 当作可恢复的 UI 错误

场景: 快速 route 切换时旧 metadata response 不会覆盖新 route 的 sidebar
测试: test_web_init_docsify_sidebar_prevents_stale_response_overwrite
假设 用户快速从 note A 导航到 note B
当   note A 的 metadata response 晚于 note B 返回
那么 shell 最终只显示 note B 的 sidebar
并且 note A 的晚到响应不会覆盖当前 state

场景: metadata request 失败不会阻塞 note body 渲染
测试: test_web_init_docsify_sidebar_metadata_failure_does_not_block_note_body
假设 note body 已可由 docsify 正常渲染
并且 当前 note 的 metadata request 失败
当   页面进入 error state
那么 note body 仍保持可读
并且 error state 被限制在 sidebar 区域
