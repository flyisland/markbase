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
3. Relevant files under `docs/design-docs/` and `spec/`
4. `README.md` if user-visible behavior may change

## Core Documents

- `ARCHITECTURE.md`: system map, boundaries, invariants, and shared-logic rules
- `docs/core-beliefs.md`: project-specific engineering beliefs for choosing between valid implementations
- `docs/DESIGN.md`: entry index for design docs
- `README.md`: user-facing behavior and command contract
- `docs/design-docs/design-001-links-and-embeds.md`: link, embed, backlink, and rename semantics
- `spec/properties_design.md`: `file.*` vs `note.*` field model
- `spec/query_design.md`: query mode and translation rules
- `spec/note_render_design.md`: `.base` rendering pipeline
- `spec/template_schema.md`: template schema behavior

## Execution Rules

- Search for an existing implementation before adding new logic.
- Reuse shared logic instead of creating a second parser or translator.
- Keep CLI parsing, environment handling, and stdout/stderr routing in `src/main.rs`.
- Do not hide writes in modules that sound read-only.
- Treat query translation and renderer filter translation as coupled behavior.

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
- Query defaults are agent-oriented; human-readable tables are opt-in.
