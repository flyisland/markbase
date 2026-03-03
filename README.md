# Markdown Base CLI (markbase)

A high-performance CLI tool for indexing and querying Markdown notes, designed for both AI agents and human users with Obsidian compatibility in mind.

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/flyisland/markbase)

## Installation

```bash
git clone <repository-url>
cd markbase
cargo build --release
./target/release/markbase --help
```

**Prerequisites:** Rust 1.85+ (DuckDB is bundled)

## Quick Start

```bash
export MARKBASE_BASE_DIR=/path/to/your/notes

markbase index
markbase query "author == 'Tom'"
markbase query "SELECT path, name FROM notes WHERE list_contains(tags, 'todo')"
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MARKBASE_BASE_DIR` | Vault directory | `.` (current directory) |
| `MARKBASE_OUTPUT_FORMAT` | Output format for query/template list | `table` |

**Priority:** CLI args > Environment variables > Defaults

```bash
export MARKBASE_BASE_DIR=/path/to/notes
export MARKBASE_OUTPUT_FORMAT=json

markbase index
markbase query "list_contains(tags, 'design')"
```

## Concepts

### Note Properties

Each indexed note has two property types:

**Reserved Fields** (native metadata):
| Field | Type | Description |
|-------|------|-------------|
| `path` | TEXT | File path relative to base-dir |
| `folder` | TEXT | Directory path relative to base-dir |
| `name` | TEXT | File name without extension |
| `ext` | TEXT | File extension |
| `size` | INTEGER | File size in bytes |
| `ctime` | TIMESTAMPTZ | Created time |
| `mtime` | TIMESTAMPTZ | Modified time |
| `tags` | VARCHAR[] | Tags from content (`#tag`) and frontmatter |
| `links` | VARCHAR[] | Wiki-links `[[link]]` + embeds `![[embed]]` from body and frontmatter |
| `backlinks` | VARCHAR[] | Notes linking to this note (reverse of links) |
| `embeds` | VARCHAR[] | Embeds `![[embed]]` from body only |

**Frontmatter Properties**: Any YAML frontmatter field is queryable:

```yaml
---
title: My Note
author: John
status: in-progress
---
```

Query it directly: `markbase query "author == 'John'"`

### Field Resolution

Reserved fields are checked first. If a frontmatter field conflicts with a reserved field (except `tags`), it's ignored with a warning during indexing.

### Name Uniqueness

Note names must be unique across the entire vault, regardless of their directory location.

- **Index**: When indexing, if two notes have the same name (different paths), a warning is shown and the duplicate is skipped
- **Create**: Creating a note fails if a note with that name already exists
- **Rename**: Renaming a note fails if a note with the target name already exists

### Link Format (Obsidian Style)

Always use the **filename only** — no path, no extension:

```markdown
# ✅ Correct
[[中国移动]]
[[张三]]

# ❌ Wrong
[[entities/中国移动.md]]
[[people/张三]]
```

Wiki-links in **frontmatter properties** must additionally be wrapped in quotes:

```yaml
# ✅ Correct
related_customer: "[[中石油]]"
attendees_internal: ["[[张三]]", "[[李四]]"]

# ❌ Wrong
related_customer: [[中国移动]]
attendees_internal: [[[张三]], [[李四]]]
```

## Commands

### `index`

Index Markdown notes to DuckDB.

```bash
markbase index              # Index base directory
markbase index --force      # Rebuild from scratch
markbase index -v           # Verbose output
```

Features:
- Incremental updates (skips unchanged files)
- Detects and removes deleted files
- Obsidian-compatible (wiki-links, embeds, frontmatter, tags)

### `query`

Query indexed notes.

**Two input modes:**

```bash
# Expression mode (WHERE clause only)
markbase query "author == 'Tom'"
markbase query "list_contains(tags, 'project')"
markbase query "author == 'Tom' ORDER BY mtime DESC LIMIT 10"

# SQL mode (full SELECT statement)
markbase query "SELECT path, name, author FROM notes WHERE author = 'Tom'"
```

**Output formats:**

```bash
markbase query "list_contains(tags, 'todo')" -o json
markbase query "list_contains(tags, 'todo')" -o list
```

**Debug:**

```bash
markbase query --dry-run "author == 'Tom'"  # Show translated SQL
```

**Type casts for non-string comparisons:**

```bash
markbase query "year::INTEGER >= 2024"
markbase query "created::TIMESTAMP > '2024-01-01'"
```

### `note`

Create and manage notes.

**Create a note:**

```bash
markbase note new my-note                    # Create in base-dir
markbase note new notes/my-note              # Create in subdirectory
markbase note new my-note --template daily   # Use template
```

**Rename a note:**

```bash
markbase note rename old-name new-name
```

Behavior:
- Looks up note by name (not path)
- Fails if name is ambiguous or new name exists
- Updates all `[[old-name]]` links and `![[old-name]]` embeds across the vault (body and frontmatter)
- Preserves aliases, section anchors, and block IDs

### `template`

Manage MKS templates.

```bash
markbase template list                  # List templates (table format)
markbase template list -o json          # JSON format
markbase template list -F "tags,type"   # Additional fields
markbase template describe daily        # Show template content
```

Templates are stored in `templates/` under base-dir.

## Query Syntax

markbase translates field names (reserved fields and frontmatter properties) to DuckDB queries. All DuckDB SQL keywords and operators are supported natively.

**Commonly Used Functions:**
- `list_contains(field, value)` - Array containment

**Examples:**

```bash
markbase query "folder == './notes'"
markbase query "mtime > '2024-01-01'"
markbase query "size > 10000"
markbase query "list_contains(tags, 'todo')"
markbase query "author == 'John' AND status == 'active'"
markbase query "author IS NOT NULL"
markbase query "name LIKE '%meeting%'"
```

## License

MIT
