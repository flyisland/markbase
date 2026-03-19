# Template System Design

**Status:** Active  
**Target:** markbase CLI  
**Related docs:** `docs/design-docs/design-004-note-verify.md`, `ARCHITECTURE.md`, `README.md`

## Scope

This document defines the active markbase contract for template files as a shared subsystem.

It covers:

- where templates live and how they are addressed
- the active template vocabulary for outer frontmatter, `_schema`, and body directives
- the normalized template view produced by `src/template.rs`
- which parts of template frontmatter affect `markbase note new`
- what `template describe` shows
- which template semantics are shared inputs for `note new`, `template describe`, and `note verify`
- current behavior gaps that are part of the implementation reality today

It does not restate the full `note verify` command contract, output shape, or exit behavior. Those belong to `docs/design-docs/design-004-note-verify.md`.

## Why This Doc Exists

Historically, template behavior was split across:

- the legacy MTS format reference
- README command descriptions
- implementation plans for `description`
- `note verify` design
- the current implementation in `src/template.rs`, `src/creator.rs`, `src/describe.rs`, and `src/verifier.rs`

That left one gap: there was no active design doc describing the template subsystem itself, especially the shared normalization rules and the boundary between template semantics and `note verify`.

This document fills that gap.

## Template Storage And Identity

- Templates are Markdown files stored under `MARKBASE_BASE_DIR/templates/`.
- A template is addressed by its path-free template name, which maps to `templates/<name>.md`.
- `TemplateDocument::load()` is the shared load path for template-backed commands.
- Missing template files are command or verification errors at the caller boundary; template loading itself is not responsible for CLI formatting.

## Template Model

Templates have two distinct layers:

1. Outer frontmatter
2. Markdown body

The active frontmatter model is split three ways:

- outer frontmatter stores metadata on the template file itself
- `_schema` stores template-owned routing and validation metadata
- `_schema.create` stores the note-creation defaults to materialize during `note new --template`

The Markdown body is copied into created notes after variable substitution. Template callouts remain in the instance body unchanged.

### The Three `description` Keys

Templates use the word `description` in three different places. They are related, but they do not mean the same thing:

1. `_schema.description`: template-level routing prompt
2. `_schema.properties.description`: schema definition for the instance field named `description`
3. `_schema.create.description`: the actual one-line semantic summary stored on the instance note

These three layers must stay distinct. `_schema.description` does not replace `_schema.properties.description`, and neither of them replaces the instance frontmatter field.

## Active Template Vocabulary

markbase templates currently expose three authoring surfaces:

1. outer frontmatter
2. `_schema` inside frontmatter
3. Markdown body directives

### `_schema` Root Keys

The active template vocabulary uses these `_schema` keys:

| Key | Meaning | Current markbase role |
| --- | --- | --- |
| `description` | Template-level routing prompt | Stored in template; not executed by `note new`; may be consumed by external agents |
| `strict` | Whether writes should be restricted to declared fields | Declared vocabulary only; not executed by `note new`; not enforced by current `note verify` |
| `required` | Required field names for template-backed notes | Used by `note verify`; normalized to include `description` |
| `filename.description` | Natural-language filename guidance | Declared vocabulary only; not executed by current CLI |
| `location` | Relative directory for created notes | Used by `note new --template`; checked by `note verify` |
| `properties` | Field constraint definitions | Used partly by normalization and verification |
| `create` | Literal frontmatter defaults for created notes | Used by `note new --template`; not treated by `note verify` as exact-match constraints |

### `_schema.properties.<field>` Keys

The active field-definition vocabulary uses these keys:

| Key | Meaning | Current markbase role |
| --- | --- | --- |
| `type` | Field type such as `text`, `number`, `boolean`, `date`, `datetime`, `list` | Used by `note verify`; only `description.type` is auto-normalized for creation |
| `format` | Extra field format, currently `link` | Used by `note verify` |
| `target` | Expected target note `type` for link fields | Used by `note verify` |
| `enum` | Allowed values | Used by `note verify` |
| `description` | Field-level prompt or extraction hint | Preserved in schema; shown via `template describe`; included in verify definition lines |
| `default` | Fallback value when no information is available | Declared vocabulary only; not materialized by current `note new` |

### Field Types

Current template field types:

| Type | Meaning |
| --- | --- |
| `text` | Plain text |
| `number` | Numeric value |
| `boolean` | Boolean value |
| `date` | Date value in `YYYY-MM-DD` form |
| `datetime` | Date-time value in `YYYY-MM-DDTHH:MM` form |
| `list` | Array value |

## Shared Normalized View

`src/template.rs` is the canonical normalization layer for template-backed read paths.

Current normalization rules:

1. Parse frontmatter as YAML when possible; on parse failure, treat the file as body-only content with empty frontmatter.
2. Ensure `_schema` exists and is an object. If absent or non-object, normalize it to an empty object.
3. Preserve `_schema.location` separately as the template-selected creation directory.
4. Ensure `_schema.required` exists and is an array.
5. Ensure `_schema.required` contains `description`.
6. Ensure `_schema.properties` exists and is an object.
7. Ensure `_schema.properties.description` exists and is an object.
8. Ensure `_schema.properties.description.type` defaults to `text`.
9. Ensure `_schema.properties.description.description` defaults to `一句话说明这个 note 是什么`.
10. Ensure `_schema.create` exists and is an object.
11. Ensure the instance materialization path produces a string `description` field even when `_schema.create.description` is absent or non-string.
12. During the current compatibility window, selected legacy outer-frontmatter instance-like fields may be absorbed into `_schema.create`, but arbitrary outer frontmatter is not treated as the active instance skeleton.

This normalized view is shared by:

- `template describe`
- `note new --template`

`note verify` currently reads template files independently instead of reusing the normalized template view. Verification behavior therefore depends on raw template contents plus its own logic, except where shared concepts are documented here.

## Command Ownership

### `template describe`

`template describe <name>` shows the normalized template view.

This command exists so users and agents can inspect the exact template content that markbase uses for template-backed creation flows, including the normalized `_schema.create` block and auto-normalized `description` schema fields for older templates.

### `note new --template`

`note new --template <name>` creates an instance from the normalized template view and applies only a narrow subset of template semantics.

Current creation flow:

1. Load and normalize the template.
2. Compute the output directory from `_schema.location` when present; otherwise fall back to `inbox/`.
3. Render the instance frontmatter from `_schema.create`.
4. Remove `_schema` from the instance frontmatter before writing the file.
5. Auto-inject `templates: ["[[<template-name>]]"]`.
6. Replace supported body/frontmatter variables such as `{{name}}`, `{{date}}`, `{{time}}`, and `{{datetime}}`.

### `note verify`

`note verify` consumes template files as validation inputs, but the verify command contract is owned by `docs/design-docs/design-004-note-verify.md`.

This document only defines the template-side semantics that verification depends on:

- templates live in `templates/`
- `_schema` is template-only metadata
- `_schema.create` is a creation blueprint, not an exact-match verification surface
- continuing invariants come from `_schema.required` and `_schema.properties`

## Instance Creation Contract

When markbase creates a note from a template:

- `_schema.create` fields are copied into the instance
- `_schema` is stripped and never written into the instance
- `templates: ["[[<template-name>]]"]` is injected by the system
- body content is copied into the instance
- template variables are substituted
- template callouts are preserved in the instance body

`note new` does not currently synthesize instance fields from schema definitions beyond the `description` normalization path described above.

## Schema Field Support Matrix

The current implementation does not treat every `_schema` key as executable creation behavior.

### Keys used by `note new --template`

- `_schema.location`
- `_schema.create`
- `_schema.required`, only indirectly through normalization that forces `description` into the list
- `_schema.properties.description`, only indirectly through normalization that ensures a default schema entry exists

### Keys not executed by `note new --template`

- `_schema.description`
- `_schema.strict`
- `_schema.filename`
- `_schema.properties.<field>.type` for ordinary fields
- `_schema.properties.<field>.enum`
- `_schema.properties.<field>.format`
- `_schema.properties.<field>.target`
- `_schema.properties.<field>.default`

In particular, schema `default` values are not materialized into instance frontmatter during note creation.

## Outer Frontmatter Versus `_schema`

markbase currently uses this split:

- outer frontmatter describes the template file itself
- `_schema` describes template-owned metadata and constraints
- `_schema.create` describes the created instance frontmatter

This split matters for both creation and verification:

- `note new` materializes `_schema.create` and strips `_schema` entirely from instances
- `note new` auto-injects `templates` instead of requiring template authors to hand-write it
- `note verify` ignores outer-frontmatter seed literals and checks schema-driven constraints instead

Compatibility note:

- current normalization may still absorb selected legacy outer-frontmatter instance fields into `_schema.create`
- `_schema.create` wins when both old and new forms are present
- legacy outer-frontmatter `templates` is never copied into created instances

## Body Directives

Templates may embed agent-oriented directives in the Markdown body using Obsidian callouts. markbase treats these as note content, not as a separate metadata channel.

### Directive Types

Current directive shapes:

- `[!agent-fill]`: initial collection directive
- `[!agent-update]`: consolidation directive

`[!agent-update]` may carry an update policy title. The documented vocabulary is:

- `Overwrite`
- `Append`
- `Accumulate`

### markbase CLI Behavior

The current CLI contract is intentionally narrow:

- `note new --template` copies directive callouts into the created instance body unchanged
- `template describe` shows them as part of the normalized template body
- `note verify` does not currently validate directive semantics

This means directive content is part of the template authoring surface, but markbase's direct responsibility today is preserving it through creation and inspection rather than executing it.

## Dangling Link Convention

Template field definitions may use `format: link`. In agent workflows, unresolved links may appear in dangling-reference form:

```text
[?[David Chen]]
```

This is not a confirmed wikilink. The current markbase verification contract for dangling references is defined in `docs/design-docs/design-004-note-verify.md`.

## Reference Template

This example shows the active shape of a template that uses `_schema`, `_schema.create`, and body directives together:

```markdown
---
_schema:
  description: 标准客户档案模版。用于建立新客户的基本信息库，记录组织架构、技术栈和关键活动。
  strict: false
  required: [description, type, industry, size]
  filename:
    description: 使用客户的常用简称作为文件名，如"绿米"而非"绿米联合创新科技有限公司"。
  location: company/
  properties:
    description:
      type: text
      description: 一句话说明这个 note 是什么
    type:
      type: text
      enum: [company]
    industry:
      type: text
      description: 客户所在行业，如"智能家居"、"金融科技"。
    size:
      type: text
      enum: [startup, smb, enterprise]
      description: 公司规模，从枚举值中选择最匹配的。
    website:
      type: text
      description: 官网 URL。
    related_contacts:
      type: list
      format: link
      target: person
      description: 该客户的已知联系人，每人一个双链。
  create:
    type: company
    description: ""
    industry: ""
    size: ""
    website: ""
    related_contacts: []
    tags: []
    aliases: []
---

## 1. 公司简介

> [!agent-fill]-
> 用 2-3 句话概括公司的主营业务、市场定位和核心产品。

## 2. 组织架构

> [!agent-update]- Accumulate
> 每次发现新的联系人信息时追加一条记录。
```

## Current Gaps And Non-Goals

These are current active behavior notes, not proposals:

- There is no generalized schema-default materialization path; `_schema.properties.<field>.default` remains distinct from `_schema.create.<field>`.
- `note verify` does not currently reuse `TemplateDocument` normalization, so creation and verification do not share one fully normalized template object.
- The legacy MTS reference may describe a broader schema intent than the subset of behavior currently executed by markbase.

## Boundary With `design-004-note-verify`

The split between the two docs is intentional.

This document owns:

- template location and identity
- normalized template read model
- instance generation semantics for `note new`
- which template keys are creation-active today
- shared terminology for outer frontmatter versus `_schema`

`docs/design-docs/design-004-note-verify.md` owns:

- `note verify` CLI contract
- verification flow and early-return rules
- issue levels, summary behavior, and exit codes
- how `_schema.required`, `_schema.properties`, link constraints, and template body embed requirements are checked during verification

When template storage, normalization, or creation semantics change, update this document first and then update `design-004-note-verify.md` only if verification behavior or ownership boundaries also change.
