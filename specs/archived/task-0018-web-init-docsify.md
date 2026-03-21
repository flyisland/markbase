---
id: task-0018
title: "实现 web init-docsify 命令"
status: completed
design: design-012
boundaries:
  allowed:
    - "src/main.rs"
    - "src/web/**"
    - "README.md"
    - "ARCHITECTURE.md"
    - "docs/design-docs/implemented/design-012-docsify-frontend-integration.md"
    - "specs/archived/task-0018-web-init-docsify.md"
    - "tests/cli_web.rs"
    - "tests/common/**"
  forbidden_patterns:
    - "src/renderer/**"
    - "src/query/**"
    - "docs/design-docs/implemented/**"
completion_criteria:
  - id: "cc-001"
    scenario: "`web init-docsify` 需要显式 homepage"
    test: "test_web_init_docsify_requires_homepage"
  - id: "cc-002"
    scenario: "命令在 `<base-dir>/index.html` 生成 docsify shell"
    test: "test_web_init_docsify_writes_index_html_to_base_dir_root"
  - id: "cc-003"
    scenario: "已有 shell 时默认拒绝覆盖，`--force` 才允许重写"
    test: "test_web_init_docsify_refuses_overwrite_without_force"
  - id: "cc-004"
    scenario: "生成的 shell 使用指定 homepage canonical route"
    test: "test_web_init_docsify_embeds_configured_homepage"
  - id: "cc-005"
    scenario: "缺少 `index.html` 时 `web serve` 拒绝启动并提示初始化 docsify"
    test: "test_web_serve_requires_docsify_index_html"
  - id: "cc-006"
    scenario: "访问 `/` 根路径时自动返回生成的 index shell"
    test: "test_web_root_serves_generated_index_html"
  - id: "cc-007"
    scenario: "前端插件把 `.md` 和 `.base` 文档链接保留在 docsify shell 内导航"
    test: "test_web_init_docsify_plugin_rewrites_internal_document_links"
  - id: "cc-008"
    scenario: "前端插件不改写 binary resource URL"
    test: "test_web_init_docsify_plugin_leaves_binary_resource_urls_untouched"
  - id: "cc-009"
    scenario: "README 与 ARCHITECTURE 记录 docsify 初始化边界"
    test: "test_web_init_docsify_docs_match_behavior"
---

## Intent

落地 `design-012` 定义的首版 `markbase web init-docsify`，为现有
`markbase web serve` 提供一个受支持的 docsify 前端壳，而不改变
`design-003` 已锁定的后端 Markdown / resource HTTP 合同。

这个任务的重点是：

- 明确的 CLI surface
- 生成位置与覆盖行为
- docsify 壳内对内部文档链接与 binary resource 链接的分类处理
- 文档和回归测试

这个任务不负责扩展后端 link rewrite 语义，也不负责一次性实现所有
frontend plugin。

## Decisions

- 新增命令 `markbase web init-docsify --homepage <canonical-url> [--force]`
- `--homepage` 在首版中是必选项；命令不得猜测默认首页
- 生成位置固定为 `<base-dir>/index.html`
- 首版至少生成根目录 `index.html`
- 若目标 `index.html` 已存在，默认返回失败；只有 `--force` 才允许覆盖
- `web serve` 作为用户浏览入口，在缺少 `<base-dir>/index.html` 时必须拒绝启动
- 上述失败必须返回解释性信息，指导用户先执行 `markbase web init-docsify --homepage <canonical-url>`
- 当 `index.html` 已由 `web init-docsify` 生成后，请求 `/` 时应返回该 shell，而不是报告 canonical route miss
- docsify 壳通过现有 `markbase web serve` 提供内容，不引入单独 HTML app server
- 前端插件只重写内部文档导航链接，即指向 `.md` 或 `.base` 的 markbase canonical 路径
- 前端插件不得改写 binary resource URL；图片、PDF、附件应直接请求资源路径
- 插件逻辑应作用于 docsify app container 内渲染后的链接元素，而不是修改后端输出合同
- 首版可以继续使用 docsify CDN 资源，不要求 vendored assets
- 首版不要求 sidebar、search、Mermaid、callout CSS 等附加能力
- README 记录用户可见命令和打开方式；ARCHITECTURE 记录 docsify shell 属于 frontend integration layer

## Boundaries

### Allowed Changes

- src/main.rs
- src/web/**
- README.md
- ARCHITECTURE.md
- docs/design-docs/implemented/design-012-docsify-frontend-integration.md
- specs/archived/task-0018-web-init-docsify.md
- tests/cli_web.rs
- tests/common/**

### Forbidden

- 不得修改 `design-003` 已锁定的 backend href contract
- 不得让 `web init-docsify` 在未传 `--homepage` 时默默选择 `/README.md` 或其他猜测性默认值
- 不得在本任务中顺手加入新的后端 OFM normalization 语义
- 不得把 binary resource URL 一并 hash 化，导致图片和附件请求失效
- 不得把 callout styling、Mermaid、sidebar、search 作为首版完成前提

## Completion Criteria

场景: `web init-docsify` 需要显式 homepage
测试: test_web_init_docsify_requires_homepage
假设 用户执行 `markbase web init-docsify`
当   未传 `--homepage`
那么 命令返回参数错误
并且 不会生成任何 docsify shell 文件

场景: 命令在 `<base-dir>/index.html` 生成 docsify shell
测试: test_web_init_docsify_writes_index_html_to_base_dir_root
假设 用户执行 `markbase web init-docsify --homepage /HOME.md`
当   命令成功
那么 `<base-dir>/index.html` 被创建
并且 该文件可被现有 `web serve` 直接作为 `/index.html` 提供

场景: 已有 shell 时默认拒绝覆盖，`--force` 才允许重写
测试: test_web_init_docsify_refuses_overwrite_without_force
假设 `<base-dir>/index.html` 已存在
当   用户再次执行命令且未传 `--force`
那么 命令返回解释性失败
并且 原有文件内容保持不变
当   用户传入 `--force`
那么 允许覆盖并写入新内容

场景: 生成的 shell 使用指定 homepage canonical route
测试: test_web_init_docsify_embeds_configured_homepage
假设 用户执行 `markbase web init-docsify --homepage /All%20Opputunities%20Logs.base`
当   命令成功
那么 生成的 `index.html` 中 docsify 配置使用该 canonical route
并且 不会被替换成隐式默认首页

场景: 缺少 `index.html` 时 `web serve` 拒绝启动并提示初始化 docsify
测试: test_web_serve_requires_docsify_index_html
假设 `<base-dir>/index.html` 不存在
当   用户执行 `markbase web serve`
那么 命令启动失败
并且 返回解释性错误，提示先执行 `markbase web init-docsify --homepage <canonical-url>`

场景: 访问 `/` 根路径时自动返回生成的 index shell
测试: test_web_root_serves_generated_index_html
假设 `<base-dir>/index.html` 已由 `web init-docsify` 生成
当   浏览器请求 `http://127.0.0.1:3000/`
那么 server 返回该 `index.html` 的内容
并且 不会报 `ERROR: canonical URL '/' was not found in the indexed vault.`

场景: 前端插件把 `.md` 和 `.base` 文档链接保留在 docsify shell 内导航
测试: test_web_init_docsify_plugin_rewrites_internal_document_links
假设 backend Markdown 中出现指向 `/entities/person/alice.md` 或 `/All%20Logs.base` 的链接
当   docsify 壳内插件处理渲染后的链接
那么 这些文档链接被改写为 docsify shell 内导航形式
并且 点击后不会离开 `/index.html`

场景: 前端插件不改写 binary resource URL
测试: test_web_init_docsify_plugin_leaves_binary_resource_urls_untouched
假设 backend Markdown 中出现图片、PDF 或其他附件资源 URL
当   docsify 壳内插件处理渲染后的链接或图片元素
那么 binary resource URL 保持直接资源路径
并且 图片显示和附件下载不会因为 hash 路由改写而失效

场景: README 与 ARCHITECTURE 记录 docsify 初始化边界
测试: test_web_init_docsify_docs_match_behavior
假设 首版 `web init-docsify` 已实现
当   检查 README 与 ARCHITECTURE
那么 README 说明命令、生成位置和打开方式
并且 ARCHITECTURE 说明 docsify shell 属于 frontend integration，而不是 backend route contract
