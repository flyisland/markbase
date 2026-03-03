# Markbase Developer Guide

This document is intended for markbase developers and development agents, covering architecture design, development standards, and implementation details.

## 1. Project Overview

Markbase is a high-performance CLI tool for scanning, parsing, and indexing Markdown notes into a DuckDB database, enabling instant metadata queries with Obsidian compatibility.

**Core Value**: Provides structured Markdown knowledge base access for AI Agents.

## 2. Tech Stack

- **Language**: Rust 1.85+ (2024 edition)
- **CLI Framework**: clap v4.5 (derive feature)
- **Database**: DuckDB (bundled with `duckdb` crate)
- **File Traversal**: walkdir v2.5
- **Parsing**: gray_matter (frontmatter), regex (wiki-links/tags), serde_yaml (frontmatter rewrite)
- **Serialization**: serde, serde_json

**Design Principle**: Minimize dependencies, optimize binary size (`strip = true` in Cargo.toml)

## 3. Design Principles

**Design Principle 1 - Name Uniqueness**: Note names must be unique across the entire vault, regardless of directory location. This enables simple `[[note-name]]` linking without path ambiguity.

**Design Principle 2 - Obsidian Link Format**: Wiki-links use filename only (no path, no extension). Example: `[[my-note]]` not `[[notes/my-note.md]]`. Frontmatter links must be quoted: `related: "[[target]]"`.

## 4. Data Model

### 4.1 Schema

Database location: `{{base-dir}}/.markbase/markbase.duckdb`

```sql
CREATE TABLE notes (
    path       TEXT PRIMARY KEY,        -- Path relative to base-dir
    folder     TEXT,                    -- Directory path
    name       TEXT,                    -- File name without extension
    ext        TEXT,                    -- Extension
    size       INTEGER,                 -- Size in bytes
    ctime      TIMESTAMPTZ,             -- Creation time
    mtime      TIMESTAMPTZ,             -- Modification time
    tags       VARCHAR[],               -- Tag array
    links      VARCHAR[],               -- Wiki-links array
    backlinks  VARCHAR[],               -- Backlink array
    embeds     VARCHAR[],               -- Embed array
    properties JSON                     -- Frontmatter properties
);
```

**Indexes**:
```sql
CREATE INDEX idx_mtime ON notes(mtime);
CREATE INDEX idx_folder ON notes(folder);
CREATE INDEX idx_name ON notes(name);
```

### 4.2 Field Resolution Priority

When querying fields:
1. Check reserved fields first (`path`, `name`, `mtime`, etc.)
2. If not matched, extract from `properties` JSON: `json_extract_string(properties, '$."field"')`
3. Support nested paths: `a.b.c` → `json_extract_string(properties, '$."a"."b"."c"')`

## 5. Core Module Responsibilities

### 5.1 Module Overview

```
src/
├── main.rs          # CLI entry point, argument parsing and command dispatch
├── db.rs            # DuckDB connection management, schema initialization, CRUD
├── scanner.rs       # index command driver, directory traversal, incremental update, backlink computation
├── extractor.rs     # Single file parsing: frontmatter, wiki-links, tags
├── creator.rs       # note new command, template rendering
├── renamer.rs       # note rename command, link updates
├── describe.rs      # template describe command
├── lib.rs           # Library exports
└── query/
    ├── mod.rs       # Output formatting (table/json/list)
    ├── detector.rs  # SQL/expression mode detection, security validation
    ├── translator.rs # Field name translation
    ├── error_map.rs # DuckDB error mapping
    └── executor.rs  # Query execution orchestration
```

### 5.2 Key Design Decisions

**`scanner.rs`**:
- Uses WalkDir iterator for sequential processing (avoids memory spikes)
- Incremental update logic: compare `mtime + size`, skip unchanged files
- Deletion detection: after traversal, compare DB with filesystem, remove deleted entries
- Conflict handling: skip duplicate files with warning
- Backlinks: after all notes are inserted, perform a second traversal to compute backlinks

**`extractor.rs`**:
- Stateless parser, no database awareness
- Merges content tags (`#tag`) and frontmatter tags
- Extracts links from body (`[[...]]`, `![[...]]`) and frontmatter (`[[...]]`)
- Shared regex patterns (`EMBED_RE`, `WIKILINK_RE`) exposed as public constants
- Code blocks in body are excluded from link matching
- Returns `ExtractedContent` with `links`, `embeds`, `tags`, `frontmatter`

**`db.rs`**:
- Owns DuckDB connection, implements Drop trait to ensure closure
- Uses `INSERT OR REPLACE` for upsert
- Row value access via `duckdb::types::Value`, note that column order must match schema

**`query/detector.rs`**:
- Mode detection: starts with `SELECT` → SQL mode, otherwise → expression mode
- Security validation: reject non-SELECT statements, multi-statement injection

**`query/translator.rs`**:
- Reserved fields pass through directly
- Frontmatter fields translated to `json_extract_string(properties, ...)`
- Preserve type cast syntax (`::INTEGER`, `::TIMESTAMP`)

## 6. Command Internal Logic

For detailed usage, see README.md; this section explains implementation details.

### `index`

**Flow**:
1. Traverse directory tree (WalkDir)
2. For each `.md` file:
   - Compare `mtime + size` in DB, skip if unchanged
   - Call `extractor.rs` to parse content
   - Insert/update DB
3. Deletion detection: entries in DB but not in filesystem → delete
4. Conflict detection: files with same name → warn and skip
5. Backlink computation: traverse all notes' `links`, populate target note's `backlinks`
6. Commit transaction

**`--force` flag**: Delete `.markbase/markbase.duckdb` and reindex

### `query`

**Two input modes**:
- **Expression mode**: `author == 'Tom'` → `SELECT * FROM notes WHERE author == 'Tom'`
- **SQL mode**: `SELECT path FROM notes WHERE ...` → execute directly (field names only translated)

**Field translation**:
```sql
-- Expression: author == 'John'
-- Translates to:
SELECT * FROM notes WHERE json_extract_string(properties, '$."author"') = 'John'

-- SQL: SELECT name, author FROM notes WHERE author = 'John'
-- Translates to:
SELECT name, json_extract_string(properties, '$."author"') FROM notes WHERE json_extract_string(properties, '$."author"') = 'John'
```

**Special handling**:
- `list_contains(field, value)` uses `(properties->'$."field"')::VARCHAR[]` for frontmatter array fields

**Error mapping**: DuckDB errors converted to user-friendly messages (unknown field, type cast failure, etc.)

### `note rename`

**Flow**:
1. Find note by name (not path)
2. Uniqueness check: if file with same name exists → fail
3. Rename file
4. Full vault scan: update `[[old-name]]` and `![[old-name]]` in body and frontmatter
5. Preserve aliases, anchors, and block IDs: `[[old-name#Heading|alias]]` → `[[new-name#Heading|alias]]`
6. Reindex affected notes

## 7. Performance Targets

| Metric | Target |
|--------|--------|
| Cold start index 10,000 notes | < 5s |
| Complex query latency (10k rows) | < 50ms |
| Complex expression compilation | < 100ms |

**Optimization strategies**:
- Incremental updates avoid full scans
- Index optimization (mtime, folder, name)
- Binary size optimization (`strip = true`)

## 8. Constraints and Security

- **Single writer**: DuckDB constraint, only one `index` process can run at a time
- **Query security**: Only SELECT statements allowed, reject multi-statement injection
- **Error handling**: Comprehensive use of `Result` and `?` operator
- **Thread safety**: Use `Mutex<Database>` for multi-threaded scenarios
- **Graceful shutdown**: `Database` implements Drop trait

## 9. Development Commands

```bash
# Development build
cargo build

# Run
export MARKBASE_BASE_DIR=./notes
cargo run -- index
cargo run -- query "name == 'readme'"

# Test
cargo test
cargo test -- --nocapture

# Release build
cargo build --release

# Code checks
cargo clippy -- -D warnings
cargo fmt --check
```

## 10. Project Structure

```
markbase/
├── Cargo.toml           # Dependency configuration
├── Cargo.lock           # Dependency lock file
├── README.md            # User documentation
├── AGENTS.md            # This file
├── src/
│   ├── main.rs          # CLI entry point
│   ├── db.rs            # Database operations
│   ├── scanner.rs       # Index scanning
│   ├── extractor.rs     # Content extraction
│   ├── creator.rs       # Note creation
│   ├── renamer.rs       # Note renaming
│   ├── describe.rs      # Template description
│   ├── lib.rs           # Library exports
│   └── query/           # Query system
│       ├── mod.rs       # Output formatting
│       ├── detector.rs  # Mode detection
│       ├── translator.rs # Field translation
│       ├── error_map.rs # Error mapping
│       └── executor.rs  # Query execution
└── target/              # Build output
```

## 11. Development Status

### Completed ✅

- Core indexing functionality
- Query system (SQL mode + expression mode)
- Field translation and security validation
- Multiple output formats (table/json/list)
- Backlink tracking
- Incremental update and deletion detection
- Note creation (template support)
- Note renaming (link updates)
- Template management

### Technical Debt

- Integration test coverage
- Performance benchmarking (10k notes target)
- Parallel index processing
- Configuration file support
- Query result caching

### Test Coverage

| Module | Coverage Scope |
|--------|----------------|
| `detector.rs` | Mode detection, expression splitting, security validation |
| `translator.rs` | Field translation, reserved fields, nested paths, type casts, array handling |
| `error_map.rs` | DuckDB error mapping |
| `executor.rs` | Query execution, error wrapping |
| `extractor.rs` | Frontmatter, tags, links, embeds parsing |
| `db.rs` | CRUD operations, queries |
| `scanner.rs` | Scanning, indexing, backlinks, deletion detection, conflict handling |
| `query/mod.rs` | Output formatting |
| `creator.rs` | Template parsing, note creation |
| `renamer.rs` | Link updates, renaming |
| `main.rs` | CLI argument parsing |

## 12. Development Workflow

### 11.1 Branch Strategy

**Never develop directly on `main` branch**

Create feature branches:
```bash
git checkout -b feat/<description>
git checkout -b fix/<description>
```

Branch prefixes: `feat/`, `fix/`, `refactor/`, `test/`, `docs/`

### 11.2 Pre-commit Checks

**Must pass the following checks**:

```bash
cargo clippy -- -D warnings  # Lint
cargo test                   # Test
cargo fmt --check            # Format
```

Do not commit if any check fails.

### 11.3 Documentation Sync

**When to update README.md**:
- Commands, options, or behavior changes
- Query operators or function changes
- Environment variable changes

**When to update AGENTS.md**:
- Dependencies or tech stack changes
- Data model changes
- Architecture or algorithm changes
- Performance targets or constraint changes
- Development status changes

### 11.4 Commit Message Convention

```
<type>(<scope>): <summary>

# Examples
feat(query): add exists() function support
fix(scanner): handle symlink cycles in walkdir
refactor(db): simplify upsert_note params
test(compiler): add coverage for nested JSON paths
docs(readme): update query operator table
```

### 11.5 Definition of Done

Task completion requires:

- [ ] Branch synced with main
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` all pass
- [ ] `cargo fmt --check` passes
- [ ] User-visible behavior changes updated in README.md
- [ ] Architecture changes updated in AGENTS.md

## 13. Testing Strategy

**Test Value**: Verify meaningful behavior, not just code coverage

**Must write tests**:
- New feature → core behavior + key boundaries
- Bug fix → regression test
- Public API changes → review existing tests

**Good test characteristics**:
- Verify correct output or side effects
- Cover edge cases and error paths
- Breakable by incorrect implementations (naive implementations cannot pass)

**Avoid**:
- Writing tests solely for coverage
- Duplicating existing tests
- Being overly sensitive to unrelated refactors

## 14. Rust Best Practices

### 13.1 Error Handling

- Use `thiserror` to define structured error types
- No `.unwrap()` / `.expect()` in non-test code
- Error messages should explain failure reason: `"failed to open {path}: {source}"` not `"io error"`

### 13.2 Dependency Management

- Check existing dependencies before adding new ones
- Use `cargo add <crate>` instead of manually editing `Cargo.toml`
- Use `cargo add <crate> --features <feature>` when features are needed

### 13.3 Code Style

- Keep functions short and focused
- Consider extracting code blocks that need comments to explain
- Prioritize correctness, optimize based on measurements

## 15. Code Reuse

**Respect module boundaries**:
- Each module has clear responsibility
- Don't put logic in wrong module just to avoid refactoring
- Explicitly move when boundaries need to change

**Check before adding new functionality**:
- Does the logic already exist? Search first
- Does it belong to an existing module?
- Does the new module have a single clear responsibility?

**New CLI subcommands**:
- Follow existing `main.rs` patterns
- Implementation in separate files, not inline in `main.rs`

## 16. CLI UX Principles

### 15.1 Output Structure

- Default output provides structured summary (counts, status, warnings)
- `--verbose` for process details
- No uninformative confirmations (e.g., "Done!")

### 15.2 Output Targets

- Query results and structured data → stdout (supports piping)
- Warnings, errors, diagnostic info → stderr

### 15.3 Exit Codes

- Success → `0`
- Errors (affecting expected outcome) → non-zero
- Non-fatal warnings → `0` (but must report to stderr)

### 15.4 Output Examples

```
# index
Indexing ./notes...
  ✓ 142 files indexed  (3 new, 5 updated, 0 errors)  [1.2s]
  ⚠ Skipped: notes/broken.md — invalid frontmatter (line 4)

# query
path                      mtime
────────────────────────  ───────────────────
./notes/task-a.md         2025-01-10 09:00:00
./notes/task-b.md         2025-01-12 14:30:00

2 results
```
