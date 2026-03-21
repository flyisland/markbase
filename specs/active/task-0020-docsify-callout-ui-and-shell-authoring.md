---
id: task-0020
title: "实现 docsify callout UI 与 shell 模板化维护"
status: active
design: design-012-patch-01
boundaries:
  allowed:
    - "src/web/**"
    - "README.md"
    - "ARCHITECTURE.md"
    - "docs/design-docs/implemented/design-012-docsify-frontend-integration.md"
    - "docs/design-docs/draft/design-012-patch-01-callout-ui-and-shell-authoring.md"
    - "docs/design-docs/obsolete/design-012-patch-01-callout-ui-and-shell-authoring.md"
    - "specs/active/task-0020-docsify-callout-ui-and-shell-authoring.md"
    - "tests/cli_web.rs"
    - "tests/common/**"
  forbidden_patterns:
    - "src/renderer/**"
    - "src/query/**"
    - "src/db.rs"
    - "src/template.rs"
    - "docs/design-docs/draft/design-014-docsify-note-sidebar-ui.md"
completion_criteria:
  - id: "cc-001"
    scenario: "`web init-docsify` 仍只生成单个 `index.html` 浏览器入口文件"
    test: "test_web_init_docsify_writes_single_file_shell_output"
  - id: "cc-002"
    scenario: "仓库内 docsify shell 不再以单个大型 Rust string 直接维护"
    test: "code review"
  - id: "cc-003"
    scenario: "生成的 shell 包含 callout DOM upgrade 逻辑"
    test: "test_web_init_docsify_includes_callout_upgrade_plugin"
  - id: "cc-004"
    scenario: "callout upgrade 支持普通 callout 与 foldable marker 识别"
    test: "test_web_init_docsify_callout_plugin_recognizes_foldable_markers"
  - id: "cc-005"
    scenario: "无 trailing title text 时使用稳定的默认标题"
    test: "test_web_init_docsify_callout_plugin_uses_stable_default_titles"
  - id: "cc-006"
    scenario: "foldable callout 使用原生 disclosure 语义而不是自定义状态机"
    test: "test_web_init_docsify_callout_plugin_uses_details_summary"
  - id: "cc-007"
    scenario: "nested callout 在 DOM upgrade 后仍保持正确层级与语义"
    test: "test_web_init_docsify_callout_plugin_preserves_nested_callouts"
  - id: "cc-008"
    scenario: "callout upgrade 只作用于 docsify 渲染后的 blockquote，不新增后端 callout rewrite"
    test: "test_web_init_docsify_callout_plugin_preserves_backend_markdown_contract"
  - id: "cc-009"
    scenario: "既有内部文档链接 hash 导航与 binary resource 直连行为保持不变"
    test: "test_web_init_docsify_callout_changes_do_not_regress_navigation_plugin"
  - id: "cc-010"
    scenario: "README 与 ARCHITECTURE 记录 callout UI 归属和单文件输出合同"
    test: "test_web_init_docsify_callout_docs_match_behavior"
  - id: "cc-011"
    scenario: "浏览器实际渲染后的 callout DOM 与 foldable 交互符合合同"
    test: "manual browser acceptance"
  - id: "cc-012"
    scenario: "实现完成后 patch 内容并回写到 `design-012`，patch 归档为 merged"
    test: "doc review"
---

## Intent

落地 `design-012-patch-01`，为已存在的 docsify shell 增加 Obsidian callout
UI 支持，并把 shell 的仓库内维护方式从单个大型 Rust string 收敛为更易维护
的模板/资产 authoring 方式，同时保持对用户输出仍是单个 `index.html`。

这个任务的重点是：

- docsify 侧 callout UI ownership
- `[!type]`、`[!type]+`、`[!type]-` marker 识别
- 基于渲染后 DOM 的 callout upgrade
- 单文件输出合同不变前提下的 shell 模板化维护
- 文档与回归测试

这个任务不负责后端 callout 语义扩展，也不负责 sidebar、search、Mermaid
或其他 docsify 前端能力。

## Decisions

- callout rendering 属于 docsify UI 责任，而不是后端 HTML generation 责任
- 后端继续只负责 preserving blockquote/callout container structure；本任务不得新增后端 callout rewrite
- docsify shell 在 Markdown 渲染完成后扫描 `blockquote` 并执行 callout DOM upgrade
- `[!type]` 表示非折叠 callout；`+` 表示默认展开；`-` 表示默认折叠
- 若 marker line 没有 trailing title text，则使用由 callout type 派生的稳定默认标题；同一 type 必须产生一致标题
- foldable callout 使用 `<details>` / `<summary>` 作为首选 DOM 语义
- 非 foldable callout 使用普通容器加 `data-callout` 等语义属性承载样式
- nested callout 必须可工作；实现应按 inside-out 顺序处理嵌套 blockquote
- `web init-docsify` 对用户仍然只生成 `<base-dir>/index.html`
- 仓库内可引入模板文件或静态片段来维护 shell HTML、JS、CSS，但最终产物必须内联到单个 `index.html`
- 既有 docsify 导航插件行为不得回退：`.md` / `.base` 继续留在 shell 内导航，binary resource URL 继续直接请求
- 浏览器级 callout 验收属于独立 acceptance 层，不与 unit/integration test 混用
- 本任务的浏览器级 callout 验收采用 manual browser acceptance；不要求在本任务中引入新的自动化浏览器测试基础设施
- 实现完成后，应将 patch 内容折回 `design-012`，并把 patch 移到 `docs/design-docs/obsolete/` 且标记 `obsolete:merged`

## Boundaries

### Allowed Changes

- src/web/**
- README.md
- ARCHITECTURE.md
- docs/design-docs/implemented/design-012-docsify-frontend-integration.md
- docs/design-docs/draft/design-012-patch-01-callout-ui-and-shell-authoring.md
- docs/design-docs/obsolete/design-012-patch-01-callout-ui-and-shell-authoring.md
- specs/active/task-0020-docsify-callout-ui-and-shell-authoring.md
- tests/cli_web.rs
- tests/common/**

### Forbidden

- 不得新增后端 callout HTML generation
- 不得修改 `design-003` 已锁定的 backend Markdown / resource contract
- 不得修改 `src/renderer/**`、`src/query/**`、`src/db.rs` 或 `src/template.rs`
- 不得把 docsify shell 输出扩展成默认多文件安装物
- 不得把 sidebar、search、Mermaid、metadata mode、theme redesign 作为本任务完成前提
- 不得要求前端重新解析 vault-aware note embeds 或 canonical route semantics

## Completion Criteria

场景: `web init-docsify` 仍只生成单个 `index.html` 浏览器入口文件
测试: test_web_init_docsify_writes_single_file_shell_output
假设 用户执行 `markbase web init-docsify --homepage /HOME.md`
当   命令成功
那么 `<base-dir>` 下新增的浏览器入口文件仍只有 `index.html`
并且 不要求用户再管理额外 JS 或 CSS 文件

场景: 仓库内 docsify shell 不再以单个大型 Rust string 直接维护
测试: code review
假设 docsify shell 已支持 callout UI
当   阅读实现
那么 shell HTML、JS 或 CSS 至少一部分来自仓库内模板或静态片段
并且 最终生成流程仍输出单个 `index.html`

场景: 生成的 shell 包含 callout DOM upgrade 逻辑
测试: test_web_init_docsify_includes_callout_upgrade_plugin
假设 用户已生成 docsify shell
当   检查生成的 `index.html`
那么 其中包含识别并升级 callout blockquote 的前端插件逻辑
并且 该逻辑属于 docsify shell，而不是后端 Markdown rewrite

场景: callout upgrade 支持普通 callout 与 foldable marker 识别
测试: test_web_init_docsify_callout_plugin_recognizes_foldable_markers
假设 shell 处理 docsify 渲染后的 callout blockquote
当   marker line 分别为 `[!info]`、`[!faq]+`、`[!faq]-`
那么 插件能区分非折叠、默认展开和默认折叠三种模式
并且 可提取 trailing title text 作为 callout title

场景: 无 trailing title text 时使用稳定的默认标题
测试: test_web_init_docsify_callout_plugin_uses_stable_default_titles
假设 shell 处理不带 trailing title text 的 `[!info]` 与 `[!faq]-` callout
当   插件生成对应 callout UI
那么 每个 callout 都有非空标题
并且 默认标题由 callout type 稳定派生
并且 同一 type 在不同 note 中产生一致的默认标题

场景: foldable callout 使用原生 disclosure 语义而不是自定义状态机
测试: test_web_init_docsify_callout_plugin_uses_details_summary
假设 shell 识别到 foldable callout
当   生成对应 DOM
那么 其结构使用 `<details>` 与 `<summary>`
并且 默认展开状态与 `+` / `-` marker 一致

场景: nested callout 在 DOM upgrade 后仍保持正确层级与语义
测试: test_web_init_docsify_callout_plugin_preserves_nested_callouts
假设 一个 callout body 内部还包含下一层 callout
当   插件执行 inside-out 的 callout DOM upgrade
那么 内层 callout 不会被外层升级过程破坏
并且 最终 DOM 中外层与内层 callout 都保留各自的 type、title 与 foldable 语义

场景: callout upgrade 只作用于 docsify 渲染后的 blockquote，不新增后端 callout rewrite
测试: test_web_init_docsify_callout_plugin_preserves_backend_markdown_contract
假设 backend 仍输出普通 Markdown callout marker
当   `web get` 或 `web serve` 返回 note Markdown
那么 backend response contract 不新增 callout-specific HTML 或 rewrite
并且 callout 语义增强只存在于 docsify shell 生成物中

场景: 既有内部文档链接 hash 导航与 binary resource 直连行为保持不变
测试: test_web_init_docsify_callout_changes_do_not_regress_navigation_plugin
假设 shell 已包含 callout upgrade 与既有导航插件
当   检查 `.md`、`.base` 与 binary resource URL 处理逻辑
那么 `.md` / `.base` 仍在 docsify shell 内导航
并且 binary resource URL 仍保持直接请求路径

场景: README 与 ARCHITECTURE 记录 callout UI 归属和单文件输出合同
测试: test_web_init_docsify_callout_docs_match_behavior
假设 docsify callout UI 已实现
当   检查 README 与 ARCHITECTURE
那么 README 说明 docsify shell 支持 callout UI，且用户侧仍是单文件 `index.html`
并且 ARCHITECTURE 说明 callout UI 属于 frontend integration layer，而不是 backend route contract

场景: 浏览器实际渲染后的 callout DOM 与 foldable 交互符合合同
测试: manual browser acceptance
假设 含有普通 callout、无 trailing title 的 callout、`[!faq]+`、`[!faq]-` 与 nested callout 的 note 可通过 docsify shell 打开
当   浏览器加载 `/index.html#/<canonical-note-route>` 并完成 docsify 渲染
那么 普通 callout 被升级为非折叠 callout UI 容器
并且 无 trailing title 的 callout 显示稳定的默认标题
并且 foldable callout 被升级为 `<details>` / `<summary>` 语义结构
并且 `+` 默认展开、`-` 默认折叠
并且 用户交互可以切换 foldable callout 的展开状态
并且 nested callout 在最终 DOM 中保持正确层级
并且 callout 内部文档链接与 binary resource URL 行为不回退

场景: 实现完成后 patch 内容并回写到 `design-012`，patch 归档为 merged
测试: doc review
假设 `design-012-patch-01` 已完全实现
当   检查设计文档目录
那么 `design-012` 已吸收 patch 的最终合同
并且 patch 文件移动到 `docs/design-docs/obsolete/`
并且 patch frontmatter 标记为 `obsolete:merged`
