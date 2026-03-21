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

markbase query "author == 'Tom'"
markbase query "SELECT file.path, file.name FROM notes WHERE list_contains(file.tags, 'todo')"
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MARKBASE_BASE_DIR` | Vault directory | `.` (current directory) |
| `MARKBASE_INDEX_LOG_LEVEL` | Automatic indexing output (`off`, `summary`, `verbose`) | `off` |
| `MARKBASE_COMPUTE_BACKLINKS` | Compute `file.backlinks` during automatic indexing | disabled |

**Priority:** CLI args > Environment variables > Defaults

```bash
export MARKBASE_BASE_DIR=/path/to/notes
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
| `file.backlinks` | VARCHAR[] | Notes linking to this note (reverse of links); empty unless backlinks computation is enabled |
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

### Indexing Scope

Indexing walks the vault recursively under `MARKBASE_BASE_DIR`.

- Only files with an extension are indexed
- Markdown files (`.md`) are parsed for frontmatter, tags, links, and embeds
- Non-Markdown files with an extension, including `.base`, are still indexed as resources, but they do not contribute parsed Markdown fields
- Dot-prefixed hidden files and directories are skipped by default
- Root `.gitignore` and `.markbaseignore` patterns are applied during indexing
- When both ignore files match the same path, `.markbaseignore` is applied after `.gitignore`

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

### `query`

Query notes in your vault.

**Two input modes:**

```bash
# Expression mode (WHERE clause only)
markbase query "note.author == 'Tom'"          # frontmatter (explicit)
markbase query "author == 'Tom'"               # frontmatter (shorthand)
markbase query "file.mtime > '2024-01-01'"     # file metadata
markbase query "list_contains(file.tags, 'project')"  # file array field
markbase query "author == 'Tom' ORDER BY file.mtime DESC LIMIT 10"

# Backlinks are disabled by default to keep indexing fast
markbase query "list_contains(file.backlinks, 'source')"
markbase --compute-backlinks query "list_contains(file.backlinks, 'source')"

# SQL mode (full SELECT statement)
markbase query "SELECT file.path, note.author FROM notes WHERE note.author = 'Tom'"
```

`file.backlinks` is empty unless backlinks computation is enabled with
`--compute-backlinks` or `MARKBASE_COMPUTE_BACKLINKS`.

Default columns for empty input or expression mode: `file.path`, `file.name`, `description`, `file.mtime`, `file.size`, `file.tags`.

**Output formats:**
- default output is `json`, optimized for agents and scripts
- `-o table` renders compact Markdown tables for humans

```bash
markbase query "SELECT file.name, title FROM notes" -o table
```

```md
| file.name | title |
| --- | --- |
| readme | README |
| todo | Todo List |
```

```bash
markbase query "SELECT file.name, title, file.tags FROM notes"
```

```json
[
  {
    "file.name": "readme",
    "title": "README",
    "file.tags": ["documentation", "important"]
  },
  {
    "file.name": "todo",
    "title": "Todo List",
    "file.tags": ["todo", "work"]
  }
]
```

Empty results stay machine-friendly:
- default `json` prints `[]`
- `-o table` prints just the header row and separator

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

Without a template, `markbase note new` creates a Markdown note in `base-dir/inbox` with a default frontmatter field: `description: 临时笔记`.

```bash
markbase note new my-note                    # Create in base-dir/inbox
markbase note new my-note --template daily   # Create in base-dir/inbox if template has no location
markbase note new customer --template company # Create in _schema.location if template defines one
```

`name` must be a pure note name: no directory components and no file extension.

On success, `markbase note new` prints only the note path relative to `base-dir`.

**Rename a note:**

```bash
markbase note rename old-name new-name
```

Behavior:
- `old-name` and `new-name` must be names only (no path components)
- Looks up note by name (not path)
- Fails if name is ambiguous or new name exists
- Updates all `[[old-name]]` links and `![[old-name]]` embeds across the vault (body and frontmatter)
- Preserves aliases, section anchors, and block IDs
- Normalizes rewritten Markdown-note targets to path-free, extension-free form such as `[[folder/old.md#Section]] -> [[new#Section]]`
- Preserves table-safe escaped separators such as `[[old-note\|Alias]] -> [[new-note\|Alias]]`
- Skips fenced code blocks and inline code spans when rewriting body links
- Reindexes the vault immediately after the rename completes

Extensions are allowed when renaming resource-style files such as `aaa.jpeg`; the forbidden part is the path, not the suffix.

**Resolve one or more entity names to notes:**

```bash
markbase note resolve "acme"
markbase note resolve "张伟" "阿里"
```

Outputs JSON by default for agent-friendly entity alignment. Each input returns `query`, `status`, and `matches`.

Each resolve input is a path-free query string, never a path or file-style name with an extension.

Statuses:
- `exact` — one note matched by `file.name`
- `alias` — one note matched by frontmatter `aliases`
- `name_contains_query` — one note matched because `file.name` contains the query
- `query_contains_name` — one note matched because the query contains `file.name`
- `multiple` — more than one candidate matched; disambiguate before linking
- `missing` — no exact name, alias, or partial-name candidate matched

Each match includes `name`, `path`, `type`, `description`, and `matched_by`. Missing descriptions are emitted as `null`, not omitted.

`matched_by` can be `name`, `alias`, `name_contains_query`, or `query_contains_name`.

Matching is case-insensitive for exact name, alias, and partial `file.name` resolution. Returned `name` and `path` keep their indexed casing.

Match priority is deterministic:
- `name`
- `alias`
- `name_contains_query`
- `query_contains_name`

Partial matching applies only to `file.name`, never to frontmatter `aliases`.

A single `exact`, `alias`, `name_contains_query`, or `query_contains_name` result is still only a low-cost alignment hint: compare `description` and context before reusing the note. If the description is clearly about a different thing, prefer creating a new note instead of forcing reuse.

**Verify a note against its template schema:**

```bash
markbase note verify <name>
```

`<name>` must be a note name only: no path and no file extension.

Checks that the note conforms to all constraints defined in its referenced MTS template(s), and also runs a global `description` check before template validation:
- Global frontmatter `description` exists, is a string, and is not blank (reported as ERROR)
- Referenced template frontmatter must parse successfully as YAML, or verification fails
- Targets that resolve to `templates/<name>.md` are rejected as template files rather than verified as note instances
- Directory location matches `_schema.location`
- Required frontmatter fields are present
- Field types and enum values are correct
- Link fields must be a single pure Obsidian wikilink such as `[[note]]` or `[[folder/note.md#Heading|Alias]]`
- Link fields point to notes of the expected `type`
- Template Markdown body `.base` embeds must also appear in the note body, so required embedded views are not dropped from instances
- Embedded `.base` targets in the Markdown body must exist in the indexed vault; missing or unreadable `.base` targets are reported as errors after the rest of verification continues

`note verify` no longer treats template outer-frontmatter seed values as literal-match requirements. `_schema.create` defines creation-time defaults, while continuing invariants must be modeled through `_schema.required` and `_schema.properties`. In practice, stable identity fields such as `type` should be declared in both places, while mutable seed fields such as `status` may evolve after creation as long as they still satisfy the schema.

Verification issues are reported to stderr. For issue output, the header includes `file.path`, and each schema-related issue includes a compact `Definition:` line so agents can repair notes with the expected type/constraints. Exit code is non-zero whenever verification produces any `ERROR`; dangling link references remain `INFO` and do not fail the command by themselves.

**Render a note (expand note and `.base` embeds):**

```bash
markbase note render <n>            # Markdown with embedded JSON blocks (default)
markbase note render <n> -o table    # Markdown tables for embedded Base views
markbase note render <n> --dry-run   # show SQL without executing
```

`<n>` must be either a note name (no extension) or a `.base` filename, never a path.

Rendering a Markdown note prints its body to stdout and scans normal Markdown
body content for live embed tokens:

- `![[note]]` and `![[note|Alias]]` expand to the embedded note's rendered body
- embedded note frontmatter is stripped; only the body is emitted
- note embeds with heading or block selectors such as `![[note#Heading]]` stay
  literal output for now
- `![[tasks.base]]` and `![[tasks.base#Open Tasks]]` render Base views at that
  token position
- non-Markdown, non-`.base` embeds are passed through unchanged

Inline note embeds are block-oriented. `Before![[note]]After` renders as
`Before`, then the embedded note body, then `After` on separate lines. The same
recursive render rules apply inside embedded note bodies, so nested note embeds
and nested `.base` embeds continue to expand.

Recursive note rendering is cycle-safe. If an embed would revisit a note that
is already on the active render stack, markbase warns on stderr and leaves a
placeholder comment in stdout instead of recursing forever. Missing embedded
notes are also soft failures: render continues after emitting a warning and a
placeholder comment. Unreadable embedded notes behave the same way and emit a
read-failure warning plus placeholder instead of aborting the whole render.

When a nested `.base` embed runs inside an embedded note body, `this` is bound
to the embedded note currently being rendered, not the original top-level note.

`![[tasks.base#Open Tasks]]` renders only the matching view. If the view does
not exist, markbase warns on stderr and leaves an HTML comment placeholder at
that line in stdout. Fenced code blocks and inline code spans are never treated
as live `.base` embeds, even if they contain the same syntax literally.

If a `.base` embed appears inline with surrounding text, markbase expands the
embed and keeps the surrounding text in output rather than requiring the embed
to occupy the entire line by itself.

If a live note or `.base` embed appears inside a blockquote or callout body,
markbase preserves that quote container line-by-line during expansion,
including blank lines and nested quote depth. List items remain outside the
supported live-embed container contract: note and `.base` embeds inside list
items stay literal output, even when quote or callout syntax appears on the
same logical line.

For `-o table`, each rendered Base view becomes a compact Markdown table:

```md
<!-- start: [markbase] rendered from tasks.base -->

> **Open Tasks**

| name | priority |
| --- | --- |
| [[task-a]] | high |
| [[task-b]] | medium |

<!-- end: [markbase] rendered from tasks.base -->
```

By default, the same view is wrapped in a JSON code fence so agents can parse it directly from the rendered Markdown:

````md
<!-- start: [markbase] rendered from tasks.base -->

> **Open Tasks**

```json
[
  {
    "name": "[[task-a]]",
    "priority": "high"
  },
  {
    "name": "[[task-b]]",
    "priority": "medium"
  }
]
```
<!-- end: [markbase] rendered from tasks.base -->
````

Supported filters: `link(this)`, `link("name")`, `file.hasLink(this.file)`,
`file.hasTag()`, `file.inFolder()`, date comparisons, `isEmpty()`, `contains()`.

When a frontmatter scalar stores a pure wikilink such as `company: "[[acme]]"`,
Base equality filters compare by the normalized target name, so
`company == this.file.name` and `company == link(this)` both match.

Warnings (unsupported filters, missing embedded notes, missing base files) go to stderr.
Exit code is non-zero only on hard errors (e.g. note not found).

### `web`

Initialize the supported docsify shell, serve browser routes, or inspect the
final web Markdown body.

```bash
markbase web init-docsify --homepage /HOME.md
markbase web serve                           # listen on 127.0.0.1:3000
markbase web serve --bind 127.0.0.1 --port 4000
markbase web serve --cache-control "public, max-age=60"
markbase web get /entities/person/alice.md  # print final web Markdown body
```

`markbase web init-docsify --homepage <canonical-url>` writes `index.html` into
the base-dir root. `--homepage` is required, and an existing `index.html` is
left untouched unless `--force` is provided.

`markbase web serve` is the user-facing browser entrypoint. It requires
`base-dir/index.html` to exist and will refuse to start until the docsify shell
has been initialized. Once initialized:

- requesting `/` returns `index.html`
- requesting `/index.html` returns the same docsify shell
- the shell keeps internal `.md` and `.base` document links inside docsify
- binary resource URLs such as images and attachments continue to resolve
  directly

By default, `web serve` returns `Cache-Control: no-store, no-cache,
must-revalidate` plus matching legacy no-cache headers on every response. Pass
`--cache-control <value>` to override that header for all responses served by
the process.

Web routing is path-based and derived from indexed `file.path`, but internal
rendering still resolves Markdown notes and `.base` targets by name. The
canonical note or resource URL is always `/<file.path>` with browser-safe
percent-encoding.

Each `web serve` request refreshes the index before route resolution and uses a
request-scoped DuckDB handle. For Markdown notes and direct `.base` targets,
the server returns docsify/marked-renderable Markdown rather than an HTML
shell. For binary resources, it returns raw bytes with the corresponding
`Content-Type`.

The server-side Markdown pipeline:

- reuses note-render semantics for recursive `![[note]]` expansion, `.base`
  expansion, soft-failure placeholders, and quote-container preservation
- rewrites `[[note]]` links to canonical path-based Markdown links
- rewrites non-Markdown `![[...]]` resource embeds to standard Markdown images
  or links
- removes `%%comment%%` from normal Markdown body content
- preserves fenced code blocks and inline code spans literally
- leaves unresolved wikilinks, unresolved resource embeds, selector-based note
  embeds, and block-target note embeds as literal source text in v1

`markbase web get <canonical-url>` prints the same Markdown body that
`web serve` returns for a Markdown note or `.base` route. If the canonical URL
resolves to a binary resource, `web get` exits with an explanatory failure
instead of streaming bytes.

HTTP miss and bad-path behavior:

- route miss returns `404 Not Found`
- invalid percent-decoding returns `400 Bad Request`

### `template`

Manage MTS templates.

```bash
markbase template list            # JSON (default, agent-first)
markbase template list -o table   # Compact Markdown table
markbase template describe daily  # Show normalized template content
```

Templates are stored in `templates/` under base-dir. `template describe` shows the normalized template view used by the CLI, including `_schema.create` and auto-injected `description` schema/default fields when older templates omit them. For new templates, author note-creation defaults under `_schema.create` and let `markbase note new --template` inject the `templates` field automatically:

```yaml
_schema:
  description: 用于匹配客户公司资料的模板
  location: company/
  required:
    - description
    - type
  properties:
    description:
      type: text
      description: 一句话说明这个 note 是什么
    type:
      type: text
      enum: [company]
  create:
    description: ""
    type: company
    tags: []
```

Here, `_schema.description` is the template routing prompt, `_schema.properties.description` is the schema definition for the instance field, and `_schema.create.description` is the concrete value written into new notes. Created notes receive `templates: ["[[<template-name>]]"]` from the CLI; template authors should not hand-write that field in the template.

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
