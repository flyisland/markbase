---
id: task-0021
title: "实现动态 docsify entry HTML serving"
status: active
design: design-012-patch-02
boundaries:
  allowed:
    - "src/main.rs"
    - "src/web/**"
    - "README.md"
    - "ARCHITECTURE.md"
    - "docs/design-docs/draft/design-012-patch-02-dynamic-docsify-shell-serving.md"
    - "docs/design-docs/implemented/design-012-docsify-frontend-integration.md"
    - "specs/active/task-0021-dynamic-docsify-entry-html-serving.md"
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
    scenario: "`web serve` CLI surface 支持可选 `--homepage <canonical-url>`"
    test: "test_web_serve_command_parses_optional_homepage"
  - id: "cc-001"
    scenario: "`web serve` 在存在且版本匹配的导出 `index.html` 时继续直接使用该文件"
    test: "test_web_serve_uses_exported_entry_html_when_version_matches"
  - id: "cc-002"
    scenario: "导出 `index.html` 版本不匹配时，`web serve` 不再拒绝启动，而是回退到动态 docsify entry HTML"
    test: "test_web_serve_falls_back_to_dynamic_entry_html_when_exported_version_is_stale"
  - id: "cc-003"
    scenario: "缺少导出 `index.html` 时，`web serve --homepage <canonical-url>` 可直接动态提供浏览器入口"
    test: "test_web_serve_can_dynamically_serve_entry_html_without_exported_index"
  - id: "cc-004"
    scenario: "缺少可用导出 `index.html` 且也没有可用 homepage source 时，`web serve` 返回解释性失败"
    test: "test_web_serve_requires_homepage_source_when_no_usable_entry_html_exists"
  - id: "cc-005"
    scenario: "动态 docsify entry HTML 与 `web init-docsify` 对同一 homepage 生成的 `index.html` 完全一致"
    test: "test_web_dynamic_entry_html_matches_init_docsify_output_byte_for_byte"
  - id: "cc-006"
    scenario: "导出 `index.html` 与动态 docsify entry HTML 都携带可解析的 homepage metadata"
    test: "test_web_entry_html_embeds_homepage_metadata_for_runtime_reuse"
  - id: "cc-007"
    scenario: "当导出 `index.html` 存在但不再可直接使用时，`web serve` 明确规定 homepage source 优先级"
    test: "test_web_serve_dynamic_homepage_source_precedence_is_explicit"
  - id: "cc-008"
    scenario: "`web serve` 启动时为三种 entry HTML 模式输出清晰 INFO 信息"
    test: "test_web_serve_logs_clear_entry_html_mode_info"
  - id: "cc-009"
    scenario: "`/` 与 `/index.html` 在动态模式下都返回相同 docsify entry HTML"
    test: "test_web_dynamic_entry_html_serves_root_and_index_routes_consistently"
  - id: "cc-010"
    scenario: "动态 entry HTML 不回退既有 docsify 前端能力"
    test: "test_web_dynamic_entry_html_preserves_docsify_frontend_contract"
  - id: "cc-011"
    scenario: "`web init-docsify` 仍保留为显式导出命令，但文档定位变为导出/调试工具"
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

- `web serve` 的浏览器入口优先语义从“必须先有导出 `index.html`”改为“优先提供 docsify entry HTML；导出文件只是可复用输入之一”
- `web init-docsify` 继续保留，但在产品定位上主要用于显式导出、调试、对比最终 HTML，以及高级用户手工修改导出物
- `web serve` 与 `web init-docsify` 必须复用同一个 docsify entry HTML renderer；不得维护两套生成路径
- `web serve` CLI surface 新增可选 `--homepage <canonical-url>`，仅用于 dynamic mode 的 homepage source
- 对于同一个 homepage 与同一个当前二进制版本，动态返回的 entry HTML 与 `web init-docsify` 写出的 `index.html` 必须字节级一致
- `web init-docsify` 生成的 `index.html` 必须继续包含 `markbase` version / git metadata，并新增一个稳定、可解析的 homepage metadata marker，供 `web serve` 读取
- `web serve` 的 entry HTML source 优先级固定为：
  1. 存在且版本匹配的导出 `index.html`
  2. CLI `--homepage <canonical-url>` 提供的显式 homepage
  3. 存在但版本不匹配的导出 `index.html` 中嵌入的 homepage metadata
- 若存在且版本匹配的导出 `index.html`，`web serve` 直接使用该文件内容；即使同时传了 `--homepage` 也不做运行时重渲染，而是输出 INFO 说明导出文件优先
- 若导出 `index.html` 存在但版本不匹配，`web serve` 不得拒绝启动；它应忽略该旧文件内容，改用当前版本动态生成的 docsify entry HTML
- 若导出 `index.html` 不存在，且 CLI 传入 `--homepage`，`web serve` 直接动态生成 docsify entry HTML
- 若导出 `index.html` 不存在且未传 `--homepage`，`web serve` 返回解释性失败，提示用户传入 `--homepage` 或先执行 `web init-docsify`
- 若导出 `index.html` 存在但既版本不匹配又缺少可解析 homepage metadata，且 CLI 也未传 `--homepage`，`web serve` 返回解释性失败
- 动态模式下，请求 `/` 与 `/index.html` 必须返回相同 docsify entry HTML；不得让一个路径走动态返回、另一个路径走 404 或磁盘 miss
- `web get` 仍是 shell-independent inspection command；本任务不改变 `web get` 合同
- 动态 / 导出两种 entry HTML 模式都必须保留现有 docsify 前端合同：内部 `.md` / `.base` 导航、resource URL 直连、callout UI、版本 footer
- `web serve` 启动时至少要区分三类 INFO：
  1. 使用版本匹配的导出 `index.html`
  2. 因缺少导出 `index.html` 而动态生成 docsify entry HTML
  3. 因导出 `index.html` 版本不匹配而忽略旧文件并动态生成 docsify entry HTML

## Boundaries

### Allowed Changes

- src/main.rs
- src/web/**
- README.md
- ARCHITECTURE.md
- docs/design-docs/draft/design-012-patch-02-dynamic-docsify-shell-serving.md
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

场景: `web serve` CLI surface 支持可选 `--homepage <canonical-url>`
测试: test_web_serve_command_parses_optional_homepage
假设 用户执行 `markbase web serve --homepage /HOME.md`
当   CLI 解析参数
那么 命令接受该可选参数
并且 该参数不会破坏既有 `web serve` options 的解析合同

场景: `web serve` 在存在且版本匹配的导出 `index.html` 时继续直接使用该文件
测试: test_web_serve_uses_exported_entry_html_when_version_matches
假设 `<base-dir>/index.html` 已由当前版本 `web init-docsify` 导出
当   用户执行 `markbase web serve`
那么 `web serve` 继续使用该导出文件作为 docsify entry HTML
并且 不会因为 dynamic mode 存在而重渲染不同内容
当   用户执行 `markbase web serve --homepage /OTHER.md`
那么 `web serve` 仍继续使用该导出文件
并且 `--homepage` 不会覆盖版本匹配导出文件中已固定的 homepage

场景: 导出 `index.html` 版本不匹配时，`web serve` 不再拒绝启动，而是回退到动态 docsify entry HTML
测试: test_web_serve_falls_back_to_dynamic_entry_html_when_exported_version_is_stale
假设 `<base-dir>/index.html` 存在，但其中嵌入的 `markbase` version 与当前二进制不匹配
当   用户执行 `markbase web serve`
那么 命令不会因 version mismatch 失败
并且 会忽略旧文件内容并返回当前版本动态生成的 docsify entry HTML

场景: 缺少导出 `index.html` 时，`web serve --homepage <canonical-url>` 可直接动态提供浏览器入口
测试: test_web_serve_can_dynamically_serve_entry_html_without_exported_index
假设 `<base-dir>/index.html` 不存在
并且 用户执行 `markbase web serve --homepage /HOME.md`
当   浏览器请求 `/` 或 `/index.html`
那么 server 返回动态生成的 docsify entry HTML
并且 无需先执行 `web init-docsify`

场景: 缺少可用导出 `index.html` 且也没有可用 homepage source 时，`web serve` 返回解释性失败
测试: test_web_serve_requires_homepage_source_when_no_usable_entry_html_exists
假设 `<base-dir>/index.html` 不存在
并且 用户未传 `--homepage`
当   执行 `markbase web serve`
那么 命令启动失败
并且 stderr 明确提示需要传入 `--homepage` 或先执行 `markbase web init-docsify --homepage <canonical-url>`

场景: 动态 docsify entry HTML 与 `web init-docsify` 对同一 homepage 生成的 `index.html` 完全一致
测试: test_web_dynamic_entry_html_matches_init_docsify_output_byte_for_byte
假设 用户对同一个 base-dir 和同一个 homepage 既可运行 `web init-docsify`，也可触发 dynamic mode
当   对比导出的 `index.html` 与动态返回的 entry HTML
那么 两者内容字节级一致
并且 不允许仅“语义等价但文本不同”

场景: 导出 `index.html` 与动态 docsify entry HTML 都携带可解析的 homepage metadata
测试: test_web_entry_html_embeds_homepage_metadata_for_runtime_reuse
假设 当前版本可生成 docsify entry HTML
当   检查导出文件与动态返回结果
那么 两者都包含稳定的 homepage metadata marker
并且 `web serve` 可在 stale exported file fallback 时读取该 marker

场景: 当导出 `index.html` 存在但不再可直接使用时，`web serve` 明确规定 homepage source 优先级
测试: test_web_serve_dynamic_homepage_source_precedence_is_explicit
假设 同时存在 stale exported `index.html` 与 CLI `--homepage`
当   `web serve` 进入 dynamic mode
那么 CLI `--homepage` 优先于旧文件中嵌入的 homepage metadata
并且 若无 CLI `--homepage`，才回退读取旧文件中的 homepage metadata

场景: `web serve` 启动时为三种 entry HTML 模式输出清晰 INFO 信息
测试: test_web_serve_logs_clear_entry_html_mode_info
假设 用户分别遇到版本匹配导出文件、缺少导出文件、导出文件版本不匹配三种情况
当   执行 `markbase web serve`
那么 stderr 中都有清晰且可区分的 INFO 提示
并且 提示中明确说明当前使用的是 exported entry HTML 还是 dynamic entry HTML

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

场景: `web init-docsify` 仍保留为显式导出命令，但文档定位变为导出/调试工具
测试: doc review
假设 `design-012-patch-02` 已实现
当   检查 README、ARCHITECTURE 与 design docs
那么 文档明确说明 `web serve` dynamic entry HTML 是默认浏览器入口
并且 `web init-docsify` 的定位已降为导出/调试工具，而不是强前置安装步骤
