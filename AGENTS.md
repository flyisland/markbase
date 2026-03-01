# Agent Specification: Markdown Base CLI

## 1. Project Overview

The goal is to build a high-performance Command Line Interface (CLI) tool designed to scan, parse, and index Markdown files into a **DuckDB** database for instantaneous metadata searching with Obsidian compatibility in mind.

## 2. Technical Stack

See [README.md](./README.md#tech-stack) for the complete tech stack details.

## 3. Data Schema

The DuckDB local file (`.markbase/markbase.duckdb`) utilizes the following schema to support Markdown structures:

| Property   | Type        | Description                              |
| ---------- | ----------- | ---------------------------------------- |
| path       | TEXT        | File path relative to base-dir (primary key) |
| folder     | TEXT        | Directory path relative to base-dir     |
| name       | TEXT        | File name (without extension)            |
| ext        | TEXT        | File extension                           |
| size       | INTEGER     | File size in bytes                       |
| ctime      | TIMESTAMPTZ | Created time                             |
| mtime      | TIMESTAMPTZ | Modified time                            |
| content    | TEXT        | Full file content (including frontmatter)|
| tags       | VARCHAR[]   | Array of tags                            |
| links      | VARCHAR[]   | Array of wiki-links                      |
| backlinks  | VARCHAR[]   | Array of backlink files                  |
| embeds     | VARCHAR[]   | Array of embeds                          |
| properties | JSON        | Frontmatter properties                   |

**Note**: Array types stored as native VARCHAR[] arrays, properties as JSON.

### 3.1 Indexes
```sql
CREATE INDEX IF NOT EXISTS idx_mtime ON documents(mtime);
CREATE INDEX IF NOT EXISTS idx_folder ON documents(folder);
CREATE INDEX IF NOT EXISTS idx_name ON documents(name);
```

## 4. Operational Requirements

For command usage, options, and examples, see [README.md](./README.md#commands). The following documents the **internal behavior logic** relevant to implementation.

### Command: `index`
- **Concurrency**: Sequential processing with WalkDir iterator.
- **Logic**:
    - Perform incremental updates by comparing `mtime`; skip unchanged files.
    - Extract YAML Frontmatter using `gray_matter`.
    - Parse wiki-links `[[link]]`, embeds `![[embed]]`, and tags `#tag` using regex.
    - Calculate backlinks (reverse link lookup) post-indexing across all documents.
    - Insert documents using parameterized queries with `duckdb::params!`.
    - Warn if a frontmatter field conflicts with a reserved field (except `tags`).

### Command: `query`
- **Field resolution**: Reserved fields are checked first, then frontmatter properties via JSON path.
- **Nested JSON paths**: `a.b.c` → `json_extract_string(properties, '$."a"."b"."c"')`
- **Operators**: `==`, `!=`, `>`, `<`, `>=`, `<=`, `=~` (LIKE pattern), `=` (alias for `==`)
- **Logical operators**: `and` / `or` with standard precedence (`and` binds tighter than `or`)
- **Functions**: `has(field, value)` for array containment; `exists(field)` for property existence
- **Output**: Timestamps displayed as `YYYY-MM-DD HH:MM:SS`; supports table, json, list formats.
- **SQL safety**: Query compiler generates parameterized SQL to prevent injection.

### Command: `note`
Manages notes within the vault.

#### `note new`
- Creates a markdown note at the given path under base-dir.
- If `--template` is provided, copies and renders the named template from `templates/` under base-dir.

#### `note rename`
- Renames a note by its name (not path).
- Fails if the note name is ambiguous (multiple files with same name) or if the new name already exists.
- Updates all wiki-links pointing to the old name across all files in the vault.
- Preserves aliases and section anchors in links.

## 5. Module Responsibilities

> For actual implementation, read the source files directly under `src/`. This section describes each module's responsibilities and key design decisions that are not self-evident from the code.

- **`main.rs`** — CLI entry point using clap derive macros. Handles argument parsing and dispatches to the appropriate command handler. Database path is derived from base-dir as `{{base-dir}}/.markbase/markbase.duckdb`; base-dir can be overridden via environment variable (`MARKBASE_BASE_DIR`) or CLI arg (`--base-dir`).

- **`scanner.rs`** — Drives the `index` command. Walks the directory tree, compares `mtime` for incremental updates, orchestrates calls to `extractor.rs` and `db.rs`, and computes backlinks as a reverse-lookup pass *after* all documents are inserted.

- **`extractor.rs`** — Stateless parsing of a single file's content. Extracts frontmatter (via `gray_matter`), wiki-links, embeds, and tags using regex. Tags are merged from both content (`#tag`) and frontmatter (`tags:` field). Returns a `Document` struct; has no knowledge of the database.

- **`db.rs`** — All DuckDB interaction. Owns the connection and schema initialization. Uses `INSERT OR REPLACE` for upserts. Row values are accessed via `duckdb::types::Value` — be aware that column index order must match the schema definition exactly.

- **`creator.rs`** — Handles the `note new` command. Resolves the template path under `templates/` in base-dir, copies it, and substitutes template variables. Fails explicitly if the target file already exists.

- **`renamer.rs`** — Handles the `note rename` command. Looks up notes by name, validates uniqueness, renames the file on disk, updates all wiki-links pointing to the old name across all files, and reindexes affected documents. Preserves aliases and section anchors in links.

- **`query/tokenizer.rs`** — Converts a raw query string into a flat token stream. Distinguishes `Function` tokens (identifiers followed by `(`) from `Field` tokens at this stage.

- **`query/parser.rs`** — Recursive descent parser. Builds an AST from the token stream. Enforces operator precedence: `and` binds tighter than `or`. Grouping with parentheses is supported.

- **`query/compiler.rs`** — Walks the AST and emits parameterized DuckDB SQL. Field resolution happens here: reserved fields map directly to column names; all others map to `json_extract_string(properties, ...)`. Nested dot-notation (e.g. `a.b.c`) is handled by splitting on `.` and constructing the JSON path.

- **`query/mod.rs`** — Output formatting only (table, json, list). No query logic lives here.

## 6. Performance Goals
- **Indexing Speed**: < 5 seconds for 10,000 files (cold start).
- **Search Latency**: < 50ms for complex queries on 10,000 rows.
- **Query Latency**: < 100ms for complex query expressions.
- **Binary Size**: Optimized release builds with `strip = true` in Cargo.toml.

## 7. Constraints & Safety
- **Single Writer**: Only one `index` process can run at a time (DuckDB constraint).
- **Graceful Shutdown**: Database connection closed when `Database` struct is dropped.
- **Error Handling**: Comprehensive use of `Result` type with `?` operator for propagation.
- **Incremental Updates**: Compare `mtime` to skip unchanged files.
- **Parameterized Queries**: Query compiler uses parameterized queries to prevent SQL injection.
- **Thread Safety**: Uses `Mutex<Database>` for thread-safe access in multi-threaded contexts.

## 8. Development Commands

See [README.md](./README.md#development) for usage examples and [README.md#testing] for test execution.

## 9. Project Structure

See [README.md](./README.md#project-structure) for the complete project structure.

## 10. Development Status

### Completed ✅
- Core indexing functionality
- Query system (SQL-like expressions)
- Field-based queries (reserved fields + frontmatter, simplified resolution)
- Query operators (==, !=, >, <, >=, <=, =~, =)
- Logical operators (and, or) with precedence
- has() function for array containment
- Multiple output formats (table, json, list)
- Backlink tracking
- Rust migration complete
- CLI with clap derive macros
- Incremental updates via mtime comparison
- Note creation with templates (`note new` command)
- Note renaming with link updates (`note rename` command)
- Single equals operator (=) support
- Simplified field resolution (no file.*/note.* namespaces)
- Frontmatter conflict warnings (reserved fields except tags)
- `template describe` command
- `MDB_OUTPUT` environment variable support
- Global `--output-format` / `-o` option for query and template list
- `note new --template` outputs path + content for agent workflow
- Query type error transformation (compile-time + runtime)

### Technical Debt / Future Improvements
- Add integration tests for full query pipeline
- Benchmark performance against 10,000 files goal
- Consider parallel processing for indexing
- Add configuration file support
- Implement query result caching

### Test Coverage Summary

| Module         | Coverage                                                            |
| -------------- | ------------------------------------------------------------------- |
| `tokenizer.rs` | Field tokenization, operators, literals, functions, parentheses     |
| `parser.rs`    | Expression parsing, operators, grouping, precedence, single equals  |
| `compiler.rs`  | SQL generation, field resolution, all operators, type checking     |
| `extractor.rs` | Frontmatter, tags, wiki-links, embeds, edge cases                   |
| `db.rs`        | Database operations, queries, CRUD, error transformation            |
| `scanner.rs`   | File scanning, indexing, backlinks, subdirectories                  |
| `query/mod.rs` | Output formatting (table, JSON, list)                               |
| `creator.rs`   | Template resolution, file creation                                  |
| `renamer.rs`   | Link updates, note renaming, edge cases                             |
| `main.rs`      | CLI options, default values, parsing                                |

## 11. Development Workflow (Agent Guidelines)

### 11.1 Branch Policy

> **Trigger**: Whenever a user proposes any development task (including bug fixes, new features, documentation changes, etc.), you **must first** check the current branch status.
>
> If not on `main` branch, check whether you need to create a new branch based on latest `main`.
> If already on a feature/fix branch, continue working on that branch.
> **Never** make code changes directly on `main` branch.

- **Never commit directly to `main`.**
- Always create a new feature branch before starting any development task:
  ```bash
  git checkout -b feat/<short-description>
  # or
  git checkout -b fix/<short-description>
  ```
- Branch naming convention: use `feat/`, `fix/`, `refactor/`, `test/`, `docs/` prefixes.

### 11.2 Pre-commit Validation
Before marking any task as complete, always run the following in order:

```bash
# 1. Lint — all warnings treated as errors
cargo clippy -- -D warnings

# 2. Tests — all tests must pass
cargo test

# 3. Formatting check
cargo fmt --check
```

Do not proceed if `clippy` reports errors or any test fails. Fix issues first.

### 11.3 Documentation Sync
After completing a development task, review and update the following files as needed:

- **`README.md`** — update if any of the following changed:
  - Commands, options, or their behavior
  - Query operators or supported functions
  - Environment variables
  - Tech stack or dependencies
  - Project structure

- **`AGENTS.md`** — update if any of the following changed:
  - Data schema (fields, types, indexes)
  - Implementation details (algorithms, module responsibilities)
  - Performance goals or constraints
  - Development status (move items from "Future Improvements" to "Completed ✅")

### 11.4 Commit Message Convention
Use conventional commits format:
```
<type>(<scope>): <short summary>

# Examples:
feat(query): add exists() function support
fix(scanner): handle symlink cycles in walkdir
refactor(db): simplify upsert_document params
test(compiler): add coverage for nested JSON paths
docs(readme): update query operator table
```

### 11.5 Definition of Done

A task is not complete until all of the following have been performed — not just checked mentally, but actually executed:

- [ ] Branch is up to date with `main`
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo test` — all tests pass
- [ ] `cargo fmt --check` passes
- [ ] `README.md` updated if user-facing behavior changed
- [ ] `AGENTS.md` updated if architecture or schema changed

### 11.6 Testing Strategy
The goal is tests that verify **meaningful behavior**, not tests that merely execute code paths.

**When tests are required:**
- Every new feature must be accompanied by unit tests covering its core behavior and key edge cases.
- Every bug fix must include a regression test that reproduces the original bug.
- Any change to a public function signature must be followed by a review of existing tests to confirm they still meaningfully validate the intended behavior.

**What a good test verifies:**
- The correct output or side effect for a given input, not just "it didn't panic".
- Boundary conditions and error paths, not only the happy path.
- Behavior that would be broken by a wrong implementation — if a test would pass even with a naive or incorrect version of the code, it has no value.

**What to avoid:**
- Tests written solely to increase coverage numbers.
- Tests that duplicate what another test already covers without adding new signal.
- Overly brittle tests that break on minor refactors unrelated to the behavior being tested.

### 11.7 Rust Best Practices

These are points where agent-written Rust code commonly goes wrong, beyond what `clippy` catches automatically.

**Error handling:**
- Define structured error types with `thiserror` rather than returning bare `Box<dyn Error>` from library functions. Reserve `Box<dyn Error>` only for top-level `main` or quick prototypes.
- Never use `.unwrap()` or `.expect()` in non-test code. Propagate errors with `?` or handle them explicitly with a clear rationale.
- Error messages should describe what failed and why, not just the error type (e.g. `"failed to open database at {path}: {source}"` not just `"io error"`).

**Dependencies:**
- Before adding a new crate, check whether the existing stack already covers the need. This project is binary-size sensitive (`strip = true`).
- Prefer crates already in `Cargo.toml` over introducing new ones for marginal convenience.
- Always add new crates via `cargo add <crate>` rather than editing `Cargo.toml` directly. This ensures the version is current, `Cargo.lock` is updated atomically, and features are validated immediately. For crates with required features, use `cargo add <crate> --features <feature>`.

**Code style:**
- Keep functions small and focused. If a function needs a comment to explain what a block does, that block is a candidate for extraction.
- Avoid premature optimization. Correctness and clarity first; optimize only when there is a measured performance problem.

### 11.8 Code Reuse

Before implementing new functionality, always check whether an existing module can be extended rather than duplicating logic or adding unnecessary files.

**Respect module boundaries:**
- Each module owns a specific responsibility. Adding logic that belongs to module A into module B to avoid a refactor is the most common source of long-term debt. If the boundary needs to move, move it explicitly.
- If an existing module needs to be extended to support a new use case, prefer adding a well-named method to that module over working around it from the outside.

**Before adding new code, ask:**
- Does this logic already exist somewhere? Search before writing.
- Does this belong in an existing module, or does it genuinely warrant a new one?
- If creating a new module, does it have a single, clearly statable responsibility?

**New CLI subcommands:**
- Follow the existing pattern in `main.rs` for argument parsing.
- Implement the command's logic in a dedicated file, not inline in `main.rs`.

### 11.9 CLI UX Principles

**Output structure:**
- Default output should provide a structured summary sufficient for the user to confirm what happened — counts, status, and any non-fatal warnings — without requiring `--verbose`.
- `--verbose` is for process detail: per-file status, intermediate steps, generated SQL, and similar diagnostic information.
- Never emit content-free confirmations like `"Done!"` or `"Success!"` on their own. If there is nothing meaningful to report, stay silent.

**Output targets:**
- Query results and structured data → stdout (so they can be piped).
- Warnings, errors, and diagnostic messages → stderr.
- This separation ensures `markbase query "..." | jq` and similar pipelines work correctly.

**Exit codes:**
- Exit `0` only on full success.
- Exit non-zero on any error that prevented the command from completing its intended effect.
- Non-fatal warnings (e.g. skipped files) should not cause a non-zero exit, but must be reported on stderr.

**Summary format reference:**
```
# index — default output
Indexing ./notes...
  ✓ 142 files indexed  (3 new, 5 updated, 0 errors)  [1.2s]
  ⚠ Skipped: notes/broken.md — invalid frontmatter (line 4)

# query — default output
path                      mtime
────────────────────────  ───────────────────
./notes/task-a.md         2025-01-10 09:00:00
./notes/task-b.md         2025-01-12 14:30:00

2 results
```