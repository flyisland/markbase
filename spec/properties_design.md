# Properties Design Specification

**Status:** Stable / Production Ready  
**Date:** 2026-03-04  
**Target System:** markbase CLI

---

## 1. Overview

Markbase stores two types of properties for each indexed note:

1. **Reserved Fields** — Native metadata columns in the database
2. **Frontmatter Properties** — Custom YAML fields stored in a JSON column

This document defines the complete properties system, including storage, resolution, and query translation.

---

## 2. Database Schema

### 2.1 Table Definition

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

## 3. Reserved Fields

Reserved fields are native database columns with optimized storage and indexing.

### 3.1 Field Reference

| Field | Type | Index | Description |
|-------|------|-------|-------------|
| `path` | TEXT | Yes (PK) | File path relative to base-dir |
| `folder` | TEXT | Yes | Directory path relative to base-dir |
| `name` | TEXT | Yes | File name without extension |
| `ext` | TEXT | No | File extension |
| `size` | INTEGER | No | File size in bytes |
| `ctime` | TIMESTAMPTZ | No | Creation time |
| `mtime` | TIMESTAMPTZ | Yes | Modification time |
| `tags` | VARCHAR[] | No | Tags from content (`#tag`) and frontmatter |
| `links` | VARCHAR[] | No | Wiki-links `[[link]]` + embeds `![[embed]]` from body and frontmatter |
| `backlinks` | VARCHAR[] | No | Notes linking to this note (reverse of links) |
| `embeds` | VARCHAR[] | No | Embeds `![[embed]]` from body only |

### 3.2 Implementation

Reserved fields are defined in `src/query/detector.rs`:

```rust
pub fn is_reserved_field(field: &str) -> bool {
    matches!(
        field,
        "path" | "folder" | "name" | "ext" | "size" |
        "ctime" | "mtime" | "tags" | "links" | "backlinks" | "embeds"
    )
}
```

---

## 4. Frontmatter Properties

### 4.1 Storage

All YAML frontmatter fields are extracted during indexing and stored in the `properties` JSON column.

**Example note:**

```yaml
---
title: Project Alpha
author: John
status: in-progress
year: 2024
aliases: [Alpha, Project-A]
---
```

**Stored as:**

```json
{
  "title": "Project Alpha",
  "author": "John",
  "status": "in-progress",
  "year": 2024,
  "aliases": ["Alpha", "Project-A"]
}
```

> **Note:** Type casting (`::INTEGER`, `::TIMESTAMP`, etc.) is a query-time operation, not a storage format. Users specify casts in queries like `year::INTEGER >= 2024` to instruct DuckDB how to interpret the stored string value.

### 4.2 Conflict Resolution

Reserved fields take precedence. If a frontmatter field conflicts with a reserved field (except `tags`), it's ignored with a warning during indexing:

```rust
// scanner.rs
if is_reserved_field(key) && key != "tags" {
    eprintln!(
        "⚠ {}: frontmatter field '{}' conflicts with a reserved field and will be ignored.",
        path, key
    );
}
```

### 4.3 Nested Properties

Nested YAML structures are stored as nested JSON objects:

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

---

## 5. Field Resolution

When querying, fields are resolved in this priority order:

1. **Reserved fields** — Direct column access
2. **Frontmatter properties** — Extracted from JSON

### 5.1 Resolution Algorithm

```
function resolve_field(field_name):
    if is_reserved_field(field_name):
        return field_name  // Direct column access
    
    if field_name contains '.':
        // Nested: a.b.c → json_extract_string(properties, '$."a"."b"."c"')
        return translate_to_json_path(field_name)
    
    // Frontmatter: author → json_extract_string(properties, '$."author"')
    return json_extract_string(properties, '$."field_name"')
```

---

## 6. Query Translation

### 6.1 Translation Rules

| Input | Translated Output |
|-------|------------------|
| `author == 'Tom'` | `json_extract_string(properties, '$."author"') == 'Tom'` |
| `_schema.strict` | `json_extract_string(properties, '$."_schema"."strict"')` |
| `list_contains(categories, 'work')` | `list_contains((properties->'$."categories"')::VARCHAR[], 'work')` |
| `list_contains(tags, 'todo')` | `list_contains(tags, 'todo')` |
| `year::INTEGER >= 2024` | `json_extract_string(properties, '$."year"')::INTEGER >= 2024` |
| `author IS NOT NULL` | `json_extract_string(properties, '$."author"') IS NOT NULL` |

### 6.2 Reserved Field Handling in `list_contains`

Reserved array fields (`tags`, `links`, `backlinks`, `embeds`) pass through directly:

```sql
-- Reserved field
list_contains(tags, 'todo') 
→ list_contains(tags, 'todo')

-- Frontmatter array field
list_contains(categories, 'work')
→ list_contains((properties->'$."categories"')::VARCHAR[], 'work')
```

### 6.3 Type Casting

The translator preserves type cast syntax (`::TYPE`) and does not auto-insert casts. Users must explicitly cast:

```sql
-- User writes
year::INTEGER >= 2024

-- Translated
json_extract_string(properties, '$."year"')::INTEGER >= 2024
```

---

## 7. Query Modes

### 7.1 Expression Mode

Non-SELECT input is treated as an expression:

```
author == 'Tom' and year::INTEGER >= 2024
→
SELECT path, name, mtime, size, tags FROM notes
WHERE json_extract_string(properties, '$."author"') == 'Tom'
  AND json_extract_string(properties, '$."year"')::INTEGER >= 2024
```

### 7.2 SQL Mode

SELECT statements are translated field-by-field:

```
SELECT path, author, mtime FROM notes WHERE author = 'Tom'
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
| `Column not found` | `Error: unknown field '...', if this is a frontmatter field check for typos` |
| `Invalid json path` | `Error: invalid nested property path '...', check the syntax e.g. _schema.strict` |

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

---

## 11. Module Architecture

```
query/
├── mod.rs          # Output formatting (table/json/list)
├── detector.rs     # Query mode detection, reserved field list, safety validation
├── translator.rs   # Field translation logic
├── executor.rs     # Query execution and error wrapping
└── error_map.rs    # DuckDB error → user-friendly message mapping
```

---

## 12. Appendix: Reserved Fields vs Frontmatter

| Aspect | Reserved Fields | Frontmatter Properties |
|--------|-----------------|------------------------|
| Storage | Native columns | JSON column (`properties`) |
| Indexing | Optimized indexes | None |
| Query syntax | Direct | `json_extract_string()` |
| Type casting | Native | User-managed |
| Array handling | Native `list_contains()` | Cast to `::VARCHAR[]` |
| Conflict handling | N/A (takes precedence) | Warn and ignore |

---

## 13. Migration Notes

### Removed Fields

- `content` field has been removed from schema. Agents requiring file content should read the file directly.

### Deprecated Functions

- `has(field, value)` → replaced with `list_contains(field, value)`
- `exists(field)` → replaced with `field IS NOT NULL`
