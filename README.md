# Markdown Base CLI (markbase)

A high-performance CLI tool for indexing and querying Markdown notes, designed for both AI agents and human users with Obsidian compatibility in mind.

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/flyisland/markbase)

## Installation

**From crates.io (recommended):**

```bash
cargo install markbase
```

**Build from source:**

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
markbase query "SELECT file.path, file.name FROM notes WHERE list_contains(file.tags, 'todo')"
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
markbase query "list_contains(file.tags, 'design')"
```

## Concepts

### Note Properties

Each indexed note has two namespaces for properties:

**File Properties** (`file.*` prefix):
Access native database columns representing file metadata:

| Field | Type | Description |
|-------|------|-------------|
| `file.path` | TEXT | File path relative to base-dir |
| `file.folder` | TEXT | Directory path relative to base-dir |
| `file.name` | TEXT | File name without extension |
| `file.ext` | TEXT | File extension |
| `file.size` | INTEGER | File size in bytes |
| `file.ctime` | TIMESTAMPTZ | Created time |
| `file.mtime` | TIMESTAMPTZ | Modified time |
| `file.tags` | VARCHAR[] | Tags from content (`#tag`) and frontmatter |
| `file.links` | VARCHAR[] | Wiki-links `[[link]]` + embeds `![[embed]]` from body and frontmatter |
| `file.backlinks` | VARCHAR[] | Notes linking to this note (reverse of links) |
| `file.embeds` | VARCHAR[] | Embeds `![[embed]]` from body only |

**Note Properties** (`note.*` prefix or bare):
Access YAML frontmatter fields:

```yaml
---
title: My Note
author: John
status: in-progress
---
```

Query using explicit prefix or bare shorthand:
```bash
markbase query "note.author == 'John'"    # explicit
markbase query "author == 'John'"         # shorthand (same result)
```

### Tags

Tags are extracted from two sources:

**Content tags** (`#tag` in note body):
- Obsidian format: `#` followed by alphanumeric characters, underscores, hyphens, and forward slashes
- Must contain at least one non-numerical character (e.g., `#1984` is invalid, `#y1984` is valid)
- Case-insensitive (e.g., `#tag` and `#TAG` are identical)
- Supports nested tags using `/` separator (e.g., `#project/2024/q1`)

**Frontmatter tags**:
- YAML list format: `tags: [tag1, tag2]` or `tags: [project/2024]`

All tags are merged into `file.tags` and can be queried with `list_contains(file.tags, 'tag-name')`.

### Field Resolution

| Syntax | Resolves To | Example |
|--------|-------------|---------|
| `file.*` | Native database column | `file.name` → `name` column |
| `note.*` | Frontmatter JSON extraction | `note.author` → `properties->"author"` |
| bare (no prefix) | Frontmatter JSON extraction (shorthand for `note.*`) | `author` → `properties->"author"` |

The `file.*` and `note.*` namespaces are completely separate — no naming conflicts.

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
markbase query "note.author == 'Tom'"          # frontmatter (explicit)
markbase query "author == 'Tom'"               # frontmatter (shorthand)
markbase query "file.mtime > '2024-01-01'"     # file metadata
markbase query "list_contains(file.tags, 'project')"  # file array field
markbase query "author == 'Tom' ORDER BY file.mtime DESC LIMIT 10"

# SQL mode (full SELECT statement)
markbase query "SELECT file.path, note.author FROM notes WHERE note.author = 'Tom'"
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
markbase query "note.year::INTEGER >= 2024"
markbase query "note.created::TIMESTAMP > '2024-01-01'"
# or using bare shorthand:
markbase query "year::INTEGER >= 2024"
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

Manage MTS templates.

```bash
markbase template list                  # List templates (table format)
markbase template list -o json          # JSON format
markbase template list -F "tags,type"   # Additional fields
markbase template describe daily        # Show template content
```

Templates are stored in `templates/` under base-dir.

## Query Syntax

markbase translates field names using explicit namespaces (`file.*` for file metadata, `note.*` or bare for frontmatter) to DuckDB queries. All DuckDB SQL keywords and operators are supported natively.

**Commonly Used Functions:**
- `list_contains(field, value)` - Array containment
  - `list_contains(file.tags, 'todo')` - file array field (native)
  - `list_contains(note.categories, 'work')` - frontmatter array (cast to VARCHAR[])

**Field Prefix Reference:**

| Prefix | Namespace | Use For | Example |
|--------|-----------|---------|---------|
| `file.` | File properties | Metadata columns | `file.name`, `file.mtime`, `file.size` |
| `note.` | Note properties | Frontmatter fields | `note.author`, `note.status` |
| (bare) | Note properties | Shorthand for `note.*` | `author`, `status` |

**Examples:**

```bash
# File metadata queries (require file.* prefix)
markbase query "file.folder == './notes'"
markbase query "file.mtime > '2024-01-01'"
markbase query "file.size > 10000"
markbase query "file.name LIKE '%meeting%'"
markbase query "list_contains(file.tags, 'todo')"

# Frontmatter queries (note.* prefix or bare)
markbase query "note.author == 'John'"
markbase query "author == 'John'"  # same as above
markbase query "note.status == 'active'"
markbase query "author IS NOT NULL"

# Combined queries
markbase query "author == 'John' AND file.mtime > '2024-01-01'"
markbase query "list_contains(file.tags, 'todo') AND status == 'active'"
```

## License

MIT
