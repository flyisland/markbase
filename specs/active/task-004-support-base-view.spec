---
id: task-004
title: "支持 `.base#View` 渲染选择器"
status: active
exec-plan: exec-001
phase: 2
boundaries:
  allowed:
    - "src/renderer/mod.rs"
    - "src/link_syntax.rs"
    - "tests/cli_note.rs"
    - "README.md"
  forbidden_patterns:
    - "specs/**"
    - "src/query/**"
    - "src/db.rs"
completion_criteria:
  - id: "cc-001"
    scenario: "单独一行的 `![[File.base#View]]` 只渲染指定 view"
    test: "test_note_render_base_embed_with_view_selector"
  - id: "cc-002"
    scenario: "未指定 view 时仍渲染全部 views"
    test: "test_note_render_base_embed_without_view_selector"
  - id: "cc-003"
    scenario: "找不到指定 view 时输出 warning 和占位注释"
    test: "test_note_render_base_embed_missing_view_selector"
  - id: "cc-004"
    scenario: "非 `.base` embed 仍保持原样输出"
    test: "test_note_render_non_base_embed_passthrough_after_parser_change"
---

## Intent

让 `note render` 在现有 `.base` embed 行替换能力上支持 `![[File.base#View]]`，从而只执行指定的 base view，补齐设计文档中已经写明但实现尚未覆盖的 Obsidian/markbase 语义。

## Decisions

- 仅支持“单独占一行”的 `.base` embed，与现有渲染范围一致
- “单独占一行”定义为：去掉行首行尾空白后，整行恰好只包含一个 `.base` embed token；带有 `> `、`- `、列表编号、callout 标记等前缀的行仍不处理
- view selector 使用 embed 中 `#` 后的名称做**区分大小写**的精确匹配
- `.base` embed 上的 alias / size 可以被解析，但渲染时忽略
- 找不到指定 view 时，stderr 必须输出：
  `WARN: view '<view-name>' not found in '<base-name>', skipping.`
- 同时 stdout 必须在原位输出：
  `<!-- [markbase] view '<view-name>' not found in '<base-name>' -->`
- 找到指定 view 时，只渲染该 view，不回退渲染其他 view

## Boundaries

### Allowed Changes

- src/renderer/mod.rs
- src/link_syntax.rs
- tests/cli_note.rs
- README.md

### Forbidden

- 不得扩展到 Markdown inline link / image 语法
- 不得修改 renderer filter 翻译规则
- 不得改动数据库 schema 或 note render 命令行参数

## Completion Criteria

场景: 单独一行的 `![[File.base#View]]` 只渲染指定 view
测试: test_note_render_base_embed_with_view_selector
假设 `tasks.base` 包含 `Open Tasks` 和 `Closed Tasks` 两个 view
当   渲染正文为 `![[tasks.base#Open Tasks]]` 的 note
那么 输出只包含 `Open Tasks` 的渲染结果

场景: 未指定 view 时仍渲染全部 views
测试: test_note_render_base_embed_without_view_selector
假设 `tasks.base` 包含多个 view
当   渲染正文为 `![[tasks.base]]` 的 note
那么 输出行为与当前实现一致，所有 view 依次渲染

场景: 找不到指定 view 时输出 warning 和占位注释
测试: test_note_render_base_embed_missing_view_selector
假设 正文为 `![[tasks.base#Missing View]]`
当   执行渲染
那么 stderr 输出 warning，stdout 在原位输出可见占位注释
并且 不渲染 `tasks.base` 中的任何其他 view

场景: 非 `.base` embed 仍保持原样输出
测试: test_note_render_non_base_embed_passthrough_after_parser_change
假设 正文包含 `![[image.png]]`
当   执行渲染
那么 该行原样输出，不触发 base 渲染逻辑
