# Markbase Agent Guide

This file is the entry point for developers and coding agents. It is intentionally brief.

## Quick Start

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

Common local run pattern:

```bash
export MARKBASE_BASE_DIR=./notes
cargo run -- query "author == 'Tom'"
```

## Read First

Before making non-trivial changes, read these in order:

1. `ARCHITECTURE.md`
2. `docs/core-beliefs.md`
3. `docs/DOCUMENTATION.md`
4. Relevant files under `docs/design-docs/` and `docs/design-docs/legacy/`
5. Relevant files under `docs/exec-plans/active/` and `specs/active/` when the task is covered there
6. `README.md` if user-visible behavior may change

## Core Documents

- `ARCHITECTURE.md`: system map, boundaries, invariants, and shared-logic rules
- `docs/core-beliefs.md`: project-specific engineering beliefs for choosing between valid implementations
- `docs/DOCUMENTATION.md`: where docs belong, which layer is authoritative, and how to classify new docs
- `docs/DESIGN.md`: entry index for design docs
- `docs/PLANS.md`: entry index for active execution plans
- `README.md`: user-facing behavior and command contract
- `docs/design-docs/design-001-links-and-embeds.md`: link, embed, backlink, and rename semantics
- `docs/design-docs/legacy/properties_design.md`: `file.*` vs `note.*` field model
- `docs/design-docs/legacy/query_design.md`: query mode and translation rules
- `docs/design-docs/legacy/note_render_design.md`: `.base` rendering pipeline
- `docs/design-docs/design-006-template-system.md`: active template subsystem behavior

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
- If project-level decision principles change, update `docs/core-beliefs.md`.
- If a repeatable bug is fixed, add or strengthen a test.

## Notes for This Repo

- Note-facing names must be path-free.
- Markdown note identity is name-based; non-Markdown indexed resources keep their filename including extension.
- `.base` files are valid render targets and participate in indexed resource behavior.
- `src/link_syntax.rs` is the shared link/embed parsing contract; do not reintroduce per-module regex parsing.
- Query defaults are agent-oriented; human-readable tables are opt-in.
