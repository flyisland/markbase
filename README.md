# Markdown Base CLI (mdb)

A high-performance CLI tool for indexing and querying Markdown files with DuckDB. Obsidian-compatible.

[![Rust](https://img.shields.io/badge/Rust-1.85+-orange?logo=rust)](https://rust-lang.org)
[![DuckDB](https://img.shields.io/badge/DuckDB-1.4+-yellow?logo=duckdb)](https://duckdb.org)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/flyisland/mdb)

## Installation

### From Source

```bash
# Clone the repository
git clone <repository-url>
cd mdb

# Build release binary
cargo build --release

# The binary will be at target/release/mdb
./target/release/mdb --help
```

### Prerequisites

- Rust 1.85+ (2024 edition)
- DuckDB (bundled with the `duckdb` crate)

## Quick Start

```bash
# Index notes
mdb index --base-dir ./my-notes

# Query notes
mdb query "has(tags, 'todo')"
```

## Properties

Every indexed markdown file has two types of properties: native file metadata and frontmatter properties.

**Field Resolution**: Reserved fields are checked first, then frontmatter properties.

### Reserved Fields (Native Properties)

| Field | Type | Description |
|-------|------|-------------|
| `path` | TEXT | Full file path (primary key) |
| `folder` | TEXT | Directory path |
| `name` | TEXT | File name (without extension) |
| `ext` | TEXT | File extension (e.g., `md`) |
| `size` | INTEGER | File size in bytes |
| `ctime` | TIMESTAMP | Created time |
| `mtime` | TIMESTAMP | Modified time |
| `content` | TEXT | Full file content |
| `tags` | VARCHAR[] | Array of `#tags` (from content AND frontmatter) |
| `links` | VARCHAR[] | Array of `[[wiki-links]]` |
| `backlinks` | VARCHAR[] | Files linking to this file |
| `embeds` | VARCHAR[] | Array of `![[embeds]]` |

```bash
# Query reserved fields
mdb query "folder == './notes'"
mdb query "mtime > '2024-01-01'"
mdb query "size > 10000"
mdb query "has(tags, 'todo')"
mdb query "has(links, 'target-page')"
```

### Frontmatter Properties

Properties defined in YAML frontmatter are also available:

```yaml
---
title: My Note
author: John
category: project
status: in-progress
tags: [design, research]
date: 2024-01-15
---
```

```bash
# Query frontmatter properties (resolved automatically)
mdb query "author == 'John'"
mdb query "category == 'project'"
mdb query "status == 'in-progress'"
mdb query "has(tags, 'design')"
```

**Note:** If a frontmatter field conflicts with a reserved field (except `tags`), a warning will be shown during indexing and the frontmatter value will be ignored.

### Property Types

| Frontmatter Type | Query Example |
|-----------------|---------------|
| String | `author == 'John'` |
| Number | `year >= 2024` |
| Boolean | `published == true` |
| Array | `has(tags, 'design')` |
| Date | `date > '2024-01-01'` |
| Exists | `exists(author)` |

## Commands

### `index`
Scans Markdown files and indexes to DuckDB.

```bash
mdb index --base-dir ./notes        # Index base directory
mdb index --base-dir ./notes --force     # Force re-index
mdb index --base-dir ./notes -v     # Verbose
```

### `query`
Query indexed files with SQL-like expressions.

```bash
# Query reserved fields
mdb query "has(tags, 'project')"
mdb query "folder =~ '%projects%'"
mdb query "mtime > '2024-01-01'"
mdb query "size > 1000"

# Query frontmatter properties
mdb query "category == 'work'"
mdb query "author == 'John'"

# Nested properties
mdb query "_schema.strict == 'true'"

# Output formats
mdb query "has(tags, 'todo')" -o json
mdb query "has(tags, 'todo')" -o list
# Query results include count in output (table/list) or metadata (JSON)

# Select fields (default: path, mtime)
mdb query "name == 'readme'" -F "path,name,size"
mdb query "category == 'project'" -F "path,author,category"
```

### `new`
Create a new markdown note with optional template.

```bash
mdb new my-note                    # Create note in base-dir
mdb new notes/my-note              # Create in subdirectory
mdb new my-note --template daily   # Create with template (outputs path + content)
```

**With template:** Returns `path:` and `content:` for agent workflow integration:

```
path: /home/user/notes/today.md
content: ---
date: ""
mood: ""
summary: ""
tags: []
---

## 今日记录

---
```

### `template`
Manage templates (MKS schema-based templates).

MKS (Markdown Knowledge Schema) is a protocol for connecting unstructured conversation flow with structured knowledge bases. See [spec/schema.md](./spec/schema.md) for the complete specification.

```bash
mdb template list                  # List all templates (default: table format)
mdb template list -o json          # List in JSON format
mdb template list -o list         # List in list format
mdb template list -F "tags,type"  # List with additional fields
mdb template describe daily        # Show template content
```

**Note:** Templates are expected in the `templates/` directory under base-dir. Default fields shown: `name`, `_schema.description`, `path`.

**Fields:** Reserved fields (`path`, `folder`, `name`, `ext`, `size`, `ctime`, `mtime`, `content`, `tags`, `links`, `backlinks`, `embeds`) and frontmatter properties (e.g., `author`, `category`). Nested properties supported (e.g., `_schema.strict`).

**Operators:** `==`, `!=`, `>`, `<`, `>=`, `<=`, `=~` (LIKE), `and`, `or`

**Functions:** `has(field, value)` - array containment | `exists(field)` - property existence check

**Note:** Reserved fields are checked first, then frontmatter properties. If a frontmatter field conflicts with a reserved field (except `tags`), a warning will be shown during indexing.

**Note:** Timestamps are displayed in human-readable format (YYYY-MM-DD HH:MM:SS)

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MDB_DATABASE` | Path to DuckDB database | `.mdb/mdb.duckdb` |
| `MDB_BASE_DIR` | Base directory for indexing | `.` |
| `MDB_OUTPUT` | Output format for query and template list | `table` |

**Priority:** CLI arguments > Environment variables > Defaults

```bash
# Set environment variables
export MDB_DATABASE=/path/to/db.duckdb
export MDB_BASE_DIR=/path/to/notes
export MDB_OUTPUT=json

# Use environment variables
mdb query "has(tags, 'design')"

# CLI arguments override environment variables
mdb --database /other/db.duckdb query "..."
mdb index --base-dir /other/dir
mdb --output-format json query "..."
```

## Features

- Fast indexing with DuckDB
- SQL-like query language
- Obsidian support (wiki-links, embeds, frontmatter, tags)
- Incremental updates
- File watching mode for auto-reindexing
- Multiple output formats (table, json, list)
- Human-readable timestamps
- Shorthand field notation for conciseness
- Note creation with templates
- Template listing with MKS schema support

## Development

```bash
# Build debug version
cargo build

# Run in development
cargo run -- index --base-dir ./notes
cargo run -- query "file.name == 'readme'"

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Build release
cargo build --release

# Run with verbose output
cargo run -- index --base-dir ./notes -v
```

## Testing

The project includes comprehensive unit tests covering all major components:

- **127 total tests** across all modules
- **Query System**: Tokenizer, parser, compiler, and SQL generation
- **Content Extraction**: Frontmatter, tags, wiki-links, embeds
- **Database**: CRUD operations, queries, and filtering
- **Scanner**: File discovery, indexing, and backlink tracking
- **Watcher**: File monitoring and incremental indexing
- **Output**: Table, JSON, and list formatting

Run tests with: `cargo test`

## Tech Stack

- **Language:** Rust 1.85+ (2024 edition)
- **CLI Framework:** clap v4.5 (derive feature)
- **Database:** DuckDB via `duckdb` crate (bundled feature)
- **File Discovery:** walkdir v2.5
- **Parser:** gray_matter (frontmatter), regex (wiki-links/tags)
- **Serialization:** serde, serde_json

## Project Structure

```
mdb/
├── Cargo.toml           # Rust dependencies and metadata
├── Cargo.lock           # Dependency lock file
├── README.md            # User documentation
├── AGENTS.md            # This file - agent specification
├── src/
│   ├── main.rs          # CLI entry point with clap
│   ├── db.rs            # DuckDB database operations
│   ├── scanner.rs       # File discovery and indexing
│   ├── extractor.rs     # Markdown content extraction
│   ├── creator.rs       # Note creation with templates
│   ├── describe.rs      # Template description
│   ├── lib.rs           # Library exports
│   └── query/           # Query system
│       ├── mod.rs       # Output formatting (table/json/list)
│       ├── tokenizer.rs # Query tokenization
│       ├── parser.rs    # AST parsing
│       └── compiler.rs  # SQL compilation
└── target/              # Build output
```

## License

MIT
