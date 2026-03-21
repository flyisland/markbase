---
id: design-011
title: "Note Creation"
status: implemented
module: note-create
---

# `note new` Design

**Status:** Implemented  
**Target:** markbase CLI  
**Related docs:** `ARCHITECTURE.md`, `README.md`, `docs/design-docs/implemented/design-006-template-system.md`, `docs/design-docs/implemented/design-005-indexing.md`

## Scope

This document defines the active contract for `markbase note new`.

It covers:

- input validation for new note names
- default note creation without a template
- template-backed creation flow and its boundary with the template subsystem
- duplicate-name checks and output-path rules
- creation-time variable substitution

It does not define:

- template normalization details beyond the parts consumed by note creation
- note verification rules
- rename semantics
- note resolution behavior

## Purpose

`note new` is the primary filesystem write path for creating Markdown notes in a
vault that follows markbase naming and template conventions.

Its contract is:

- accept note-facing names rather than paths
- create notes directly on the filesystem
- keep creation behavior compatible with the template subsystem
- defend the global logical-name uniqueness model used across the repo

## Command Contract

```bash
markbase note new my-note
markbase note new customer --template company
```

Execution is split across:

- `src/main.rs`: CLI parsing and stdout output
- `src/name_validator.rs`: note-name validation
- `src/creator.rs`: duplicate detection, target-path selection, file creation,
  and variable substitution
- `src/template.rs`: template loading, normalization, and create-surface
  rendering

The command writes vault files directly and does not require DuckDB.

## Input Rules

`note new` validates the requested note name with `validate_note_name()`.

Current rules:

- name cannot be empty
- name must not include directory components
- name must not include a file extension

Examples rejected before any file write:

- `company/acme`
- `../acme`
- `acme.md`

## Creation Without A Template

Without `--template`, `note new` creates:

- a Markdown file at `inbox/<name>.md`
- with the default frontmatter field `description: 临时笔记`

This default content is owned by `src/template.rs::default_note_content()`.

## Template-Backed Creation

With `--template <name>`, `note new`:

1. loads `templates/<name>.md`
2. uses the normalized template view from `TemplateDocument::load(...)`
3. renders the create surface from `_schema.create`
4. strips `_schema` from the created instance
5. auto-injects `templates: ["[[<template-name>]]"]`
6. applies supported variable substitution
7. writes the final Markdown file to the target directory

Template semantics such as `_schema.create`, `_schema.location`, and legacy
normalization are owned by
`docs/design-docs/implemented/design-006-template-system.md`.

This document owns only the command-level creation behavior.

## Output Path Rules

Target directory selection is:

- `_schema.location` when the normalized template exposes one
- otherwise `inbox/`

The final written file name is always `<name>.md`.

If the parent directory does not exist, `note new` creates it before writing the
file.

## Duplicate Detection

Before writing the new file, `note new` scans the vault filesystem for an
existing logical-name collision.

Current duplicate rule:

- if any existing file has stem `<name>` or filename exactly `<name>`, creation
  fails

This makes creation consistent with the repo-wide rule that Markdown-note
identity is basename-based and resource identity is full-filename-based.

`note new` fails rather than silently disambiguating by directory.

## Variable Substitution

Current built-in variable replacement supports:

- `{{name}}`
- `{{date}}`
- `{{time}}`
- `{{datetime}}`

Whitespace inside the braces is tolerated, for example `{{ name }}`.

Replacement happens on the rendered create document before the file is written.

## Output Contract

On success, stdout prints only the path of the created file relative to
`MARKBASE_BASE_DIR`.

Examples:

- `inbox/my-note.md`
- `company/customer.md`

This relative-path-only success output is part of the public command contract.

On failure, the command returns a non-zero exit and prints an error message.

## Filesystem And Index Boundary

`note new` writes the note file directly to disk.

It does not trigger automatic indexing as part of the command itself.

The new note becomes visible to DB-backed commands on the next normal indexing
pass, which usually happens automatically when a DB-backed command runs.

## Relationship To Other Docs

- `ARCHITECTURE.md` defines note creation as an explicit write path and the
  global name uniqueness invariant it must defend.
- `docs/design-docs/implemented/design-006-template-system.md` owns template
  storage, normalization, and create-surface semantics.
- `docs/design-docs/implemented/design-005-indexing.md` defines the logical-name
  model that note creation must preserve.
- `README.md` documents user-facing examples and quick-start behavior.
