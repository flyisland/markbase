---
id: task-0022
title: "修正 docsify callout body 的块级结构保留"
status: active
design: design-012
boundaries:
  allowed:
    - "src/web/**"
    - "tests/cli_web.rs"
    - "tests/common.rs"
    - "README.md"
    - "ARCHITECTURE.md"
    - "docs/design-docs/implemented/design-012-docsify-frontend-integration.md"
    - "specs/active/task-0022-docsify-callout-multiline-body.md"
  forbidden_patterns:
    - "specs/**"
    - "src/renderer/**"
    - "src/query/**"
    - "src/db.rs"
    - "src/template.rs"
    - "docs/design-docs/draft/**"
completion_criteria:
  - id: "cc-001"
    scenario: "docsify callout upgrade 不会把多行 body 内容压平到单行"
    test: "test_web_init_docsify_callout_plugin_preserves_multiline_body_structure"
  - id: "cc-002"
    scenario: "包含 inline code 的多段 callout body 在 upgrade 后仍保留逐行展示"
    test: "test_web_init_docsify_callout_plugin_preserves_line_breaks_around_inline_code"
  - id: "cc-003"
    scenario: "callout body 内的列表结构在 upgrade 后仍保持为块级列表"
    test: "test_web_init_docsify_callout_plugin_preserves_list_structure"
  - id: "cc-004"
    scenario: "backend `web get` / `web serve` 返回的 Markdown contract 保持不变"
    test: "test_web_init_docsify_callout_multiline_fix_preserves_backend_markdown_contract"
  - id: "cc-005"
    scenario: "既有 foldable、icon、nested callout 与 docsify 导航行为不回退"
    test: "test_web_init_docsify_callout_multiline_fix_does_not_regress_existing_frontend_contract"
  - id: "cc-006"
    scenario: "浏览器中 repo-owned multiline callout fixture 的段落与子项按多行显示"
    test: "manual browser acceptance"
---

## Intent

修正当前 docsify shell 的 callout DOM upgrade 在处理多行 body 内容时会压平块级结构的问题。

当前已确认的回归表现是：

- 后端返回的原始 Markdown 仍保持逐行 callout 内容
- Obsidian 中显示正常
- docsify 浏览器页面中，callout body 被挤成连续的一行或一串内联片段

这个任务的目标是让 docsify callout UI 与现有前端合同重新对齐：callout body 必须保留 Markdown 渲染后的块级结构，而不是在 upgrade 过程中把多个段落、列表项或带 inline code 的行压平成单个内联容器。

这个任务不负责扩展后端 callout 语义，也不负责 redesign docsify shell 的整体样式或交互。

## Decisions

- 该问题属于 docsify frontend integration 范围，修复点在 `src/web/**` 生成的 shell 资产，而不是后端 Markdown 输出层
- backend 继续返回普通 Markdown callout marker；不得新增 callout-specific HTML rewrite
- callout upgrade 必须以“保留 docsify/marked 已生成的块级 DOM 结构”为前提，不能为了包装 callout UI 而把 body child nodes 重新串接成单一段落
- 对于多行普通文本、含 inline code 的逐行说明、列表、以及混合段落内容，最终 callout body 都必须保持可读的逐行/逐块布局
- foldable callout 的 `<details>` / `<summary>` 语义、默认展开状态、title icon、nested callout 处理顺序与既有内部 `.md` / `.base` 导航行为必须保持不变
- 本任务优先补充可重复的回归测试；不得仅依赖一次性的人工观察来防止回归
- automated tests 不得依赖外部 vault、个人笔记路径或仓库外文件；回归样例必须由 repo 内测试代码使用 `TestVault` 或等价仓库内 fixture 自行构造
- 为避免“只断言 shell 中存在某段 JS 字符串”却遗漏真实回归，自动化验证必须锚定可重复的 multiline callout fixture 与明确的 DOM-upgrade 逻辑合同；若现有 harness 不能直接跑浏览器 DOM，则测试至少要验证 body node transfer / container preservation 的实现合同，而不是仅验证插件被注入
- `逐际动力` 笔记中的 `Overwrite` callout 只是最初发现问题的真实案例，可作为补充人工对照，但不能作为任务完成所必需的唯一验收样例

## Boundaries

### Allowed Changes

- src/web/**
- tests/cli_web.rs
- tests/common.rs
- README.md
- ARCHITECTURE.md
- docs/design-docs/implemented/design-012-docsify-frontend-integration.md
- specs/active/task-0022-docsify-callout-multiline-body.md

### Forbidden

- 不得通过修改 vault note 内容来规避前端 bug
- 不得把任何自动化测试建立在仓库外部文件、个人 vault 路径或手工准备的数据之上
- 不得把 callout body “换行修复” 下沉到 `web get` / `web serve` 的 backend Markdown rewrite
- 不得修改 `src/renderer/**`、`src/query/**`、`src/db.rs` 或 `src/template.rs`
- 不得回退既有 callout foldable、icon、nested callout 或 docsify 导航能力
- 不得把问题扩大成新的 sidebar、search、Mermaid 或 theme 重构任务

## Completion Criteria

场景: docsify callout upgrade 不会把多行 body 内容压平到单行
测试: test_web_init_docsify_callout_plugin_preserves_multiline_body_structure
假设 repo 内测试通过 `TestVault` 构造一个含有多行 callout body 的 note，且这些行在 Markdown 中分别占据独立逻辑行
当   docsify shell 完成 callout DOM upgrade
那么 最终 callout body 仍以多行或多块形式展示
并且 不会出现多个逻辑行被压成连续单行文本的回归
并且 该验证不依赖仓库外部笔记文件

场景: 包含 inline code 的多段 callout body 在 upgrade 后仍保留逐行展示
测试: test_web_init_docsify_callout_plugin_preserves_line_breaks_around_inline_code
假设 repo 内测试 fixture 的 callout body 含有类似 `` `- 官网` ``、`` `  - 官方首页：<官方首页 URL>` `` 这类带 inline code 的多行说明
当   docsify shell 升级该 callout
那么 每一条说明仍作为独立可读行展示
并且 inline code 样式不会触发整段 body 被错误压平
并且 fixture 由测试代码在仓库内构造

场景: callout body 内的列表结构在 upgrade 后仍保持为块级列表
测试: test_web_init_docsify_callout_plugin_preserves_list_structure
假设 repo 内测试 fixture 的 callout body 中存在 Markdown 列表或列表样式的多项结构
当   docsify shell 升级该 callout
那么 列表容器和列表项在最终 DOM 中仍保持块级结构
并且 不会被重新拼接为单个段落中的连续文本
并且 自动化验证锚定该 fixture 的结构保留合同，而不是只检查插件注入字符串

场景: backend `web get` / `web serve` 返回的 Markdown contract 保持不变
测试: test_web_init_docsify_callout_multiline_fix_preserves_backend_markdown_contract
假设 `TestVault` 构造了一个含有会触发该回归的 multiline callout fixture note
当   执行 `markbase web get <canonical-note-url>`
那么 stdout 仍返回普通 Markdown callout marker 与逐行 body
并且 不包含 docsify-specific HTML、`mb-callout` 容器或额外换行补丁标记

场景: 既有 foldable、icon、nested callout 与 docsify 导航行为不回退
测试: test_web_init_docsify_callout_multiline_fix_does_not_regress_existing_frontend_contract
假设 当前 docsify shell 已支持 callout icon、`[!type]+` / `[!type]-`、nested callout 与内部文档 hash 导航
当   引入多行 body 结构保留修复
那么 上述能力继续存在
并且 `.md` / `.base` 内部导航与 binary resource 直连行为保持不变

场景: 浏览器中 repo-owned multiline callout fixture 的段落与子项按多行显示
测试: manual browser acceptance
假设 仓库内通过 `TestVault` 或等价 repo-owned fixture 创建了一个专门复现该问题的 note，并由本地 `web serve --homepage <fixture-homepage>` 提供
当   浏览器打开该 fixture note 对应的 docsify 路由
并且 查看其中的 multiline callout
那么 “优先基于 `web-search` skill ...” 与后续说明按多行显示
并且 各个 `` `- ...` `` / `` `  - ...` `` 子项不会全部挤在同一行
并且 视觉结果体现逐行可读的块级结构
并且 完成该验收不需要依赖任何仓库外部文件
