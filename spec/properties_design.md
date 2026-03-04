# Properties Design Specification

**Status:** Stable / Production Ready  
**Date:** 2026-03-04  
**Target System:** markbase CLI

---

## 1. Overview

Markbase stores two types of properties for each indexed note:

1. **File Properties** — Native metadata columns in the database, accessed via the `file.` prefix
2. **Note Properties** — Custom YAML frontmatter fields stored in a JSON column, accessed via the `note.` prefix (or no prefix as shorthand)

This design aligns with the [Obsidian Bases property system](https://help.obsidian.md/bases/syntax#Properties), where `file.*` and `note.*` are distinct namespaces. The separation eliminates naming collisions: `file.name` (the filename) and `note.name` (a frontmatter field named `name`) can now coexist without ambiguity.

---

## 2. Database Schema

### 2.1 Table Definition

The underlying schema is unchanged. The `file`/`note` prefix distinction is a query-layer concern only — the database columns remain the same.

```sql
CREATE TABLE IF NOT EXISTS notes (
    path       TEXT PRIMARY KEY,
    folder     TEXT NOT NULL,
    name       TEXT NOT NULL,
    ext        TEXT NOT NULL,
    size       INTEGER NOT NULL,
    ctime      TIMESTAMPTZ NOT NULL,
    mtime      TIMESTAMPTZ NOT NULL,
    tags       VARCHAR[],
    links      VARCHAR[],
    backlinks  VARCHAR[],
    embeds     VARCHAR[],
    properties JSON
)
```

**Indexes:**
```sql
CREATE INDEX idx_mtime ON notes(mtime);
CREATE INDEX idx_folder ON notes(folder);
CREATE INDEX idx_name ON notes(name);
```

---

## 3. Property Namespaces

### 3.1 Overview

There are two property namespaces, selected by prefix:

| Prefix | Namespace | Resolves to |
|--------|-----------|-------------|
| `file.` | File properties | Native database columns |
| `note.` | Note (frontmatter) properties | `properties` JSON column |
| *(none)* | Shorthand for `note.` | `properties` JSON column |

**Key principle:** A bare identifier with no prefix (e.g. `author`) is always treated as `note.author` — a shorthand for the frontmatter namespace. There is no implicit fallback to file properties for unqualified names.

### 3.2 File Properties

File properties are native database columns representing filesystem and vault metadata. They must be accessed with the `file.` prefix.

| Field | Type | Index | Description |
|-------|------|-------|-------------|
| `file.path` | TEXT | Yes (PK) | File path relative to base-dir |
| `file.folder` | TEXT | Yes | Directory path relative to base-dir |
| `file.name` | TEXT | Yes | File name without extension |
| `file.ext` | TEXT | No | File extension |
| `file.size` | INTEGER | No | File size in bytes |
| `file.ctime` | TIMESTAMPTZ | No | Creation time |
| `file.mtime` | TIMESTAMPTZ | Yes | Modification time |
| `file.tags` | VARCHAR[] | No | Tags from content (`#tag`) and frontmatter |
| `file.links` | VARCHAR[] | No | Wiki-links `[[link]]` + embeds `![[embed]]` from body and frontmatter |
| `file.backlinks` | VARCHAR[] | No | Notes linking to this note (reverse of links) |
| `file.embeds` | VARCHAR[] | No | Embeds `![[embed]]` from body only |

### 3.3 Note Properties

Note properties are YAML frontmatter fields stored in the `properties` JSON column. They are accessed via the `note.` prefix, or with no prefix as shorthand.

```
note.author       ← explicit
author            ← shorthand, identical meaning
note._schema.strict  ← nested, explicit
_schema.strict       ← nested, shorthand
```

Nested YAML structures use dot notation: `note.a.b.c` maps to `json_extract_string(properties, '$."a"."b"."c"')`.

### 3.4 Implementation

File property detection is defined in `src/query/detector.rs`:

```rust
pub fn is_file_property(field: &str) -> bool {
    matches!(
        field,
        "file.path" | "file.folder" | "file.name" | "file.ext" | "file.size" |
        "file.ctime" | "file.mtime" | "file.tags" | "file.links" |
        "file.backlinks" | "file.embeds"
    )
}

/// Strips the "note." prefix if present; bare identifiers are returned unchanged.
/// Both "note.author" and "author" refer to the same frontmatter field.
pub fn note_field_key(field: &str) -> &str {
    field.strip_prefix("note.").unwrap_or(field)
}
```

---

## 4. Note Properties Storage

### 4.1 Storage Format

All YAML frontmatter fields are extracted during indexing and stored in the `properties` JSON column.

**Example note:**

```yaml
---
title: Project Alpha
author: John
status: in-progress
year: 2024
aliases: [Alpha, Project-A]
name: custom-name      # frontmatter field named "name"
---
```

**Stored as:**

```json
{
  "title": "Project Alpha",
  "author": "John",
  "status": "in-progress",
  "year": 2024,
  "aliases": ["Alpha", "Project-A"],
  "name": "custom-name"
}
```

With the prefix system, `file.name` accesses the filename column (e.g. `"project-alpha"`) while `note.name` (or bare `name`) accesses the frontmatter field (e.g. `"custom-name"`). There is no collision and no warning is needed for this case.

> **Note:** Type casting (`::INTEGER`, `::TIMESTAMP`, etc.) is a query-time operation, not a storage format. Users specify casts in queries like `note.year::INTEGER >= 2024` to instruct DuckDB how to interpret the stored string value.

### 4.2 Conflict Resolution

Because `file.*` and `note.*` are fully separate namespaces, there are no naming collisions between file properties and frontmatter fields. The `tags` field is a special case: frontmatter `tags` are **merged** into `file.tags` during indexing (same behavior as before). A frontmatter field named `tags` does not produce a warning.

No other frontmatter field produces a warning due to a naming conflict, since the namespaces are now disjoint.

### 4.3 Nested Properties

Nested YAML structures are stored as nested JSON objects and accessed with dot notation under the `note.` namespace:

```yaml
---
_schema:
  strict: true
  required: [title, author]
---
```

**Stored as:**

```json
{
  "_schema": {
    "strict": true,
    "required": ["title", "author"]
  }
}
```

**Queried as:**

```
note._schema.strict     (explicit)
_schema.strict          (shorthand)
```

---

## 5. Field Resolution

### 5.1 Resolution Algorithm

```
function resolve_field(token):
    if token starts with "file.":
        column = token after "file." prefix
        return column  // Direct column access

    if token starts with "note.":
        key = token after "note." prefix
        return translate_to_json_path(key)

    // Bare identifier — shorthand for note.*
    // Only reached if the token is not a SQL keyword or literal
    return translate_to_json_path(token)


function translate_to_json_path(key):
    if key contains '.':
        // Nested: a.b.c → json_extract_string(properties, '$."a"."b"."c"')
        segments = key.split('.')
        path = segments.map(s => '"' + s + '"').join('.')
        return 'json_extract_string(properties, \'$.' + path + '\')'

    // Simple: author → json_extract_string(properties, '$."author"')
    return 'json_extract_string(properties, \'$."' + key + '"\')'
```

### 5.2 Ambiguity-Free Design

Because all file properties require an explicit `file.` prefix, the translator never needs to guess whether a bare identifier is a file property or a note property. A bare identifier is always a note property (shorthand for `note.*`). This makes the resolution rule simple and deterministic with no priority ordering required.

---

## 6. Query Translation

### 6.1 Translation Rules

| User input | Translated SQL |
|------------|---------------|
| `file.name == 'readme'` | `name == 'readme'` |
| `file.mtime > '2024-01-01'` | `mtime > '2024-01-01'` |
| `file.folder == './notes'` | `folder == './notes'` |
| `author == 'Tom'` | `json_extract_string(properties, '$."author"') == 'Tom'` |
| `note.author == 'Tom'` | `json_extract_string(properties, '$."author"') == 'Tom'` |
| `_schema.strict` | `json_extract_string(properties, '$."_schema"."strict"')` |
| `note._schema.strict` | `json_extract_string(properties, '$."_schema"."strict"')` |
| `note.year::INTEGER >= 2024` | `json_extract_string(properties, '$."year"')::INTEGER >= 2024` |
| `year::INTEGER >= 2024` | `json_extract_string(properties, '$."year"')::INTEGER >= 2024` |
| `author IS NOT NULL` | `json_extract_string(properties, '$."author"') IS NOT NULL` |
| `note.author IS NOT NULL` | `json_extract_string(properties, '$."author"') IS NOT NULL` |

### 6.2 Array Fields and `list_contains`

File array fields (`file.tags`, `file.links`, `file.backlinks`, `file.embeds`) pass through to their native columns:

```sql
-- File array field (native column)
list_contains(file.tags, 'todo')
→ list_contains(tags, 'todo')

-- Note array field (frontmatter, cast required)
list_contains(note.categories, 'work')
→ list_contains((properties->'$."categories"')::VARCHAR[], 'work')

-- Bare shorthand (same as note.categories)
list_contains(categories, 'work')
→ list_contains((properties->'$."categories"')::VARCHAR[], 'work')
```

### 6.3 Type Casting

The translator preserves type cast syntax (`::TYPE`) and does not auto-insert casts. The cast attaches to the translated expression, not the original identifier:

```sql
-- User writes (either form is equivalent)
note.year::INTEGER >= 2024
year::INTEGER >= 2024

-- Translated
json_extract_string(properties, '$."year"')::INTEGER >= 2024
```

---

## 7. Query Modes

### 7.1 Expression Mode

Non-SELECT input is treated as an expression (WHERE clause and optional trailing clauses). Default output fields use `file.*` prefixes internally but are displayed with their plain column names:

```
file.folder == './notes' and year::INTEGER >= 2024
→
SELECT path, name, mtime, size, tags FROM notes
WHERE folder == './notes'
  AND json_extract_string(properties, '$."year"')::INTEGER >= 2024
```

```
author == 'Tom' ORDER BY file.mtime DESC
→
SELECT path, name, mtime, size, tags FROM notes
WHERE json_extract_string(properties, '$."author"') == 'Tom'
ORDER BY mtime DESC
```

### 7.2 SQL Mode

SELECT statements are translated field-by-field:

```
SELECT file.path, note.author, file.mtime FROM notes WHERE note.author = 'Tom'
→
SELECT path,
       json_extract_string(properties, '$."author"'),
       mtime
FROM notes
WHERE json_extract_string(properties, '$."author"') = 'Tom'
```

---

## 8. Security

### 8.1 Allowed Operations

Only `SELECT` statements are permitted. The following are blocked:

- `INSERT`, `UPDATE`, `DELETE`, `DROP`, `CREATE`, `ALTER`
- Multi-statement injection (semicolon-separated)

### 8.2 Validation

```rust
pub fn validate_safety(sql: &str) -> Result<(), String> {
    let upper = sql.trim().to_uppercase();

    if !upper.starts_with("SELECT") {
        return Err("Error: query command only supports SELECT statements");
    }

    if sql.contains(';') {
        let parts: Vec<&str> = sql.split(';').filter(|s| !s.trim().is_empty()).collect();
        if parts.len() > 1 {
            return Err("Error: multiple statements are not allowed");
        }
    }

    Ok(())
}
```

---

## 9. Error Mapping

DuckDB errors are translated to user-friendly messages:

| DuckDB Error | User Message |
|--------------|--------------|
| `Conversion Error` | `Error: cannot convert value '...' for field '...', expected type is ...` |
| `Column not found` | `Error: unknown field '...', use file.<field> for file properties or note.<field> for frontmatter` |
| `Invalid json path` | `Error: invalid nested property path '...', check the syntax e.g. _schema.strict or note._schema.strict` |

---

## 10. Default Query Fields

When no fields are specified in expression mode, the default SELECT includes:

```sql
SELECT path, name, mtime, size, tags FROM notes
```

This is defined in `src/query/translator.rs`:

```rust
const DEFAULT_FIELDS: &str = "path, name, mtime, size, tags";
```

The default fields use raw column names (no prefix) since they are emitted directly into SQL without going through the translator.

---

## 11. Module Architecture

```
query/
├── mod.rs          # Output formatting (table/json/list)
├── detector.rs     # Query mode detection, file property list, safety validation
├── translator.rs   # Field translation logic (file.* and note.* prefix handling)
├── executor.rs     # Query execution and error wrapping
└── error_map.rs    # DuckDB error → user-friendly message mapping
```

---

## 12. Appendix: File Properties vs Note Properties

| Aspect | File Properties (`file.*`) | Note Properties (`note.*` or bare) |
|--------|----------------------------|-------------------------------------|
| Storage | Native columns | JSON column (`properties`) |
| Indexing | Optimized indexes | None |
| Query prefix | Required (`file.`) | Optional (`note.` or bare shorthand) |
| Query syntax | Direct column access | `json_extract_string()` |
| Type casting | Native | User-managed (`::TYPE`) |
| Array handling | Native `list_contains()` | Cast to `::VARCHAR[]` |
| Naming collision | N/A (separate namespace) | N/A (separate namespace) |
| Example | `file.name == 'readme'` | `note.author == 'Tom'` or `author == 'Tom'` |

---

## 13. Migration Notes

### Breaking Changes from Previous Design

The previous design used a single flat namespace where "reserved fields" were identified by name and took precedence over frontmatter fields. This is replaced by explicit namespace prefixes:

| Old syntax | New syntax | Notes |
|------------|-----------|-------|
| `name == 'readme'` | `file.name == 'readme'` | `name` without prefix now means `note.name` |
| `mtime > '2024-01-01'` | `file.mtime > '2024-01-01'` | All file metadata requires `file.` prefix |
| `folder == './notes'` | `file.folder == './notes'` | All file metadata requires `file.` prefix |
| `list_contains(tags, 'todo')` | `list_contains(file.tags, 'todo')` | `tags` without prefix now means `note.tags` |
| `author == 'Tom'` | `author == 'Tom'` *(unchanged)* | Bare identifiers still resolve to note properties |
| `_schema.strict` | `_schema.strict` *(unchanged)* | Nested paths still resolve to note properties |

**Conflict warnings removed.** Because `file.*` and `note.*` are now separate namespaces, frontmatter fields no longer conflict with file properties (except `tags`, which continues to be merged into `file.tags`). The warning-and-ignore behavior from the previous design is no longer needed.

### Other Removed Items

- `content` field has been removed from schema. Agents requiring file content should read the file directly.

### Deprecated Functions

- `has(field, value)` → replaced with `list_contains(field, value)`
- `exists(field)` → replaced with `field IS NOT NULL`
