---
id: design-010
title: "Query Subsystem"
status: implemented
module: query
---

# Query Subsystem Design

**Status:** Implemented  
**Target:** markbase CLI  
**Related docs:** `ARCHITECTURE.md`, `README.md`, `docs/design-docs/implemented/design-005-indexing.md`, `docs/design-docs/implemented/design-002-render.md`

## Scope

This document defines the active contract for the `markbase query` command and
the shared query subsystem behind it.

It covers:

- command input modes and execution flow
- field namespace and translation rules
- default select shape, output shape, and default limit
- SQL-mode safety checks
- output formatting behavior shared by query-backed commands
- error mapping responsibilities at the query boundary

It does not define:

- render-time filter syntax beyond the requirement that it stay aligned with
  query field semantics
- note resolution semantics
- template-specific behavior

## Purpose

The query subsystem exists to give humans and agents a stable, low-friction way
to ask for indexed note data without exposing raw storage details as the public
interface.

Its contract is:

- preserve a concise user-facing expression syntax
- allow full `SELECT` SQL when users need explicit control
- keep file metadata and frontmatter access predictable across commands
- default to machine-friendly output

## Command Contract

```bash
markbase query
markbase query "author == 'Tom'"
markbase query "SELECT file.path, note.author FROM notes WHERE note.author = 'Tom'"
markbase query "author == 'Tom'" -o table
markbase query --dry-run "author == 'Tom'"
markbase query --abs-path "author == 'Tom'"
```

Execution is split across:

- `src/main.rs`: CLI parsing, automatic indexing, `--dry-run`, `--abs-path`, and
  output routing
- `src/query/detector.rs`: mode detection and SQL safety checks
- `src/query/translator.rs`: field translation and final SQL construction
- `src/query/executor.rs`: default limit handling and DB execution
- `src/query/error_map.rs`: user-facing query error mapping
- `src/query/mod.rs`: output shaping and optional absolute-path rewriting
- `src/output.rs`: JSON and Markdown table rendering

## Input Modes

`markbase query` supports three input states.

### Empty input

If no query string is provided, query runs the default select with no `WHERE`
clause.

### Expression mode

Any non-empty input that does not begin with `SELECT` is treated as expression
mode.

Expression mode accepts:

- a `WHERE`-like predicate such as `author == 'Tom'`
- an optional suffix beginning with `ORDER`, `GROUP`, `HAVING`, or `LIMIT`
- suffix-only input such as `ORDER BY file.mtime DESC LIMIT 10`

Expression mode does not require users to write `SELECT ... FROM notes`.

### SQL mode

Any input whose trimmed text begins with `SELECT`, case-insensitively, is
treated as SQL mode.

SQL mode is still translated through the shared field translator so `file.*`,
`note.*`, and bare frontmatter fields remain part of the public contract.

## Field Resolution Contract

Field resolution is the core shared behavior of the query subsystem.

- `file.*` maps to native columns in the `notes` table
- `note.*` maps to frontmatter JSON extraction
- bare identifiers are shorthand for `note.*`

Examples:

- `file.name` -> indexed file name column
- `note.author` -> frontmatter field `author`
- `author` -> same frontmatter field as `note.author`

This behavior is owned by `src/query/translator.rs` and must stay aligned with
`src/renderer/filter.rs`.

## Translation Contract

### Default select

For empty input or expression mode, the query subsystem builds this default
logical select surface:

- `file.path`
- `file.name`
- `description`
- `file.mtime`
- `file.size`
- `file.tags`

The translator emits a normalized single-line SQL string.

### SQL mode translation

In SQL mode, translator preserves the caller's `SELECT ... FROM notes ...`
shape, but still rewrites field references according to the namespace rules.

### Type casts

Expression and SQL inputs may use explicit casts such as:

- `year::INTEGER`
- `note.created::TIMESTAMP`

If a change alters cast behavior or field meaning, the query doc, README, and
renderer filter behavior must change together.

## Safety Contract

The public `query` command is read-only.

Current safety rules:

- only `SELECT` statements are accepted in SQL mode
- multi-statement input is rejected
- non-`SELECT` leading SQL keywords are rejected before execution

These checks are owned by `src/query/detector.rs` and enforced before DB
execution.

## Execution Flow

### Normal execution

For non-`--dry-run` query execution, `main.rs`:

1. resolves `MARKBASE_BASE_DIR`
2. refreshes the derived index
3. executes the translated query against DuckDB
4. formats the results to stdout

Query therefore sees current indexed note state, not stale process-local state.

### Dry-run execution

`markbase query --dry-run ...` does not require a database connection or an
index refresh.

Dry-run prints the translated SQL string to stdout and exits.

## Default Limit

Unless the translated SQL already contains `LIMIT`, query appends a default
limit of `1000`.

This default applies to:

- empty input
- expression mode
- SQL mode without an explicit `LIMIT`

If the caller already specified `LIMIT`, query preserves it and does not append
`LIMIT 1000`.

## Output Contract

The default output format is `json`.

- `json` is the default machine-facing contract
- `table` is an explicit opt-in human-facing view

### JSON output

JSON output is a pretty-printed array of records keyed by the selected field
names.

If a cell value is parsed as an array-looking scalar such as `["a","b"]`, the
query output layer emits a JSON array for that cell instead of a raw string.

Empty result sets serialize as:

```json
[]
```

### Table output

`-o table` renders a compact Markdown table.

Pipe characters in cells are escaped so output remains valid Markdown table
syntax.

### Absolute path rewriting

`--abs-path` is an output-layer concern, not a query-semantics change.

Current behavior:

- only `file.path` and `file.folder` are rewritten
- all other selected fields keep their original query result values

## Error Mapping Contract

The query subsystem converts common DuckDB failures into user-facing query
errors.

Current mapped categories include:

- conversion/type mismatch failures
- unknown field failures
- JSON path syntax failures
- parse/syntax failures
- binder/type mismatch failures

This mapping exists to preserve a stable query interface even when the DB
engine's raw error text changes shape.

## Relationship To Other Docs

- `ARCHITECTURE.md` defines subsystem boundaries and the requirement that query
  field resolution stay aligned with renderer filter translation.
- `README.md` documents user-visible command examples and flags.
- `docs/design-docs/implemented/design-005-indexing.md` defines the indexed
  data surface that query reads.
- `docs/design-docs/implemented/design-002-render.md` depends on this document
  for shared `file.*`, `note.*`, and bare-field semantics.
- `docs/references/legacy-designs/query_design.md` is historical context only.
  When it differs from current behavior, this document is authoritative.
