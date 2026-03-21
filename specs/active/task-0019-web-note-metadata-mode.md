---
id: task-0019
title: "实现 web note metadata mode"
status: active
design: design-013
boundaries:
  allowed:
    - "src/web/**"
    - "src/db.rs"
    - "src/template.rs"
    - "README.md"
    - "ARCHITECTURE.md"
    - "docs/design-docs/draft/design-013-web-note-metadata-sidebar.md"
    - "specs/active/task-0019-web-note-metadata-mode.md"
    - "tests/cli_web.rs"
    - "tests/common/**"
  forbidden_patterns:
    - "src/renderer/**"
    - "src/query/**"
    - "docs/design-docs/draft/design-014-docsify-note-sidebar-ui.md"
completion_criteria:
  - id: "cc-001"
    scenario: "canonical Markdown note route 在无 `fields` 时继续返回现有 Markdown"
    test: "test_web_note_route_without_fields_returns_markdown_body"
  - id: "cc-002"
    scenario: "`fields` 只对 canonical Markdown note route 开启 metadata JSON mode"
    test: "test_web_note_fields_mode_only_supports_markdown_note_routes"
  - id: "cc-003"
    scenario: "`fields` 支持 `properties`、`links` 与它们的组合，并拒绝未知字段或未知 query 参数"
    test: "test_web_note_fields_mode_validates_requested_fields_and_query_params"
  - id: "cc-004"
    scenario: "metadata mode 的 response envelope 固定返回 `file`，并只返回请求过的 top-level fields"
    test: "test_web_note_metadata_mode_returns_expected_response_envelope"
  - id: "cc-005"
    scenario: "`properties` 返回 ordered property list 与 semantic value nodes"
    test: "test_web_note_metadata_properties_returns_ordered_semantic_fields"
  - id: "cc-006"
    scenario: "frontmatter string 中的 wiki-link 在 metadata mode 下被解析为可点击语义片段"
    test: "test_web_note_metadata_properties_resolves_frontmatter_wikilinks"
  - id: "cc-007"
    scenario: "template-backed note 的 property metadata 带有 template-aware schema enrichment，并在多 template 场景按 `templates` 顺序取第一个命中的 schema 定义"
    test: "test_web_note_metadata_properties_includes_template_schema_enrichment"
  - id: "cc-008"
    scenario: "`links` 返回 resolved outgoing link targets，并对 unresolved target 显式标记"
    test: "test_web_note_metadata_links_returns_resolved_and_unresolved_targets"
  - id: "cc-009"
    scenario: "`web get` 与 HTTP route 在 `fields` mode 下返回相同 JSON contract"
    test: "test_web_get_matches_web_serve_for_note_metadata_mode"
---

## Intent

落地 `design-013` 定义的 web note metadata mode，让 canonical Markdown
note route 在保留现有 Markdown body contract 的同时，支持通过 `fields`
query parameter 返回可单独测试的 metadata JSON。

这个任务只负责后端 contract 与测试，重点是：

- route mode 切换
- `properties` 与 `links` JSON shape
- frontmatter wiki-link semantic resolution
- template-aware property enrichment

这个任务不负责 docsify sidebar UI，也不负责把 metadata 渲染进 note body。

## Decisions

- canonical Markdown note route 仍然是唯一的 note web identity；不新增 `/api` 路由体系
- 无 `fields` query parameter 时，`web serve` / `web get` 对 `.md` route 的行为保持为现有 Markdown output
- `fields` 的出现会把 `.md` canonical route 切换到 metadata JSON mode
- v1 仅支持 `properties` 与 `links` 两个 field name，field name 大小写敏感
- `fields` 可按逗号组合请求多个字段；重复字段去重后处理
- `fields` 必须是非空、逗号分隔且无空项的 field list；空字符串、首尾逗号、重复逗号与带空白包裹的 field token 都属于 malformed syntax
- v1 对未知 field name 与未知 query parameter 返回 `400 Bad Request`，不做 silent ignore
- v1 对 malformed `fields` syntax 返回 `400 Bad Request`，不做自动 trim、补全或容错解析
- metadata mode 仅支持 canonical Markdown note route；`.base` route 与 binary resource route 使用 `fields` 时返回 `400 Bad Request`
- metadata mode 的 response envelope 始终包含 `file` object；除 `file` 外，仅返回被请求的 top-level fields，未请求字段必须省略而不是返回 `null`
- metadata JSON 是 semantic data，不是预渲染的 docsify UI fragment
- `properties` 采用 ordered field list，而不是 JSON object map，避免把展示顺序绑定到 object key order
- property string value 中的 wiki-link 必须复用 `src/link_syntax.rs` 的 `FrontmatterString` contract 解析，不允许前端或 web 模块发明第二套 frontmatter link parser
- resolved frontmatter wiki-link 必须返回 canonical path-based href；unresolved wikilink 必须保留 unresolved state，不得猜测性生成 href
- `properties` 的 schema enrichment 复用 `design-006` 的 template semantics；该 enrichment 只用于 inspection / presentation，不引入 browser-side edit contract
- 当多个 template 对同一 property 提供 schema 定义时，v1 按 note `templates` frontmatter 数组顺序取第一个命中的 schema 定义；不得做隐式 merge
- `links` 在 v1 返回 resolved outgoing target list，可先按 normalized target 去重，不要求 source-location-aware link instances
- `web get <canonical-url-with-fields>` 与 `web serve` 对同一路径的 metadata mode 必须共享同一核心实现路径
- docsify sidebar UI 属于 `design-014` 范围，不是本任务完成前提

## Boundaries

### Allowed Changes

- src/web/**
- src/db.rs
- src/template.rs
- README.md
- ARCHITECTURE.md
- docs/design-docs/draft/design-013-web-note-metadata-sidebar.md
- specs/active/task-0019-web-note-metadata-mode.md
- tests/cli_web.rs
- tests/common/**

### Forbidden

- 不得在本任务中实现 docsify sidebar layout、样式或 DOM 交互
- 不得修改现有 `/<file.path>.md` 无 query parameter 时的 Markdown route contract
- 不得新增第二套 `/api/...` note metadata route
- 不得让 metadata mode 依赖前端重写 frontmatter wiki-link 语义
- 不得在本任务中加入 `backlinks`
- 不得把 `properties` 预渲染成 HTML 或 Markdown sidebar fragment
- 不得顺手扩展 renderer 行为或 query subsystem 行为

## Completion Criteria

场景: canonical Markdown note route 在无 `fields` 时继续返回现有 Markdown
测试: test_web_note_route_without_fields_returns_markdown_body
假设 一个 canonical Markdown note route 当前已能返回 translated Markdown
当   请求该 route 且不带 `fields`
那么 响应仍为现有 Markdown body
并且 不会切换到 metadata JSON mode

场景: `fields` 只对 canonical Markdown note route 开启 metadata JSON mode
测试: test_web_note_fields_mode_only_supports_markdown_note_routes
假设 分别请求 Markdown note route、`.base` route 与 binary resource route，并都附带 `fields`
当   系统处理这些请求
那么 只有 Markdown note route 进入 metadata JSON mode
并且 `.base` route 与 binary resource route 返回 `400 Bad Request`

场景: `fields` 支持 `properties`、`links` 与它们的组合，并拒绝未知字段或未知 query 参数
测试: test_web_note_fields_mode_validates_requested_fields_and_query_params
假设 用户请求 `?fields=properties`、`?fields=links`、`?fields=properties,links`
当   系统处理这些请求
那么 每种合法组合都返回 `application/json; charset=utf-8`
并且 未知 field name 返回 `400 Bad Request`
并且 未知 query parameter 也返回 `400 Bad Request`
并且 `?fields=`、`?fields=properties,`、`?fields=properties,,links` 与 `?fields= properties ` 这类 malformed syntax 也返回 `400 Bad Request`

场景: metadata mode 的 response envelope 固定返回 `file`，并只返回请求过的 top-level fields
测试: test_web_note_metadata_mode_returns_expected_response_envelope
假设 用户分别请求 `?fields=properties`、`?fields=links` 与 `?fields=properties,links`
当   系统处理这些请求
那么 响应 JSON 始终包含 `file`
并且 `file` 至少包含当前 note 的 canonical `path` 与内部 `name`
并且 除 `file` 外只返回被请求的 top-level fields
并且 未请求字段被省略而不是返回 `null`

场景: `properties` 返回 ordered property list 与 semantic value nodes
测试: test_web_note_metadata_properties_returns_ordered_semantic_fields
假设 一个 note 含有多个 frontmatter fields，并且包含 scalar、list、object 与 null 值
当   请求 `?fields=properties`
那么 `properties.fields` 返回稳定顺序的 field list
并且 每个 field 使用 design 约定的 semantic value node，而不是扁平字符串化结果

场景: frontmatter string 中的 wiki-link 在 metadata mode 下被解析为可点击语义片段
测试: test_web_note_metadata_properties_resolves_frontmatter_wikilinks
假设 frontmatter string、list item 或 nested object string 中含有 `[[note]]` 或 `[[note|Alias]]`
当   请求 `?fields=properties`
那么 返回值中的对应 string 被表示为 `rich_text` semantic node
并且 resolved wikilink 带 canonical href
并且 unresolved wikilink 保持 unresolved state 而非猜测性链接

场景: template-backed note 的 property metadata 带有 template-aware schema enrichment
测试: test_web_note_metadata_properties_includes_template_schema_enrichment
假设 note 通过 `templates` frontmatter 关联到一个或多个 template，且 template schema 为某些 fields 定义了 `required`、`type`、`format`、`target` 或 `description`
当   请求 `?fields=properties`
那么 对应 property field 含有来自 template semantics 的 schema enrichment
并且 这些 enrichment 不改变原始 property value 的语义内容
并且 当多个 template 对同一 field 都有定义时，系统按 `templates` 数组顺序取第一个命中的 schema 定义

场景: `links` 返回 resolved outgoing link targets，并对 unresolved target 显式标记
测试: test_web_note_metadata_links_returns_resolved_and_unresolved_targets
假设 当前 note 的 outgoing links 同时包含已解析 note、resource、`.base` target 与 unresolved target
当   请求 `?fields=links`
那么 返回的 `links` 列表对每个 target 给出 `kind`、`exists` 与 resolved canonical href（若可解析）
并且 unresolved target 没有 fabricated href

场景: `web get` 与 HTTP route 在 `fields` mode 下返回相同 JSON contract
测试: test_web_get_matches_web_serve_for_note_metadata_mode
假设 某 canonical Markdown note route 可通过 metadata mode 返回 JSON
当   分别执行 `markbase web get '<canonical-url>?fields=properties,links'` 与 HTTP 请求同一路径
那么 两者返回的 JSON contract 一致
并且 共用同一核心 metadata implementation path
