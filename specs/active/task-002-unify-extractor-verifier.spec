---
id: task-002
title: "统一 extractor 与 verifier 的 link 语义"
status: active
exec-plan: exec-001
phase: 1
boundaries:
  allowed:
    - "src/extractor.rs"
    - "src/verifier.rs"
    - "src/scanner.rs"
    - "tests/cli_index.rs"
    - "tests/cli_note.rs"
  forbidden_patterns:
    - "specs/**"
    - "src/renamer.rs"
    - "src/renderer/**"
completion_criteria:
  - id: "cc-001"
    scenario: "extractor 正确提取 escaped pipe 形式的 link/embed"
    test: "test_extract_links_with_escaped_pipe"
  - id: "cc-002"
    scenario: "frontmatter 混合文本中的 wikilink 会被提取，但 frontmatter embed 不进入 embeds"
    test: "test_extract_frontmatter_text_with_links_but_not_embeds"
  - id: "cc-003"
    scenario: "verifier 接受与 extractor 一致的合法 link 形式"
    test: "test_verify_link_field_accepts_md_anchor_alias_form"
  - id: "cc-004"
    scenario: "verifier 拒绝包含额外文本的非纯 wikilink 值"
    test: "test_verify_link_field_rejects_non_pure_wikilink_string"
---

## Intent

将 `Extractor` 和 `verify_link_field` 切换到同一套解析和 normalization 规则，消除当前“索引认为合法、校验认为非法”或反过来的不一致行为。

## Decisions

- `links` 与 `embeds` 的存储模型保持不变，不新增列
- frontmatter 中的 `![[...]]` 仍不进入 `links` / `embeds` 存储
- verifier 对 `format: link` 字段要求整个字符串就是一个 wikilink
- extractor 中 body 扫描必须只消费 `ScanContext::MarkdownBody`
- frontmatter 字符串扫描必须使用 `ScanContext::FrontmatterString`
- frontmatter 混合字符串里的 `[[...]]` 应被提取；同一字符串里的 `![[...]]` 应被忽略，但不能因此整串跳过
- verifier 校验合法 link 时必须复用 `parse_link_target()`，不再手写 `split_once('|')` / `split_once('#')`
- verifier 对纯 wikilink 的判断必须是“整个字段值只包含一个 wikilink token，且 token 覆盖去首尾空白后的全部内容”

## Boundaries

### Allowed Changes

- src/extractor.rs
- src/verifier.rs
- src/scanner.rs
- tests/cli_index.rs
- tests/cli_note.rs

### Forbidden

- 不得引入新的数据库字段
- 不得修改 renamer 或 renderer 的行为实现
- 不得改变 bare field / `file.*` / `note.*` 查询语义

## Completion Criteria

场景: extractor 正确提取 escaped pipe 形式的 link/embed
测试: test_extract_links_with_escaped_pipe
假设 Markdown 表格单元格中包含 `[[Note\\|Alias]]` 与 `![[Image.png\\|200]]`
当   执行提取
那么 `links` 中记录 `Note`
并且 `embeds` 中记录 `Image.png`

场景: frontmatter 混合文本中的 wikilink 会被提取，但 frontmatter embed 不进入 embeds
测试: test_extract_frontmatter_text_with_links_but_not_embeds
假设 frontmatter 字符串中同时包含普通文本、`[[note]]` 和 `![[note]]`
当   执行提取
那么 `[[note]]` 进入 `links`，`![[note]]` 不进入 `embeds`

场景: verifier 接受与 extractor 一致的合法 link 形式
测试: test_verify_link_field_accepts_md_anchor_alias_form
假设 schema 的 `format: link` 字段值为 `[[folder/note.md#Heading|Alias]]`
当   执行校验
那么 该值按合法 wikilink 处理，并使用统一 normalization 查找目标 note

场景: verifier 拒绝包含额外文本的非纯 wikilink 值
测试: test_verify_link_field_rejects_non_pure_wikilink_string
假设 字段值为 `prefix [[note]] suffix`
当   执行校验
那么 返回 invalid link format，而不是把其中片段误判为合法链接

场景: verifier 接受首尾空白包裹的纯 wikilink
测试: test_verify_link_field_accepts_trimmed_pure_wikilink
假设 字段值为 `  [[note]]  `
当   执行校验
那么 该值按合法 wikilink 处理
