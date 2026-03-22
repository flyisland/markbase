---
id: task-0021
title: "实现动态 docsify entry HTML serving"
status: completed
design: design-012-patch-02
boundaries:
  allowed:
    - "src/main.rs"
    - "src/web/**"
    - "README.md"
    - "ARCHITECTURE.md"
    - "docs/design-docs/obsolete/design-012-patch-02-dynamic-docsify-entry-html-serving.md"
    - "docs/design-docs/implemented/design-012-docsify-frontend-integration.md"
    - "specs/archived/task-0021-dynamic-docsify-entry-html-serving.md"
    - "tests/cli_web.rs"
    - "tests/common/**"
  forbidden_patterns:
    - "src/renderer/**"
    - "src/query/**"
    - "src/db.rs"
    - "src/template.rs"
    - "docs/design-docs/draft/design-014-docsify-note-sidebar-ui.md"
completion_criteria:
  - id: "cc-000"
    scenario: "`web serve` CLI surface 支持可选 `--homepage <homepage-ref>`"
    test: "test_web_serve_command_parses_optional_homepage"
  - id: "cc-001"
    scenario: "`web serve` 未传 `--homepage` 时，仅在存在且版本匹配的导出 `index.html` 下继续启动"
    test: "test_web_serve_uses_exported_entry_html_when_version_matches"
  - id: "cc-002"
    scenario: "`web serve` 未传 `--homepage` 且导出 `index.html` 不可用时，返回解释性失败"
    test: "test_web_serve_requires_usable_exported_entry_html_when_homepage_is_not_provided"
  - id: "cc-003"
    scenario: "传入 `--homepage` 时，`web serve` 始终进入 dynamic mode，并在存在导出 `index.html` 时明确告警忽略它"
    test: "test_web_serve_can_dynamically_serve_entry_html_without_exported_index"
  - id: "cc-004"
    scenario: "`--homepage` 可接受 note name、file.path、canonical URL，并统一解析为存在的 canonical URL"
    test: "test_web_homepage_input_resolves_to_existing_canonical_url"
  - id: "cc-005"
    scenario: "动态 docsify entry HTML 与 `web init-docsify` 对同一 homepage 生成的 `index.html` 完全一致"
    test: "test_web_dynamic_entry_html_matches_init_docsify_output_byte_for_byte"
  - id: "cc-006"
    scenario: "导出 `index.html` 与动态 docsify entry HTML 都携带可解析的 homepage metadata"
    test: "test_web_entry_html_embeds_homepage_metadata"
  - id: "cc-007"
    scenario: "`--homepage` 只允许最终解析到 `.md` 或 `.base` 目标"
    test: "test_web_homepage_input_rejects_non_document_targets"
  - id: "cc-008"
    scenario: "`web serve` 启动时为 static / dynamic-ignore-exported 两类模式输出清晰 INFO/WARN 信息"
    test: "test_web_serve_logs_clear_entry_html_mode_info"
  - id: "cc-009"
    scenario: "`/` 与 `/index.html` 在动态模式下都返回相同 docsify entry HTML"
    test: "test_web_dynamic_entry_html_serves_root_and_index_routes_consistently"
  - id: "cc-010"
    scenario: "动态 entry HTML 不回退既有 docsify 前端能力"
    test: "test_web_dynamic_entry_html_preserves_docsify_frontend_contract"
  - id: "cc-011"
    scenario: "`web init-docsify` 仍保留为显式导出命令，但 help / 文档定位变为非必须的导出/调试工具"
    test: "doc review"
---

## Intent

落地 `design-012-patch-02`，让 `markbase web serve` 在浏览器场景下优先负责
返回 docsify entry HTML，同时把 `markbase web init-docsify` 从强前置安装步骤
收敛为显式导出/调试工具。

这个任务的重点是：

- `web serve` 的 entry HTML source 选择逻辑
- 动态返回与导出文件的 single-source rendering contract
- homepage source contract
- 用户可见 INFO / error 行为
- 文档与回归测试

这个任务不负责 sidebar、search、Mermaid、metadata mode 或新的前端交互能力。

## Decisions

- `web serve` 的浏览器入口语义改为显式双模式：
  1. 未传 `--homepage` 时，只允许复用现有导出 `index.html`
  2. 传入 `--homepage` 时，始终动态生成 docsify entry HTML
- `web init-docsify` 继续保留，但在产品定位上主要用于显式导出、调试、对比最终 HTML，以及高级用户手工修改导出物
- `web serve` 与 `web init-docsify` 必须复用同一个 docsify entry HTML renderer；不得维护两套生成路径
- `web serve` CLI surface 新增可选 `--homepage <homepage-ref>`，仅用于 dynamic mode 的 homepage source
- `web init-docsify --homepage` 与 `web serve --homepage` 都接受三种输入：
  1. note name
  2. vault-relative `file.path`
  3. canonical URL
- homepage 输入必须先解析到真实存在的 `.md` 或 `.base` 目标，再统一 canonicalize 为 `/<file.path>`；普通资源文件不得作为 homepage
- 对于同一个 homepage 与同一个当前二进制版本，动态返回的 entry HTML 与 `web init-docsify` 写出的 `index.html` 必须字节级一致
- `web init-docsify` 生成的 `index.html` 必须继续包含 `markbase` version / git metadata，并继续包含稳定、可解析的 homepage metadata marker
- 若未传 `--homepage`，`web serve` 仅尝试使用现有导出 `index.html`；若该文件不存在、缺少版本 marker、或版本不匹配，则命令必须报错退出
- 若传入 `--homepage`，`web serve` 必须直接动态生成 docsify entry HTML；若 `<base-dir>/index.html` 存在，则输出 `WARN` 说明发现了该文件但本次不会使用它
- 动态模式下，请求 `/` 与 `/index.html` 必须返回相同 docsify entry HTML；不得让一个路径走动态返回、另一个路径走 404 或磁盘 miss
- `web get` 仍是 docsify-entry-HTML-independent inspection command；本任务不改变 `web get` 合同
- 动态 / 导出两种 entry HTML 模式都必须保留现有 docsify 前端合同：内部 `.md` / `.base` 导航、resource URL 直连、callout UI、版本 footer
- `web init-docsify` help 与 README 必须强调：它不是浏览器使用前的必需步骤，主要面向导出/调试/高级用户修改导出物
- `web serve` 启动时至少要区分两类日志：
  1. `INFO` 使用版本匹配的导出 `index.html`
  2. `INFO` 动态生成 docsify entry HTML；若发现现有导出文件则额外输出 `WARN` 说明忽略原因

## Boundaries

### Allowed Changes

- src/main.rs
- src/web/**
- README.md
- ARCHITECTURE.md
- docs/design-docs/obsolete/design-012-patch-02-dynamic-docsify-entry-html-serving.md
- docs/design-docs/implemented/design-012-docsify-frontend-integration.md
- specs/active/task-0021-dynamic-docsify-entry-html-serving.md
- tests/cli_web.rs
- tests/common/**

### Forbidden

- 不得创建第二套与 `web init-docsify` 脱钩的动态 entry HTML renderer
- 不得让动态 entry HTML 与导出 `index.html` 在相同输入下出现 byte drift
- 不得把 docsify entry HTML 动态返回扩展成服务端渲染 note HTML
- 不得修改现有 note / `.base` / resource backend body contract
- 不得把 sidebar、search、Mermaid 或 metadata mode 作为本任务完成前提
- 不得要求用户在动态模式下手工管理额外 JS/CSS 资产

## Completion Criteria

场景: `web serve` CLI surface 支持可选 `--homepage <homepage-ref>`
测试: test_web_serve_command_parses_optional_homepage
假设 用户执行 `markbase web serve --homepage HOME`
当   CLI 解析参数
那么 命令接受该可选参数
并且 该参数不会破坏既有 `web serve` options 的解析合同

场景: `web serve` 未传 `--homepage` 时，仅在存在且版本匹配的导出 `index.html` 下继续启动
测试: test_web_serve_uses_exported_entry_html_when_version_matches
假设 `<base-dir>/index.html` 已由当前版本 `web init-docsify` 导出
当   用户执行 `markbase web serve`
那么 `web serve` 继续使用该导出文件作为 docsify entry HTML
并且 不会在运行时重渲染不同内容

场景: `web serve` 未传 `--homepage` 且导出 `index.html` 不可用时，返回解释性失败
测试: test_web_serve_requires_usable_exported_entry_html_when_homepage_is_not_provided
假设 用户执行 `markbase web serve`
并且 `<base-dir>/index.html` 缺失、缺少版本 marker、或版本不匹配
当   命令尝试启动浏览器入口
那么 命令失败退出
并且 stderr 明确说明：未传 `--homepage` 时只允许使用现有导出 `index.html`

场景: 传入 `--homepage` 时，`web serve` 始终进入 dynamic mode，并在存在导出 `index.html` 时明确告警忽略它
测试: test_web_serve_can_dynamically_serve_entry_html_without_exported_index
假设 用户执行 `markbase web serve --homepage HOME`
当   浏览器请求 `/` 或 `/index.html`
那么 server 返回动态生成的 docsify entry HTML
并且 无需先执行 `web init-docsify`
当   `<base-dir>/index.html` 已存在
那么 stderr 输出 `WARN` 说明发现该文件但因为提供了 `--homepage` 所以不会使用它

场景: `--homepage` 可接受 note name、file.path、canonical URL，并统一解析为存在的 canonical URL
测试: test_web_homepage_input_resolves_to_existing_canonical_url
假设 用户分别传入 `HOME`、`areas/home/HOME.md`、`/areas/home/HOME.md`
当   `web serve --homepage ...` 或 `web init-docsify --homepage ...` 解析输入
那么 三者都被解析到同一个真实存在的 canonical URL
并且 最终写入 / 动态返回的 docsify entry HTML 中只出现 canonical URL 形式

场景: 动态 docsify entry HTML 与 `web init-docsify` 对同一 homepage 生成的 `index.html` 完全一致
测试: test_web_dynamic_entry_html_matches_init_docsify_output_byte_for_byte
假设 用户对同一个 base-dir 和同一个 homepage 既可运行 `web init-docsify`，也可触发 dynamic mode
当   对比导出的 `index.html` 与动态返回的 entry HTML
那么 两者内容字节级一致
并且 不允许仅“语义等价但文本不同”

场景: 导出 `index.html` 与动态 docsify entry HTML 都携带可解析的 homepage metadata
测试: test_web_entry_html_embeds_homepage_metadata
假设 当前版本可生成 docsify entry HTML
当   检查导出文件与动态返回结果
那么 两者都包含稳定的 homepage metadata marker

场景: `--homepage` 只允许最终解析到 `.md` 或 `.base` 目标
测试: test_web_homepage_input_rejects_non_document_targets
假设 用户传入指向图片、PDF 或其他非 `.md` / `.base` 资源的 note name、file.path、或 canonical URL
当   `web serve --homepage ...` 或 `web init-docsify --homepage ...` 解析输入
那么 命令失败退出
并且 stderr 说明 homepage 仅支持 `.md` / `.base`

场景: `web serve` 启动时为 static / dynamic-ignore-exported 两类模式输出清晰 INFO/WARN 信息
测试: test_web_serve_logs_clear_entry_html_mode_info
假设 用户分别遇到版本匹配导出文件、以及传入 `--homepage` 且磁盘已存在导出文件两种情况
当   执行 `markbase web serve`
那么 stderr 中都有清晰且可区分的 `INFO` / `WARN` 提示
并且 提示中明确说明当前使用的是 exported entry HTML 还是 dynamic entry HTML
并且 dynamic 情况下若发现导出文件存在，会明确说明“存在但忽略”

场景: `/` 与 `/index.html` 在动态模式下都返回相同 docsify entry HTML
测试: test_web_dynamic_entry_html_serves_root_and_index_routes_consistently
假设 `web serve` 当前运行在 dynamic mode
当   浏览器分别请求 `/` 与 `/index.html`
那么 两个路径返回的 body 相同
并且 `Content-Type` 均为 `text/html; charset=utf-8`

场景: 动态 entry HTML 不回退既有 docsify 前端能力
测试: test_web_dynamic_entry_html_preserves_docsify_frontend_contract
假设 当前版本 docsify entry HTML 已支持内部文档 hash 导航、resource URL 直连、callout UI 与版本 footer
当   `web serve` 使用 dynamic mode 返回 entry HTML
那么 这些前端合同保持不变
并且 不会因为切到 dynamic mode 而退化为旧版模板或缺少插件逻辑

场景: `web init-docsify` 仍保留为显式导出命令，但 help / 文档定位变为非必须的导出/调试工具
测试: doc review
假设 `design-012-patch-02` 已实现
当   检查 README、ARCHITECTURE、design docs 与 CLI help
那么 文档明确说明 `web serve` dynamic entry HTML 是默认浏览器入口
并且 `web init-docsify` 的定位已降为非必须的导出/调试工具，而不是强前置安装步骤
