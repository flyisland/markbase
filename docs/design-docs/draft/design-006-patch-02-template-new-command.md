---
id: design-006-patch-02
title: "Template New Command"
status: draft
parent: design-006
module: templates
---

# Template New Command

**Status:** Draft  
**Target:** `design-006` template-system follow-up  
**Related docs:** `docs/design-docs/implemented/design-006-template-system.md`, `docs/design-docs/implemented/design-011-note-creation.md`, `ARCHITECTURE.md`, `README.md`

## Purpose

This patch adds a first-class `markbase template new` command that creates a
canonical template scaffold under `MARKBASE_BASE_DIR/templates/`.

The goal is not to make template authoring fully interactive. The goal is to
give humans and agents one supported write path for creating new templates that
already match the active `_schema` / `_schema.create` contract used by
`template describe`, `note new --template`, and template-aware web metadata.

This patch treats templates as a specialized document class, not as ordinary
instance notes. A template file is a control document that defines:

- which note family it governs
- how filenames should be chosen
- where created notes should live
- which frontmatter fields are required or constrained
- which body sections agents may fill or update over time

That means `template new` is not just "note creation, but under `templates/`".
It is the authoring path for new note-management contracts.

## Review Findings Against `design-006`

Reviewing `docs/design-docs/implemented/design-006-template-system.md` shows
three design gaps that matter before a `template new` implementation lands:

1. `design-006` defines template storage, normalization, and consumption, but
   it does not define a write path for authoring new templates.
2. The document defines template identity as path-free names mapped to
   `templates/<name>.md`, but it does not define template-name validation or
   duplicate handling for a creation command.
3. The document defines the active normalized shape of templates, but it does
   not define the canonical initial scaffold that markbase itself should emit.

Without a patch, those three decisions would likely be made ad hoc in CLI code
or README examples, which would violate the repo's rule that user-facing
translation behavior should be explicit and centralized.

## Decision Summary

This patch makes five decisions:

1. Add `markbase template new <name>` as the supported template-authoring write
   path.
2. Reuse the same path-free, extension-free naming rules used by `note new`.
3. Reuse the same global duplicate-name defense used by note creation, so a new
   template cannot silently collide with an existing note or resource identity.
4. Generate one canonical template-authoring scaffold in the active `_schema`
   shape rather than emitting legacy outer-frontmatter instance fields.
5. Keep the command filesystem-only like `note new`; it does not require DuckDB
   and does not trigger indexing itself.

In addition, this patch makes two template-specific choices that differ from
ordinary note creation:

- the generated file explicitly includes `filename.description`, `location`,
  `type`, and section-scoped agent directives because those are core parts of
  template authoring in real markbase vaults
- the implementation may use a dedicated write-path module for template
  creation, because templates are control documents rather than note instances

## Command Contract

The new command surface is:

```bash
markbase template new company
```

Behavior:

1. Validate `<name>` as a template-facing name.
2. Check for an existing logical-name collision anywhere in the vault.
3. Create `templates/` if it does not already exist.
4. Write `templates/<name>.md` with the canonical scaffold defined below.
5. Print only the relative created path on stdout.

Success output:

```text
templates/company.md
```

Failure behavior:

- non-zero exit status
- user-facing error on stderr

Like `note new`, this command is a direct filesystem write path. The new
template becomes visible to indexed commands on the next normal indexing pass.

## Input Rules

`template new` accepts a path-free template name only.

The validation contract matches `note new`:

- name cannot be empty
- name must not include directories
- name must not include a file extension

Examples rejected before any file write:

- `crm/company`
- `../company`
- `company.md`

Implementation should use shared validation logic rather than reimplementing
this rule inline.

The preferred implementation shape is a dedicated
`validate_template_name()` that reuses the same path-free and extension-free
checks as `validate_note_name()` while keeping template-specific user-facing
error wording.

In particular, `template new` errors should refer to `template name`, not
`note name`, even if the underlying checks are shared.

## Duplicate Detection

Before writing the new template, markbase must perform the same logical-name
collision check used by `note new`.

Current repo-wide identity rules make this necessary:

- Markdown-note identity is basename-based across the vault.
- Indexed non-Markdown resource identity is full-filename-based.
- Template files live inside the vault and therefore participate in the same
  collision model.

So `template new company` must fail if any existing file has:

- stem `company`
- filename exactly `company`

This preserves the global uniqueness invariant from `ARCHITECTURE.md` instead
of carving out a special collision exemption for templates.

## Generated Scaffold Contract

`template new <name>` writes this canonical initial shape:

```markdown
---
# [MKS Template Definition]
_schema:
  description: >-
    模板用途说明。
    说明这种 note 用来记录什么，什么时候应该优先使用这个模板，
    以及它与相邻模板的边界是什么。
  strict: false
  required:
    - description
    - type
  filename:
    description: >-
      说明实例 note 的命名规则。
      写清楚推荐格式、命名粒度，以及至少 2-3 个示例。
  location: "inbox/"
  properties:
    description:
      type: text
      description: "一句话摘要，概括此 note 的核心内容、当前状态或主要价值，便于检索"
    type:
      type: text
      enum: ["replace-me"]
      description: "固定类型标记；在开始使用模板前改成该类 note 的稳定 type 值"
  create:
    description: ""
    type: replace-me
    tags: []
---

# {{name}}

## 1. 背景与目标

> [!agent-fill]-
> 说明这种 note 主要沉淀什么信息、解决什么问题，以及这一类内容为什么值得单独建档。
> 若该模板面向某类实体、活动或学习样本，明确写出判断标准和适用边界。

## 2. 关键信息

> [!agent-update]- Overwrite
> 固定维护这一类 note 最重要的结构化事实或当前结论。
> 适合放“当前版本应始终保持最新”的内容；每次更新时允许整体重写。

## 3. 观察与进展

> [!agent-update]- Accumulate
> 当后续出现新的事实、信号、事件或补充信息时，在此追加记录。
> 适合沉淀时间序列观察，而不是一次性静态描述。

## 4. 待确认问题

> [!agent-update]- Accumulate
> 记录当前仍未确认、需要后续补证或等待外部信息的问题。
> 若某项内容暂时无法可靠填写，优先在此说明缺口，而不是编造。

## 5. 备注
```

This scaffold is intentionally opinionated and follows the current template
house style used by real markbase vaults:

- it uses the active `_schema` plus `_schema.create` split
- it does not emit legacy outer-frontmatter instance fields
- it includes `strict`, `filename.description`, explicit `location`, explicit
  `type`, and `tags` because those are core authoring surfaces for real
  templates rather than optional decoration
- it includes both `description` and `type` in the schema/create surface so the
  new template starts from a realistic note-family contract rather than a bare
  text stub
- it demonstrates the two directive modes that current vaults rely on most:
  `agent-fill` for initial section authoring and `agent-update` for maintained
  sections
- it uses obviously incomplete placeholder values such as `replace-me` so the
  template remains visibly unfinished until the author specializes it for real
  use

The scaffold is a starting point, not a finished production template. Authors
are expected to edit `_schema.description`, `filename.description`,
`_schema.location`, the `type` field definition and enum, `_schema.create`, and
the body sections before relying on the template for normal note creation.

Under the current markbase contract, placeholder values such as `replace-me`
are an authoring convention rather than a verifier-enforced guard. They make
the scaffold obviously incomplete to humans and agents, but they are not by
themselves a guarantee that downstream commands will reject the untouched
template.

## Why `location` Is Explicit By Default

This patch intentionally writes `_schema.location` into the generated template
instead of relying only on `note new`'s inbox fallback.

That is a template-authoring decision, not an implementation convenience.

In the intended workflow, templates own instance routing. A newly created
template should therefore start with an explicit target directory, even if the
initial value is only `inbox/` and is expected to be specialized later.

This means the generated scaffold establishes a stronger baseline than "creation
falls back to inbox when location is absent":

- template authors are expected to think about routing up front
- location becomes part of the visible template contract immediately
- later verification behavior follows from that explicit contract rather than an
  implicit CLI fallback

## Why The Scaffold Is Explicit

This patch intentionally chooses one emitted scaffold instead of leaving the
output "implementation-defined."

That matters for three reasons:

1. `template describe` should show a newly created template in the same shape
   the command wrote, not a legacy compatibility form.
2. agents need a deterministic starting document that already matches the
   active template model.
3. future template examples in `README.md` should come from the same canonical
   structure the CLI generates.

## Module Ownership

The implementation boundary should mirror `note new`:

- `src/main.rs`
  CLI parsing, dispatch, stdout/stderr routing
- `src/name_validator.rs`
  template-name validation through shared path-free / extension-free rules
- new explicit write-path module such as `src/template_creator.rs`
  template-specific duplicate detection, target path selection, directory
  creation, scaffold rendering, and file write
- `src/template.rs`
  pure scaffold rendering helpers only, if needed; no hidden filesystem writes

This is intentionally not a requirement to force `template new` through
`src/creator.rs`.

`src/creator.rs` owns note-instance creation. `template new` creates control
documents whose job is to govern note families, routing, field contracts, and
agent write surfaces. A dedicated template-creation module is therefore
acceptable as long as it reuses shared validators and does not fork the public
duplicate-detection rule.

This keeps template creation as an obvious write path while preserving the
current role of `src/template.rs` as the template normalization/read-model
boundary.

## Relationship To Existing Designs

This patch extends `design-006`; it does not replace it.

After implementation, the steady-state ownership should be:

- `design-006`
  template storage, normalization, `template describe`, and `template new`
  scaffold semantics
- `design-011`
  `note new` command behavior only
- `README.md`
  user-facing command examples including `template new`

This patch does not change:

- `note new --template` instance materialization semantics
- `note verify` rules
- template normalization rules for existing files
- the legacy-compatibility window for reading older templates

## Non-Goals

This patch does not introduce:

- interactive prompts
- automatic schema inference from an example note
- extra flags such as `--location`, `--description`, or `--force`
- automatic follow-up edits to specialize the generated template
- automatic indexing after creation

Those can be considered later if the minimal write path proves insufficient.

## Merge Plan

If implemented, this patch should be folded back into
`docs/design-docs/implemented/design-006-template-system.md`, with related
user-facing examples added to `README.md`.

After merge, move this file to `docs/design-docs/obsolete/` and mark it
`status: obsolete:merged`.
