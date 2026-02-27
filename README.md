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

**Shorthand Resolution**: Native columns are checked first, then frontmatter properties.

### File Namespace (Native Properties)

Use the `file.*` prefix to access native file metadata:

| Field | Type | Description |
|-------|------|-------------|
| `file.path` | TEXT | Full file path (primary key) |
| `file.folder` | TEXT | Directory path |
| `file.name` | TEXT | File name (without extension) |
| `file.ext` | TEXT | File extension (e.g., `md`) |
| `file.size` | INTEGER | File size in bytes |
| `file.ctime` | TIMESTAMP | Created time |
| `file.mtime` | TIMESTAMP | Modified time |
| `file.content` | TEXT | Full file content |
| `file.tags` | VARCHAR[] | Array of `#tags` in content |
| `file.links` | VARCHAR[] | Array of `[[wiki-links]]` |
| `file.backlinks` | VARCHAR[] | Files linking to this file |
| `file.embeds` | VARCHAR[] | Array of `![[embeds]]` |

```bash
# Query native file properties
mdb query "file.folder == './notes'"
mdb query "file.mtime > '2024-01-01'"
mdb query "file.size > 10000"
mdb query "has(file.tags, 'todo')"
mdb query "has(file.links, 'target-page')"
```

### Note Namespace (Frontmatter Properties)

Use `note.*` prefix to explicitly access frontmatter properties stored in YAML frontmatter:

### Frontmatter Example

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

### Querying Properties

Properties are accessed via the `note.*` namespace or shorthand:

```bash
# Using shorthand (native columns checked first, then frontmatter)
mdb query "author == 'John'"
mdb query "category == 'project'"
mdb query "status == 'in-progress'"
mdb query "name == 'readme'"

# Using explicit namespace
mdb query "note.author == 'John'"
mdb query "note.category == 'project'"
```

### Property Types

| Frontmatter Type | Query Example |
|-----------------|---------------|
| String | `author == 'John'` |
| Number | `year >= 2024` |
| Boolean | `published == true` |
| Array | `has(tags, 'design')` |
| Date | `date > '2024-01-01'` |
| Exists | `exists(author)` |

**Note:** Use explicit namespaces (`file.*`, `note.*`) when field names might conflict.

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
# Basic queries (shorthand - native columns and frontmatter properties)
mdb query "has(tags, 'project')"
mdb query "category == 'work'"
mdb query "folder =~ '%projects%'"
mdb query "mtime > '2024-01-01'"

# Explicit namespace usage (file.* for native columns, note.* for frontmatter)
mdb query "has(file.tags, 'todo')"
mdb query "note.author == 'John'"

# Output formats
mdb query "has(tags, 'todo')" -o json
mdb query "has(tags, 'todo')" -o list

# Select fields (default: file.path, file.mtime)
mdb query "name == 'readme'" -f "path,name,size"
mdb query "category == 'project'" -f "path,note.author,category"
```

### `new`
Create a new markdown note with optional template.

```bash
mdb new my-note                    # Create note in base-dir
mdb new notes/my-note              # Create in subdirectory
mdb new my-note --template daily   # Create with template
```

### `template`
Manage templates (MKS schema-based templates).

```bash
mdb template list                  # List all templates
mdb template list -f "tags,type"  # List with additional fields
```

**Note:** Templates are expected in the `templates/` directory under base-dir. Default fields shown: `name`, `_schema.description`, `path`.

**Fields:** Native columns (`path`, `folder`, `name`, `ext`, `size`, `ctime`, `mtime`, `content`, `tags`, `links`, `backlinks`, `embeds`) and frontmatter properties (e.g., `author`, `category`). Use `file.*` prefix for explicit namespace or shorthand for convenience.

**Operators:** `==`, `!=`, `>`, `<`, `>=`, `<=`, `=~` (LIKE), `and`, `or`

**Functions:** `has(field, value)` - array containment | `exists(field)` - property existence check

**Note:** Shorthand notation - native columns (path, folder, name, tags, etc.) resolve first, then frontmatter properties. Use explicit namespaces (`file.*`, `note.*`) when field names might conflict.

**Note:** Timestamps are displayed in human-readable format (YYYY-MM-DD HH:MM:SS)

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MDB_DATABASE` | Path to DuckDB database | `.mdb/mdb.duckdb` |
| `MDB_BASE_DIR` | Base directory for indexing | `.` |

**Priority:** CLI arguments > Environment variables > Defaults

```bash
# Set environment variables
export MDB_DATABASE=/path/to/db.duckdb
export MDB_BASE_DIR=/path/to/notes

# Use environment variables
mdb query "has(tags, 'design')"

# CLI arguments override environment variables
mdb --database /other/db.duckdb query "..."
mdb index -b /other/dir
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

- **102 total tests** across all modules
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
│   ├── watcher.rs       # File monitoring for watch mode
│   ├── creator.rs      # Note creation with templates
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
