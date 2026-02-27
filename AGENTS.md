# Agent Specification: Markdown Base CLI

## 1. Project Overview

The goal is to build a high-performance Command Line Interface (CLI) tool designed to scan, parse, and index Markdown files into a **DuckDB** database for instantaneous metadata searching with Obsidian compatibility in mind.

## 2. Technical Stack

See [README.md](./README.md#tech-stack) for the complete tech stack details.

## 3. Data Schema

The DuckDB local file (`.mdb/mdb.duckdb`) utilizes the following schema to support Markdown structures:

| Property   | Type        | Description                              |
| ---------- | ----------- | ---------------------------------------- |
| path       | TEXT        | Primary key - full file path             |
| folder     | TEXT        | Directory path                           |
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

### Command: `new`
- Creates a markdown note at the given path under base-dir.
- If `--template` is provided, copies and renders the named template from `templates/` under base-dir.

## 5. Module Responsibilities

> For actual implementation, read the source files directly under `src/`. This section describes each module's responsibilities and key design decisions that are not self-evident from the code.

- **`main.rs`** — CLI entry point using clap derive macros. Handles argument parsing and dispatches to the appropriate command handler. Database path and base-dir can be overridden via environment variables (`MDB_DATABASE`, `MDB_BASE_DIR`); CLI args take priority.

- **`scanner.rs`** — Drives the `index` command. Walks the directory tree, compares `mtime` for incremental updates, orchestrates calls to `extractor.rs` and `db.rs`, and computes backlinks as a reverse-lookup pass *after* all documents are inserted.

- **`extractor.rs`** — Stateless parsing of a single file's content. Extracts frontmatter (via `gray_matter`), wiki-links, embeds, and tags using regex. Tags are merged from both content (`#tag`) and frontmatter (`tags:` field). Returns a `Document` struct; has no knowledge of the database.

- **`db.rs`** — All DuckDB interaction. Owns the connection and schema initialization. Uses `INSERT OR REPLACE` for upserts. Row values are accessed via `duckdb::types::Value` — be aware that column index order must match the schema definition exactly.

- **`creator.rs`** — Handles the `new` command. Resolves the template path under `templates/` in base-dir, copies it, and substitutes template variables. Fails explicitly if the target file already exists.

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
- Note creation with templates (new command)
- Single equals operator (=) support
- Simplified field resolution (no file.*/note.* namespaces)
- Frontmatter conflict warnings (reserved fields except tags)

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
| `compiler.rs`  | SQL generation, field resolution, all operators                     |
| `extractor.rs` | Frontmatter, tags, wiki-links, embeds, edge cases                   |
| `db.rs`        | Database operations, queries, CRUD                                  |
| `scanner.rs`   | File scanning, indexing, backlinks, subdirectories                  |
| `query/mod.rs` | Output formatting (table, JSON, list)                               |
| `creator.rs`   | Template resolution, file creation                                  |
| `main.rs`      | CLI options, default values, parsing                                |

## 11. Development Workflow (Agent Guidelines)

### 11.1 Branch Policy
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

# 3. Formatting check (optional but recommended)
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

### 11.5 Pull Request Checklist
Before opening a PR, confirm:
- [ ] Branch is up to date with `main`
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo test` — all tests pass
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