---
id: task-0014
title: "修正 embed 展开后的 quote-container 保留"
status: completed
exec-plan: exec-005
phase: 1
boundaries:
  allowed:
    - "src/renderer/mod.rs"
    - "tests/cli_note.rs"
  forbidden_patterns:
    - "specs/**"
    - "src/query/**"
    - "src/db.rs"
    - "README.md"
completion_criteria:
  - id: "cc-001"
    scenario: "blockquote 中的 live note embed 展开后保留 quote prefix"
    test: "test_note_render_blockquote_note_embed_preserves_quote_prefix"
  - id: "cc-002"
    scenario: "callout 中的 live `.base` embed 展开后保留容器结构"
    test: "test_note_render_callout_base_embed_preserves_container"
  - id: "cc-003"
    scenario: "quote-container 中的 blank line 和 nested depth 在展开后仍保留"
    test: "test_note_render_quote_container_preserves_blank_lines_and_nested_depth"
  - id: "cc-004"
    scenario: "list item 中的 live embed 仍保持 literal output"
    test: "test_note_render_list_item_embed_remains_literal"
  - id: "cc-005"
    scenario: "soft-failure placeholder 在 quote-container 中仍保留结构"
    test: "test_note_render_quote_container_placeholder_preserves_prefix"
---

## Intent

修正当前 renderer 在 callout / blockquote 中展开 live note embed 与 live `.base` embed 时会破坏 quote-container 结构的问题，使实现与 active render 设计和 web note view 设计重新对齐。

这个任务只处理 renderer 的容器保留语义，不引入任何 web route、HTTP 接口或 web-only 补丁逻辑。

## Decisions

- blockquote 与 callout 在 live embed 展开时视为同一类 quote container
- 当 live note embed 或 live `.base` embed 出现在 quote container 内时，展开后的每一行都继承 embed-bearing source line 的 quote prefix depth
- blank line 也必须保留在同一 quote container 内，不能因为展开而逃逸出容器
- nested quote depth 必须逐行保留，例如 `>> ![[note]]` 展开后仍保持两层 quote prefix
- 对 callout，原始 marker line 如 `> [!info]` 保持不变；展开结果只继承其后续内容所在的 quote prefix，不生成新的 marker line
- inline embed 在 quote container 中仍按块内容展开，然后再对每一行应用 quote prefix preservation
- soft-failure placeholder comment 在 quote container 中也必须保留相同的 quote prefix
- list item exclusion 继续生效；即使同一逻辑行还包含 quote/callout 语法，list item 中的 live embed 仍保持 literal output
- 本任务不改变 whole-note embed 是否可执行、`.base` view 选择、`--dry-run` 语义或 direct `.base` render 合同

## Boundaries

### Allowed Changes

- src/renderer/mod.rs
- tests/cli_note.rs

### Forbidden

- 不得在 web 层单独打补丁来掩盖 renderer 语义缺口
- 不得改变 list item 中 live embed 的 passthrough 规则
- 不得顺手支持 selector-based note embed
- 不得修改 shared `src/link_syntax.rs` parser contract
- 不得改变 direct `.base` render 输出格式或 wrapper 合同

## Completion Criteria

场景: blockquote 中的 live note embed 展开后保留 quote prefix
测试: test_note_render_blockquote_note_embed_preserves_quote_prefix
假设 正文包含 blockquote 容器，内部含有 `![[note-a]]`
当   执行 `markbase note render`
那么 `note-a` 展开的每一行都保留对应的 `>` quote prefix
并且 容器结构不会在展开后断裂

场景: callout 中的 live `.base` embed 展开后保留容器结构
测试: test_note_render_callout_base_embed_preserves_container
假设 正文包含 `> [!info]` callout，内部含有 `![[tasks.base]]`
当   执行 `markbase note render -o table`
那么 `.base` 展开后的每一行都保留 callout 所在的 quote prefix
并且 原始 callout marker line 保持不变

场景: quote-container 中的 blank line 和 nested depth 在展开后仍保留
测试: test_note_render_quote_container_preserves_blank_lines_and_nested_depth
假设 展开内容包含空行且 source line 位于多层 blockquote 中
当   执行 `markbase note render`
那么 空行仍留在 quote container 内
并且 每一行保持原有 nested quote depth

场景: list item 中的 live embed 仍保持 literal output
测试: test_note_render_list_item_embed_remains_literal
假设 正文在 list item 中包含 `![[note-a]]` 或 `![[tasks.base]]`
当   执行 `markbase note render`
那么 这些 token 继续按字面输出
并且 不会因为同一行含有 quote/callout 语法而被错误展开

场景: soft-failure placeholder 在 quote-container 中仍保留结构
测试: test_note_render_quote_container_placeholder_preserves_prefix
假设 quote container 内的 embedded note 缺失或触发 cycle guard
当   执行 `markbase note render`
那么 placeholder comment 仍保留对应 quote prefix
并且 quote container 结构不被破坏
