---
id: task-003
title: "将 renamer 改为基于 token span 的精确改写"
status: active
exec-plan: exec-001
phase: 2
boundaries:
  allowed:
    - "src/renamer.rs"
    - "src/link_syntax.rs"
    - "tests/cli_note.rs"
  forbidden_patterns:
    - "specs/**"
    - "src/verifier.rs"
    - "src/renderer/**"
completion_criteria:
  - id: "cc-001"
    scenario: "rename 会保留 heading、block 和 alias 后缀"
    test: "test_note_rename_preserves_heading_block_and_alias_suffix"
  - id: "cc-002"
    scenario: "rename 不会改写 fenced code 与 inline code 中的语法示例"
    test: "test_note_rename_skips_code_context_links"
  - id: "cc-003"
    scenario: "rename 能正确处理 `.md` 扩展和路径前缀形式"
    test: "test_note_rename_updates_md_extension_and_path_forms"
  - id: "cc-004"
    scenario: "frontmatter 纯 wikilink 字符串会被正确改写"
    test: "test_note_rename_updates_frontmatter_wikilink_strings_with_shared_parser"
---

## Intent

将 rename 流程从“对整段文本做 regex 替换”改为“对真实 token 的 span 做精确改写”，避免把代码样例或非语义文本误改，同时与共享解析层保持完全一致的目标识别规则。

## Decisions

- rename 仍只扫描 `.md` 文件
- rename 仅改写真实 wikilink / embed token，不改普通文本中的近似片段
- 对 Markdown note 目标，rewrite 后的语法必须规范化为**path-free 且不带 `.md` 扩展**的形式
- 对非 Markdown 资源目标，rewrite 后必须保留扩展名
- 原语法后缀必须保留：`#Heading`、`#^block`、`|alias` 或 embed size
- rewrite 的结果必须由 `ParsedTarget` 重建，不再从原始字符串上做局部 `strip_prefix(old_name)` 拼接
- 若 token 的 `normalized_target` 不等于 `old_name`，该 token 必须原样保留

## Boundaries

### Allowed Changes

- src/renamer.rs
- src/link_syntax.rs
- tests/cli_note.rs

### Forbidden

- 不得更改 note 命名规则或路径验证规则
- 不得引入基于数据库的新 rename 依赖
- 不得扩大 rename 到非 `.md` 文件

## Completion Criteria

场景: rename 会保留 heading、block 和 alias 后缀
测试: test_note_rename_preserves_heading_block_and_alias_suffix
假设 文本包含 `[[old#Heading|Alias]]` 与 `![[old#^block]]`
当   执行 `markbase note rename old new`
那么 输出文件中的语法分别变为 `[[new#Heading|Alias]]` 与 `![[new#^block]]`

场景: rename 不会改写 fenced code 与 inline code 中的语法示例
测试: test_note_rename_skips_code_context_links
假设 文件正文中同时存在真实链接、代码块中的 `[[old]]`、行内代码中的 `![[old]]`
当   执行 rename
那么 只有真实链接被改写

场景: rename 能正确处理 `.md` 扩展和路径前缀形式
测试: test_note_rename_updates_md_extension_and_path_forms
假设 文件中包含 `[[folder/old.md#Section]]`
当   执行 rename
那么 结果为 `[[new#Section]]`

场景: rename 保留非 Markdown 资源扩展名
测试: test_note_rename_preserves_non_markdown_extension
假设 文件中包含 `![[old.base#Open Tasks]]`
当   执行 rename old.base new.base
那么 结果为 `![[new.base#Open Tasks]]`

场景: frontmatter 纯 wikilink 字符串会被正确改写
测试: test_note_rename_updates_frontmatter_wikilink_strings_with_shared_parser
假设 frontmatter 字段值为 `[[old|Alias]]`
当   执行 rename
那么 该值被改写为 `[[new|Alias]]`
