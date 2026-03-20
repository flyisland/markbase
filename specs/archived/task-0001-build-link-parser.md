---
id: task-0001
title: "建立共享 link/embed 解析器"
status: completed
exec-plan: exec-001
phase: 1
boundaries:
  allowed:
    - "src/link_syntax.rs"
    - "src/lib.rs"
    - "src/extractor.rs"
  forbidden_patterns:
    - "specs/**"
    - "src/query/**"
    - "src/db.rs"
completion_criteria:
  - id: "cc-001"
    scenario: "共享解析器能区分 wikilink 和 embed"
    test: "test_parse_link_tokens_distinguishes_embed_and_wikilink"
  - id: "cc-002"
    scenario: "转义管道符会按语义分隔符处理"
    test: "test_parse_link_tokens_preserves_escaped_pipe"
  - id: "cc-003"
    scenario: "`.md` 扩展与 anchor/alias 能被正确规范化"
    test: "test_normalize_target_strips_md_before_anchor_and_alias"
  - id: "cc-004"
    scenario: "fenced code 和 inline code 中的 link 语法会被忽略"
    test: "test_parse_link_tokens_skips_code_contexts"
---

## Intent

为 markbase 增加单一的 link/embed 解析入口，输出结构化 token 和统一的 normalization 结果，作为 extractor、renamer、verifier、renderer 的共同语义基础。这一步是整个执行计划的前置任务。

## Decisions

- 使用轻量字符扫描器，不引入完整 Markdown AST 依赖
- scanner 必须同时支持两种上下文：`MarkdownBody` 和 `FrontmatterString`
- `MarkdownBody` 模式必须跳过 ``` fenced code、`~~~` fenced code、以及反引号 code span
- `FrontmatterString` 模式不做 code context 排除，直接扫描整段字符串
- 共享模块文件固定为 `src/link_syntax.rs`
- 共享模块必须至少暴露这两个公开函数：
  - `scan_link_tokens(input: &str, context: ScanContext) -> Vec<LinkToken>`
  - `parse_link_target(raw_inner: &str) -> ParsedTarget`
- `LinkToken` 必须包含：
  - `kind`：`WikiLink` 或 `Embed`
  - `full_span`：原文中完整 `[[...]]` / `![[...]]` 的字节范围
  - `inner_span`：原文中括号内部内容的字节范围
  - `raw_inner`：不含外层括号的原始文本
  - `parsed`：`ParsedTarget`
- `ParsedTarget` 必须包含：
  - `normalized_target`：供索引、校验、渲染查找使用的逻辑目标名
  - `target_text`：去掉 alias 后、保留 anchor 之前的目标文本
  - `anchor`：未转义 `#` 后的文本，去掉前导 `#`
  - `alias_or_size`：未转义 `|` 后的文本，去掉前导 `|`
  - `is_markdown_note`：`normalized_target` 是否代表 Markdown note
- target normalization 顺序固定为：
  1. trim `raw_inner`
  2. 先找第一个未转义 `|`，拆出 alias / size
  3. 再在 alias 之前的部分找第一个未转义 `#`，拆出 anchor
  4. 对 anchor 之前的 target 部分去路径前缀，只保留最后一个 `/` 之后的 basename
  5. 若 basename 以 `.md` 结尾，则去掉 `.md`
- `\|` 在表格场景下必须按**语义分隔符**处理，最终行为与未转义 `|` 一致；反斜杠只用于跨过 Markdown 表格单元格，不会让 link 目标真的变成 `Note|Alias`
- 未闭合的 `[[` / `![[` 必须被忽略，不生成 token
- 本任务不处理 Markdown inline links、Markdown images、HTML embeds

## Boundaries

### Allowed Changes

- src/link_syntax.rs
- src/lib.rs
- src/extractor.rs

### Forbidden

- 不得修改 DuckDB schema 或 `src/db.rs`
- 不得改动 query 语法翻译层
- 不得通过放宽测试或修改 spec 来绕过解析缺陷

## Completion Criteria

场景: 共享解析器能区分 wikilink 和 embed
测试: test_parse_link_tokens_distinguishes_embed_and_wikilink
假设 输入同时包含 `[[note]]` 和 `![[note]]`
当   调用共享解析器扫描文本
那么 返回的 token 中 wikilink 和 embed 类型被正确区分

场景: 转义管道符会按语义分隔符处理
测试: test_parse_link_tokens_preserves_escaped_pipe
假设 输入包含 `[[Note\\|Alias]]` 和 `![[Image.png\\|200]]`
当   解析并规范化 target
那么 `normalized_target` 分别为 `Note` 和 `Image.png`
并且 `alias_or_size` 分别为 `Alias` 和 `200`

场景: `.md` 扩展与 anchor/alias 能被正确规范化
测试: test_normalize_target_strips_md_before_anchor_and_alias
假设 输入包含 `[[note.md#Heading|Alias]]`
当   规范化 target
那么 `normalized_target` 为 `note`
并且 `anchor` 为 `Heading`
并且 `alias_or_size` 为 `Alias`

场景: fenced code 和 inline code 中的 link 语法会被忽略
测试: test_parse_link_tokens_skips_code_contexts
假设 文本同时包含正文 link、fenced code 中的 link、inline code 中的 link
当   扫描文本
那么 仅正文中的真实 link/embed 会生成 token

场景: 未闭合 token 会被忽略
测试: test_parse_link_tokens_ignores_unclosed_syntax
假设 输入包含 `[[note`、`![[image.png` 或只有 `[[`
当   扫描文本
那么 不会生成任何 token
