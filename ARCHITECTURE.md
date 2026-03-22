# Markbase Architecture

This document is the system map for markbase. Read it before changing behavior across modules.

## 1. Purpose

Markbase is a local-first CLI that turns a Markdown vault into a queryable index for both humans and AI agents.

Its job is not to replace Markdown files with a database. Its job is to keep a rebuildable DuckDB index in sync with the filesystem so commands can resolve note metadata, links, templates, and rendered views quickly.

## 2. Source of Truth

There are only two durable sources of truth:

- The vault filesystem under `MARKBASE_BASE_DIR`
- The conventions of that vault: unique note names, Obsidian-style links, frontmatter schema, and template layout

The DuckDB database at `.markbase/markbase.duckdb` is a derived artifact. It can always be rebuilt from the vault contents.

## 3. System Shape

```text
CLI (main.rs)
  -> name/path validation
  -> ensure index is up to date when command needs DB
  -> dispatch to command module

Filesystem
  -> scanner.rs traverses files
  -> extractor.rs parses markdown notes
  -> link_syntax.rs parses shared Obsidian link/embed tokens
  -> template.rs loads and normalizes templates
  -> db.rs stores derived note records

Database-backed reads
  -> query/        translates and executes note queries
  -> resolver.rs   resolves note names and aliases
  -> verifier.rs   validates notes against template schema
  -> renderer/     expands .base embeds and renders output
  -> web/          resolves canonical routes and serves web-facing markdown/resources

Filesystem-backed reads
  -> describe.rs   shows template content

Write paths
  -> scanner.rs    refreshes the derived index
  -> creator.rs    creates note files
  -> renamer.rs    renames files and rewrites links
```

## 4. Architectural Layers

### 4.1 Entry and orchestration

- `src/main.rs` is the only CLI entry point.
- It owns argument parsing, environment variables, output routing, automatic indexing, and command dispatch.
- It may orchestrate side effects across modules, but it should not absorb domain logic that belongs in a dedicated module.

### 4.2 Core storage

- `src/db.rs` owns DuckDB connection lifecycle, schema initialization, and note persistence/query primitives.
- The `notes` table is the canonical storage contract between indexing and DB-backed commands.
- Callers should treat `db.rs` as the boundary for SQL execution and row decoding.

### 4.3 Filesystem indexing and extraction

- `src/scanner.rs` is responsible for vault traversal, incremental indexing, duplicate-name detection, deletion cleanup, and optional backlink recomputation.
- Index traversal is recursive under `MARKBASE_BASE_DIR`, skips dot-prefixed hidden paths by default, applies ignore-file filtering, and only indexes files with an extension.
- `src/scanner.rs` parses Markdown note contents for structured fields and stores non-Markdown resources as indexed records without Markdown-derived fields.
- `src/extractor.rs` is a stateless parser for a single Markdown document.
- `src/link_syntax.rs` owns shared wiki-link and embed tokenization plus target normalization.
- `src/template.rs` loads templates and normalizes template frontmatter defaults.

This layer converts raw files into structured note records, but should not know about CLI presentation.

### 4.4 Query compilation and execution

- `src/query/detector.rs` decides whether input is expression mode or SQL mode and applies security restrictions.
- `src/query/translator.rs` converts user-facing field syntax into DuckDB-safe SQL.
- `src/query/executor.rs` coordinates translated SQL execution.
- `src/query/error_map.rs` turns DuckDB failures into user-facing errors.
- `src/query/mod.rs` owns query output shaping.

This layer exists so user query ergonomics can evolve without leaking raw schema details into the CLI contract.

### 4.5 Note lifecycle operations

- `src/creator.rs` creates note files from defaults or templates.
- `src/renamer.rs` performs rename operations and rewrites wiki-links and embeds across the vault.
- `src/resolver.rs` resolves names and aliases against the indexed vault.
- `src/name_validator.rs` centralizes path-free naming rules used by note-facing commands.

### 4.6 Validation and rendering

- `src/verifier.rs` validates note instances against template schema.
- `src/renderer/` expands `.base` embeds, translates render filters to SQL, and formats render output.
- `src/describe.rs` renders template content for inspection.
- `src/output.rs` and `src/renderer/output.rs` provide shared output formatting paths.

### 4.7 Web delivery

- `src/web/` owns canonical `file.path` route resolution, request-scoped index and DB lifecycle, HTTP serving, and web-targeted OFM normalization.
- The web layer reuses `src/renderer/` for note semantics instead of implementing a second note-embed or `.base` execution path.
- Web `.md` and `.base` routes return translated Markdown bodies; binary resource routes return raw bytes with a derived content type.
- `src/web/` owns docsify entry HTML generation for the supported browser entrypoint at `/` and `base-dir/index.html`.
- The generated docsify entry HTML owns frontend-only internal-link adaptation, resource URL normalization, callout UI upgrades, and embedded metadata such as the generating `markbase` version, homepage, and git commit/time; the backend web contract remains Markdown and resource bytes rather than callout-specific HTML.
- `web serve` is the user-facing browser surface. Without `--homepage` it only reuses exported `base-dir/index.html`; with `--homepage` it resolves that input to an existing `.md` or `.base` target and dynamically renders the same browser entry HTML at runtime. `web init-docsify` remains the non-required export/debug tool and `web get` remains the entry-HTML-independent inspection command.

## 5. Core Invariants

These are system-wide contracts. If a change breaks one of them, it is an architectural change, not a local refactor.

### 5.1 Note names are globally unique

- For Markdown notes, logical identity is based on filename without the `.md` extension, not path.
- For non-Markdown resources indexed by markbase, identity uses the full filename, including extension.
- Scanner and note creation defend this invariant; DB-backed single-target commands assume it and will error or behave ambiguously if the vault violates it.
- Duplicate names must be rejected, surfaced, or skipped rather than silently disambiguated by directory.

### 5.2 Obsidian link syntax is the external contract

- Internal logic must normalize Obsidian link targets to basename references by stripping path prefixes.
- For Markdown note targets, `.md` is not part of the logical note name and should be stripped.
- For non-Markdown targets such as `.base` files and attachments, the extension remains part of the stored target name.
- Frontmatter links must remain compatible with quoted Obsidian-style values.
- Rename, verify, extraction, and rendering behavior must preserve anchors, aliases, and embed forms.

### 5.3 The database is derived, not authoritative

- Scanner output must be reproducible from vault files.
- DB-backed features must tolerate full reindexing as the recovery path.
- Persistent business state must not exist only inside DuckDB.

### 5.4 Automatic indexing is part of command execution

- Non-`--dry-run` DB-backed commands assume the index is refreshed as part of command execution.
- The indexing pass is an orchestration concern owned by `main.rs` plus `scanner.rs`, not by every feature module.

### 5.5 Read paths and write paths stay explicit

- Querying, rendering, verification, and describing are read-oriented modules.
- Creation, rename, and indexing are write-oriented modules.
- Do not hide filesystem or DB mutations inside modules whose public contract appears read-only.

### 5.6 Bare query identifiers mean frontmatter

- `file.*` maps to native DB columns.
- `note.*` maps to frontmatter JSON.
- Bare identifiers are shorthand for `note.*`, not implicit file columns.

This rule must stay consistent across query and renderer filter translation.

### 5.7 Web routes are path-based externally and name-based internally

- Canonical browser routes resolve by vault-relative `file.path`.
- Once a Markdown note route is resolved, rendering still invokes the existing note-name-based render pipeline using `file.name`.
- Path-based web URLs are an external delivery contract only; they do not weaken the core invariant that note-facing identity inside markbase remains path-free.
- Web requests refresh the index before route resolution and close their DuckDB handle after the request is produced.

## 6. Dependency Boundaries

These rules keep the codebase legible to both humans and agents.

### Allowed directions

- `main.rs` may depend on all feature modules.
- Feature modules may depend on `db.rs`, `constants.rs`, validation helpers, and shared formatters when needed.
- `scanner.rs` may depend on `extractor.rs` and `db.rs`.
- `creator.rs`, `describe.rs`, and `verifier.rs` may depend on `template.rs`.
- `renderer/mod.rs` may depend on `renderer/filter.rs`, `renderer/output.rs`, `db.rs`, and extractor constants.
- `web/` may depend on `db.rs`, `scanner.rs`, `renderer/`, `link_syntax.rs`, and shared validation helpers.

### Discouraged directions

- `extractor.rs`, `template.rs`, `name_validator.rs`, and query translation modules should remain mostly stateless utility/domain modules.
- `query/` should not read files from disk directly.
- `verifier.rs` and `renderer/` should not perform hidden writes.
- Modules should not duplicate CLI parsing, environment handling, or stdout/stderr routing that belongs in `main.rs`.

## 7. Shared Logic That Must Not Diverge

Some rules are important enough that they should have one implementation or one clearly mirrored contract across modules.

### 7.1 Wiki-link and embed normalization

- `src/extractor.rs` defines the canonical normalization behavior for Obsidian link targets.
- `src/renamer.rs`, `src/verifier.rs`, `src/scanner.rs`, and render-related code must follow the same target semantics.
- Do not introduce a second independent interpretation of path stripping, `.md` stripping, anchor removal, or alias removal.

### 7.2 Query field resolution

- `src/query/translator.rs` is the primary contract for `file.*`, `note.*`, and bare identifier semantics.
- `src/renderer/filter.rs` must stay behaviorally aligned with those same namespace rules.
- If one module changes how bare fields or `file.*` behave, the other must be updated in the same change.

### 7.3 Output shape

- `src/query/mod.rs`, `src/output.rs`, and `src/renderer/output.rs` may format different command surfaces, but they should not invent conflicting meanings for the same conceptual fields.
- In particular, agent-facing structured output and human-facing table output should remain stable enough that callers can rely on them.

### 7.4 Web normalization and routing

- `src/web/` must reuse `src/link_syntax.rs` for live wikilink and embed parsing rather than introducing a second parser.
- Web markdown generation must reuse renderer semantics for recursive note expansion, `.base` expansion, and placeholder behavior.
- Canonical web route lookup uses `file.path`, but link rewrite and render execution must continue to respect the existing name-based note identity model.

### 7.4 Validation of note-facing names

- `src/name_validator.rs` is the shared gate for path-free note, resolve, and render target names.
- New note-facing commands should reuse these validators rather than reimplementing path and extension checks inline.

## 8. Performance Model

Markbase is designed around predictable local performance, not distributed complexity.

- Fast enough indexing comes from incremental scans and narrow parsing, not from premature parallelism.
- Query latency comes from keeping the vault index current and translating field syntax cleanly into DuckDB-native operations.
- Dependency count matters because binary size, compile time, and operational surface area all affect a CLI used as tooling infrastructure.

## 9. Security Model

- `query` accepts only `SELECT` statements.
- Multi-statement SQL injection is rejected before execution.
- File-targeting commands validate names so note-oriented APIs cannot be tricked into path traversal behavior.
- Single-writer DuckDB assumptions still apply: indexing owns write coordination.

## 10. Change Guide

When changing one part of the system, inspect the neighboring contracts as well.

- Query semantics: update `src/query/`, `src/renderer/filter.rs`, `docs/design-docs/implemented/design-010-query-subsystem.md`, `README.md`, and query-related tests.
- Link parsing or rename behavior: update `src/extractor.rs`, `src/renamer.rs`, `src/verifier.rs`, `docs/design-docs/implemented/design-001-links-and-embeds.md`, and note-related tests.
- Index schema or note fields: update `src/db.rs`, `src/scanner.rs`, `README.md`, `AGENTS.md`, and any affected specs.
- Index traversal or ignore semantics: update `README.md`, `ARCHITECTURE.md`, `docs/design-docs/implemented/design-005-indexing.md`, and scanner tests.
- Template behavior: update `src/template.rs`, `src/creator.rs`, `src/describe.rs`, `src/verifier.rs`, and `docs/design-docs/implemented/design-006-template-system.md`.
- Note creation behavior: update `src/creator.rs`, `src/template.rs`, `docs/design-docs/implemented/design-011-note-creation.md`, `docs/design-docs/implemented/design-006-template-system.md`, and `README.md`.
- Render pipeline behavior: update `src/renderer/`, `docs/design-docs/implemented/design-002-render.md`, and render tests.
- Web delivery behavior: update `src/web/`, `src/renderer/` when web output shape changes, `docs/design-docs/implemented/design-003-web-note-view.md`, `README.md`, `ARCHITECTURE.md`, and web-related tests.

## 11. Documentation Role

This file should stay stable and structural. Put changing implementation details in:

- `docs/design-docs/implemented/design-009-document-system.md` for managed-document placement, authority, and lifecycle rules
- `README.md` for user-facing behavior
- `AGENTS.md` for developer/agent entry guidance
- `docs/design-docs/implemented/` and `docs/design-docs/candidate/` for managed feature-level design details
- `docs/references/` for older or supporting design material that is not part of the managed document system
- tests for executable regression coverage
