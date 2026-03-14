---
id: task-006
title: "实现 note embed 的 body 展开"
status: active
exec-plan: exec-002
phase: 1
boundaries:
  allowed:
    - "src/renderer/mod.rs"
    - "tests/cli_note.rs"
  forbidden_patterns:
    - "specs/**"
    - "src/query/**"
    - "src/db.rs"
completion_criteria:
  - id: "cc-001"
    scenario: "`![[note]]` 展开为目标 note 的 body，不包含 frontmatter"
    test: "test_note_render_embedded_note_uses_body_without_frontmatter"
  - id: "cc-002"
    scenario: "inline note embed 会按块拆开前后文本"
    test: "test_note_render_inline_note_embed_splits_surrounding_text"
  - id: "cc-003"
    scenario: "带 alias 的 note embed 与不带 alias 的运行时行为一致"
    test: "test_note_render_note_embed_alias_does_not_change_body_expansion"
  - id: "cc-004"
    scenario: "带 heading 或 block selector 的 note embed 继续按字面透传"
    test: "test_note_render_note_embed_with_selector_is_passthrough"
  - id: "cc-005"
    scenario: "缺失的 embedded note 会输出 warning 和 placeholder，但不会中止整个 render"
    test: "test_note_render_missing_embedded_note_is_soft_failure"
---

## Intent

为 renderer 建立 whole-note Markdown embed 的基础展开能力，让 `![[note]]` 成为正式支持的 render-time 语义，而不是普通文本透传。

这个任务只处理**单层** note embed 的基础输出合同，不处理递归展开和 cycle guard；那部分由后续任务单独负责。

## Decisions

- note embed 只在 `ScanContext::MarkdownBody` 可见的正文 token 中生效
- render 目标必须是 Markdown note identity；非 Markdown 目标不进入这个分支
- `![[note]]` 与 `![[note|Alias]]` 都展开为同一个 note body；alias 不参与 render 决策
- 展开时只使用 embedded note 去掉 frontmatter 之后的 Markdown body
- 不为 embedded note 添加标题、wrapper 注释、来源注释或其他额外包装
- inline note embed 采用块插入：token 前后的文本各自保留，但展开 body 必须独立成段，不能拼接回同一行
- inline note embed 的块插入规则固定如下：
  - 若 token 前后都有普通文本，则输出顺序是“前文一行 + expanded body 原样多行 + 后文一行”
  - 若 token 位于行首或行尾，则只保留存在的一侧文本，不额外制造空白文本行
  - 若 token 前一字符已经是换行，前文侧不额外补空行；若 token 后一字符已经是换行，后文侧不额外补空行
  - 若 embedded note body 为空，则该 embed 位置不输出正文内容，但仍保持前文与后文按块边界分开，不把两侧重新拼回同一行
- `![[note#Heading]]` 和 `![[note#^blockid]]` 暂不执行，继续按字面输出
- embedded note 不存在或读取失败属于 soft failure：stderr 输出 warning，stdout 原位输出 placeholder comment，然后继续渲染后续内容
- 本任务不改变 direct `.base` render 的任何行为

## Boundaries

### Allowed Changes

- src/renderer/mod.rs
- tests/cli_note.rs

### Forbidden

- 不得改动 `.base` render 的 SQL/输出格式合同
- 不得修改 shared parser 的 normalization 语义
- 不得通过让 selector note embed 也执行来简化实现
- 不得把 embedded note frontmatter 混入正文输出
- 不得在缺失 embedded note 时中止整个 render
- 不得顺手修改 direct `.base` render 入口或输出

## Completion Criteria

场景: `![[note]]` 展开为目标 note 的 body，不包含 frontmatter
测试: test_note_render_embedded_note_uses_body_without_frontmatter
假设 `note-a` 的 frontmatter 含有 `tags`、`status` 等字段，正文为多行 Markdown
当   另一个 note 渲染 `![[note-a]]`
那么 输出只包含 `note-a` 的 body 内容
并且 不包含其 frontmatter 字段或 `---` 分隔符

场景: inline note embed 会按块拆开前后文本
测试: test_note_render_inline_note_embed_splits_surrounding_text
假设 正文包含 `Before![[note-a]]After`
当   执行 `note render`
那么 输出中的 `Before`、`note-a` 的展开 body、`After` 分别出现在独立行段中

场景: 带 alias 的 note embed 与不带 alias 的运行时行为一致
测试: test_note_render_note_embed_alias_does_not_change_body_expansion
假设 两处正文分别包含 `![[note-a]]` 与 `![[note-a|Shown Name]]`
当   执行 `note render`
那么 两处都展开为相同的 `note-a` body 内容

场景: 带 heading 或 block selector 的 note embed 继续按字面透传
测试: test_note_render_note_embed_with_selector_is_passthrough
假设 正文包含 `![[note-a#Heading]]` 与 `![[note-a#^blockid]]`
当   执行 `note render`
那么 这些 token 保持原样输出
并且 不会触发 embedded note render

场景: 缺失的 embedded note 会输出 warning 和 placeholder，但不会中止整个 render
测试: test_note_render_missing_embedded_note_is_soft_failure
假设 正文包含 `![[missing-note]]`
当   执行 `note render`
那么 stderr 输出针对 `missing-note` 的 warning
并且 stdout 在原位输出 embedded note 缺失的 placeholder comment
并且 render 仍继续输出该位置之后的其余内容
