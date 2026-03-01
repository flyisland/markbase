# Markdown Base CLI (markbase)

A high-performance CLI tool for indexing and querying Markdown notes for AI agent. Obsidian-compatible.

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/flyisland/markbase)

## Installation

### From Source

```bash
# Clone the repository
git clone <repository-url>
cd markbase

# Build release binary
cargo build --release

# The binary will be at target/release/markbase
./target/release/markbase --help
```

### Prerequisites

- Rust 1.85+ (2024 edition)
- DuckDB (bundled with the `duckdb` crate)

## Quick Start

```bash
export MARKBASE_BASE_DIR=./my-notes

# Index notes
markbase index

# Query notes
markbase query "has(tags, 'todo')"
```

## Properties

Every indexed markdown note has two types of properties: native note metadata and frontmatter properties.

**Field Resolution**: Reserved fields are checked first, then frontmatter properties.

### Reserved Fields (Native Properties)

| Field | Type | Description |
|-------|------|-------------|
| `path` | TEXT | File path relative to base-dir (primary key) |
| `folder` | TEXT | Directory path relative to base-dir |
| `name` | TEXT | File name (without extension) |
| `ext` | TEXT | File extension (e.g., `md`) |
| `size` | INTEGER | Note size in bytes |
| `ctime` | TIMESTAMP | Created time |
| `mtime` | TIMESTAMP | Modified time |
| `content` | TEXT | Full note content |
| `tags` | VARCHAR[] | Array of `#tags` (from content AND frontmatter) |
| `links` | VARCHAR[] | Array of `[[wiki-links]]` |
| `backlinks` | VARCHAR[] | Notes linking to this note |
| `embeds` | VARCHAR[] | Array of `![[embeds]]` |

```bash
# Query reserved fields
markbase query "folder == './notes'"
markbase query "mtime > '2024-01-01'"
markbase query "size > 10000"
markbase query "has(tags, 'todo')"
markbase query "has(links, 'target-page')"
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
markbase query "author == 'John'"
markbase query "category == 'project'"
markbase query "status == 'in-progress'"
markbase query "has(tags, 'design')"
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
Scans Markdown notes and indexes to DuckDB.

```bash
markbase index              # Index base directory
markbase index --force      # Delete database and rebuild from scratch
markbase index -v           # Verbose output
```

### `query`
Query indexed notes with SQL-like expressions.

```bash
# Query reserved fields
markbase query "has(tags, 'project')"
markbase query "folder =~ '%projects%'"
markbase query "mtime > '2024-01-01'"
markbase query "size > 1000"

# Query frontmatter properties
markbase query "category == 'work'"
markbase query "author == 'John'"

# Nested properties
markbase query "_schema.strict == 'true'"

# Output formats
markbase query "has(tags, 'todo')" -o json
markbase query "has(tags, 'todo')" -o list
# Query results include count in output (table/list) or metadata (JSON)

# Select fields (default: path, mtime)
markbase query "name == 'readme'" -F "path,name,size"
markbase query "category == 'project'" -F "path,author,category"
```

### `note`
Manage notes (create, rename).

#### `note new`
Create a new markdown note with optional template.

```bash
markbase note new my-note                    # Create note in base-dir
markbase note new notes/my-note              # Create in subdirectory
markbase note new my-note --template daily   # Create with template (outputs path + content)
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

#### `note rename`
Rename a note and update all wiki-links pointing to it.

```bash
markbase note rename old-name new-name
```

**Behavior:**
- Looks up note by name (not path)
- Fails if note name is ambiguous (multiple files with same name)
- Fails if new name already exists
- Updates all `[[old-name]]` links to `[[new-name]]` in all notes
- Preserves aliases and section anchors: `[[old-name|alias]]` → `[[new-name|alias]]`

**Example:**
```bash
markbase note rename architecture system-design
# Renames architecture.md → system-design.md
# Updates all [[architecture]] links to [[system-design]]
```

### `template`
Manage templates (MKS schema-based templates).

MKS (Markdown Knowledge Schema) is a protocol for connecting unstructured conversation flow with structured knowledge bases. See [spec/schema.md](./spec/schema.md) for the complete specification.

```bash
markbase template list                  # List all templates (default: table format)
markbase template list -o json          # List in JSON format
markbase template list -o list         # List in list format
markbase template list -F "tags,type"  # List with additional fields
markbase template describe daily        # Show template content
```

**Note:** Templates are expected in the `templates/` directory under base-dir. Default fields shown: `name`, `_schema.description`, `path`.

**Fields:** Reserved fields (`path`, `folder`, `name`, `ext`, `size`, `ctime`, `mtime`, `content`, `tags`, `links`, `backlinks`, `embeds`) and frontmatter properties (e.g., `author`, `category`). Nested properties supported (e.g., `_schema.strict`).

**Operators:** `==`, `!=`, `>`, `<`, `>=`, `<=`, `=~` (LIKE), `and`, `or`

**Functions:** `has(field, value)` - array containment | `exists(field)` - property existence check

**Note:** Reserved fields are checked first, then frontmatter properties. If a frontmatter field conflicts with a reserved field (except `tags`), a warning will be shown during indexing.

**Note:** Timestamps are displayed in human-readable format (YYYY-MM-DD HH:MM:SS)

## Environment Variables

`MARKBASE_BASE_DIR` is the **primary way** to configure your vault. Set it once in your shell profile (e.g., `~/.bashrc`, `~/.zshrc`) and forget it:

```bash
export MARKBASE_BASE_DIR=/path/to/your/notes
```

All `markbase` commands will use this directory by default. No need to pass `--base-dir` with every command.

| Variable | Description | Default |
|----------|-------------|---------|
| `MARKBASE_BASE_DIR` | Vault (base) directory for indexing | `.` (current directory) |
| `MARKBASE_OUTPUT_FORMAT` | Output format for query and template list | `table` |

**Priority:** CLI arguments > Environment variables > Defaults

```bash
export MARKBASE_BASE_DIR=/path/to/notes
export MARKBASE_OUTPUT_FORMAT=json

markbase index
markbase query "has(tags, 'design')"
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
export MARKBASE_BASE_DIR=./notes
cargo run -- index
cargo run -- query "name == 'readme'"

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Build release
cargo build --release

# Run with verbose output
cargo run -- index -v
```

## Testing

The project includes comprehensive unit tests covering all major components:

- **127 total tests** across all modules
- **Query System**: Tokenizer, parser, compiler, and SQL generation
- **Content Extraction**: Frontmatter, tags, wiki-links, embeds
- **Database**: CRUD operations, queries, and filtering
- **Scanner**: Note discovery, indexing, and backlink tracking
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
markbase/
├── Cargo.toml           # Rust dependencies and metadata
├── Cargo.lock           # Dependency lock file
├── README.md            # User documentation
├── AGENTS.md            # This file - agent specification
├── src/
│   ├── main.rs          # CLI entry point with clap
│   ├── db.rs            # DuckDB database operations
│   ├── scanner.rs       # Note discovery and indexing
│   ├── extractor.rs     # Markdown content extraction
│   ├── creator.rs       # Note creation with templates
│   ├── renamer.rs       # Note renaming with link updates
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
