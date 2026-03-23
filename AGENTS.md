# Markbase Agent Guide

This file is the entry point for developers and coding agents. It is intentionally brief.

## Quick Start

```bash
RUSTC_WRAPPER= cargo build
RUSTC_WRAPPER= cargo test
RUSTC_WRAPPER= cargo clippy -- -D warnings
RUSTC_WRAPPER= cargo fmt --check
```

Common local run pattern:

```bash
export MARKBASE_BASE_DIR=./notes
RUSTC_WRAPPER= cargo run -- query "author == 'Tom'"
```

## Local Build Environment

- Repo-local Cargo config sets a shared target directory at `/tmp/markbase-target` so new worktrees can reuse build artifacts.
- In Codex sandbox runs, invoke Rust commands with `RUSTC_WRAPPER=` in the same command to bypass host-level `sccache` configuration that may fail inside the sandbox.
- If a command needs extra environment variables, keep `RUSTC_WRAPPER=` in that same invocation, for example `MARKBASE_BASE_DIR=./notes RUSTC_WRAPPER= cargo run -- query "author == 'Tom'"`.

## Read First

Before making non-trivial changes, read these in order:

1. `ARCHITECTURE.md`
2. `docs/guidelines/core-beliefs.md`
3. `docs/design-docs/implemented/design-009-document-system.md`
4. `specs/project.md` and `specs/org.md`
5. Relevant active design docs listed in `Task Navigation` below
6. Relevant files under `docs/exec-plans/active/` and `specs/active/` when the task is covered there
7. `README.md` if user-visible behavior may change
8. `docs/references/` only when an active document explicitly points there or historical context is required

## Core Documents

- `ARCHITECTURE.md`: system map, boundaries, invariants, and shared-logic rules
- `docs/guidelines/core-beliefs.md`: project-specific engineering beliefs for choosing between valid implementations
- `docs/design-docs/implemented/design-009-document-system.md`: managed-document rules, lifecycle states, and directory layout
- `specs/project.md`: repository-wide technical constraints
- `specs/org.md`: organization-level constraints for this repo
- `README.md`: user-facing behavior and command contract
- `docs/design-docs/implemented/design-010-query-subsystem.md`: active query command and field-translation contract
- `docs/design-docs/implemented/design-011-note-creation.md`: active `note new` command contract
- `docs/design-docs/implemented/design-003-web-note-view.md`: active web note view contract
- `docs/design-docs/implemented/design-013-web-note-metadata-mode.md`: active web note metadata route and JSON contract
- `docs/design-docs/implemented/design-001-links-and-embeds.md`: link, embed, backlink, and rename semantics
- `docs/design-docs/implemented/design-002-render.md`: active render contract
- `docs/design-docs/implemented/design-006-template-system.md`: active template subsystem behavior
- `docs/design-docs/implemented/design-008-note-resolve.md`: active note resolution contract
- `docs/design-docs/implemented/design-004-note-verify.md`: active note verification contract
- `docs/design-docs/implemented/design-005-indexing.md`: active indexing contract

## Task Navigation

Prefer active managed docs before legacy references.

- Query semantics and output: `docs/design-docs/implemented/design-010-query-subsystem.md`, then `README.md`
- Note creation: `docs/design-docs/implemented/design-011-note-creation.md`, then `docs/design-docs/implemented/design-006-template-system.md`
- Rename and link rewriting: `docs/design-docs/implemented/design-001-links-and-embeds.md`
- Note resolve: `docs/design-docs/implemented/design-008-note-resolve.md`
- Note verify: `docs/design-docs/implemented/design-004-note-verify.md`, then `docs/design-docs/implemented/design-006-template-system.md`
- Note render: `docs/design-docs/implemented/design-002-render.md`, then `docs/design-docs/implemented/design-001-links-and-embeds.md`
- Web note view: `docs/design-docs/implemented/design-003-web-note-view.md`, then `docs/design-docs/implemented/design-013-web-note-metadata-mode.md`, then `docs/design-docs/candidate/design-014-docsify-note-sidebar-ui.md` when working on docsify sidebar UI, then `docs/design-docs/implemented/design-002-render.md`, then `docs/design-docs/implemented/design-001-links-and-embeds.md`
- Index traversal, collisions, and ignore behavior: `docs/design-docs/implemented/design-005-indexing.md`
- Template normalization and describe output: `docs/design-docs/implemented/design-006-template-system.md`
- Documentation-system rules: `docs/design-docs/implemented/design-009-document-system.md`

## Historical References

Use `docs/references/` only for migration context, abandoned designs, or when an active doc explicitly points there.

- `docs/references/legacy-designs/query_design.md`: historical query design only
- `docs/references/legacy-designs/note_render_design.md`: historical render design only
- `docs/references/legacy-designs/properties_design.md`: historical field-model context only

## Execution Rules

- Search for an existing implementation before adding new logic.
- Reuse shared logic instead of creating a second parser or translator.
- Keep CLI parsing, environment handling, and stdout/stderr routing in `src/main.rs`.
- Do not hide writes in modules that sound read-only.
- Treat query translation and renderer filter translation as coupled behavior.
- When an active Exec Plan or Task Spec conflicts with current implementation details or legacy design docs, follow the active plan/spec and update the affected docs and tests in the same change.

## Validation Rules

- Run `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt --check` before considering a task complete.
- If behavior changes, update `README.md`.
- If structure, invariants, or shared-logic boundaries change, update `ARCHITECTURE.md`.
- If project-level decision principles change, update `docs/guidelines/core-beliefs.md`.
- If a repeatable bug is fixed, add or strengthen a test.

## Notes for This Repo

- Note-facing names must be path-free.
- Markdown note identity is name-based; non-Markdown indexed resources keep their filename including extension.
- `.base` files are valid render targets and participate in indexed resource behavior.
- `src/link_syntax.rs` is the shared link/embed parsing contract; do not reintroduce per-module regex parsing.
- Query defaults are agent-oriented; human-readable tables are opt-in.
