# Technical Plan: Properties Namespace Migration (file.* / note.*)

**Document Status:** Draft  
**Target Version:** TBD  
**Spec Reference:** `spec/properties_design.md`

---

## 1. Overview

This document outlines the implementation plan for migrating from the old "reserved field" system to explicit `file.` and `note.` namespace prefixes, as defined in `spec/properties_design.md`.

### 1.1 Migration Summary

| Aspect | Old Behavior | New Behavior |
|--------|-------------|--------------|
| File metadata | Bare identifiers (`name`, `mtime`) | Required prefix (`file.name`, `file.mtime`) |
| Frontmatter | Bare identifiers (`author`) | Optional prefix (`note.author` or bare) |
| Namespace collision | Warning + ignore frontmatter | None (separate namespaces) |
| Array fields | `list_contains(tags, 'x')` | `list_contains(file.tags, 'x')` |

---

## 2. Affected Components

### 2.1 Source Code

| File | Changes | Complexity |
|------|---------|------------|
| `src/query/detector.rs` | Add `is_file_property()`, `note_field_key()`; deprecate `is_reserved_field()` | Medium |
| `src/query/translator.rs` | Rewrite `translate_identifier()` for prefix handling | High |
| `src/query/error_map.rs` | Update error messages | Low |

### 2.2 Test Suite

| File | Changes |
|------|---------|
| `src/query/detector.rs` (tests) | Add tests for new functions |
| `src/query/translator.rs` (tests) | Rewrite existing tests; add prefix tests |
| Integration tests | Add end-to-end tests for new syntax |

### 2.3 Specification Files

| File | Changes |
|------|---------|
| `spec/properties_design.md` | Mark as "Implemented" status |
| `AGENTS.md` | Update field resolution section (4.2, 5.2) |

### 2.4 Documentation

| File | Changes |
|------|---------|
| `README.md` | Update query syntax section, field resolution section |

---

## 3. Implementation Strategy

### 3.1 Phase 1: Core Logic Changes

#### Step 1.1: Update `detector.rs`

Add new functions alongside existing `is_reserved_field()`:

```rust
/// Returns true for file property prefixes: file.path, file.folder, etc.
pub fn is_file_property(field: &str) -> bool {
    matches!(
        field,
        "file.path" | "file.folder" | "file.name" | "file.ext" | "file.size" |
        "file.ctime" | "file.mtime" | "file.tags" | "file.links" |
        "file.backlinks" | "file.embeds"
    )
}

/// Strips "note." prefix if present; bare identifiers returned unchanged.
/// Both "note.author" and "author" refer to the same frontmatter field.
pub fn note_field_key(field: &str) -> &str {
    field.strip_prefix("note.").unwrap_or(field)
}
```

**Strategy:** Keep `is_reserved_field()` for backward compatibility during transition; remove after full migration.

#### Step 1.2: Update `translator.rs`

Rewrite `translate_identifier()` to handle three cases:

```
┌─────────────────────────────────────────────────────────────────────┐
│                        translate_identifier(word)                    │
├─────────────────────────────────────────────────────────────────────┤
│  if word.starts_with("file.") →                                     │
│      strip "file.", return as direct column                        │
│                                                                      │
│  if word.starts_with("note.") →                                     │
│      strip "note.", translate to json_extract_string(...)          │
│                                                                      │
│  # Bare identifier                                                  │
│  if is_sql_keyword(word) → pass through                            │
│  else → translate to json_extract_string(...) (frontmatter)        │
└─────────────────────────────────────────────────────────────────────┘
```

**Key changes to `translate_identifier()`:**

1. **Detect prefix:** Check for `file.` or `note.` prefix
2. **Strip prefix:** Extract the field key
3. **Route to translation:**
   - `file.*` → direct column access (existing reserved field logic)
   - `note.*` or bare → frontmatter JSON extraction
4. **Update `list_contains` handling:** 
   - `file.tags` → native column
   - `note.tags` or bare `tags` → cast to VARCHAR[]

### 3.2 Phase 2: Test Updates

#### Test Strategy

| Test Category | Approach |
|--------------|----------|
| Unit tests (new functions) | Test `is_file_property()` and `note_field_key()` |
| Unit tests (translation) | Rewrite failing tests; add new prefix tests |
| Integration tests | Add end-to-end tests with `file.*` and `note.*` prefixes |

#### 3.2.1 Existing Tests Impact Analysis

The current test suite has **21 tests** in `translator.rs` and **17 tests** in `detector.rs`. After the migration, some tests will fail because the old syntax behavior changes:

**Tests that MUST BE UPDATED (7 tests will fail after migration):**

| Test Name | Old Behavior | New Behavior Required |
|-----------|-------------|----------------------|
| `test_translate_reserved_field` | `name, mtime` → direct column | `file.name, file.mtime` → direct column |
| `test_translate_order_by` | `mtime` → direct column | `file.mtime` → direct column |
| `test_translate_list_contains_reserved_field` | `tags` → direct column | `file.tags` → direct column |
| `test_ignore_sql_keywords` | `tags` → direct column | `file.tags` → direct column |
| `test_build_select_sql_expression_with_suffix` | `mtime` → direct column | `file.mtime` → direct column |
| `test_build_select_sql_suffix_only` | `mtime` → direct column | `file.mtime` → direct column |
| `test_translate_complex_query` | `mtime` → direct column | `file.mtime` → direct column |

**Tests that REMAIN VALID (bare = frontmatter, unchanged):**

| Test Name | Reason |
|-----------|--------|
| `test_translate_frontmatter_field` | `author` → frontmatter (bare = note.) |
| `test_translate_where_clause` | `author` → frontmatter |
| `test_translate_nested_field` | `_schema.strict` → frontmatter |
| `test_translate_type_cast` | `year` → frontmatter |
| `test_translate_is_null` | `author` → frontmatter |
| `test_translate_list_contains_frontmatter_field` | `categories` → frontmatter |
| `test_translate_list_contains_nested_field` | `meta.categories` → frontmatter |

**Test Update Action Items:**

1. **Rewrite 7 failing tests** to use `file.` prefix for file metadata
2. **Add new tests** for explicit `file.*` and `note.*` prefix handling

> **Note:** The deprecated `has()` and `exists()` functions were already removed from the codebase in a previous migration. No test removal needed for these.

#### Test Files to Update

1. `src/query/detector.rs` (lines 147-334) - Add tests for new functions
2. `src/query/translator.rs` (lines 309-488) - Rewrite failing tests + add new tests

### 3.3 Phase 3: Specification Updates

#### 3.3.1 Update `AGENTS.md`

Sections requiring updates:

| Section | Current | New |
|---------|---------|-----|
| 4.2 Field Resolution Priority | "Check reserved fields first" | "Check for file. or note. prefix" |
| 5.2 Key Design Decisions - translator.rs | "Reserved fields pass through" | "File properties use file. prefix" |
| 5.2 Key Design Decisions - detector.rs | N/A | Add "file. prefix detection" |

#### 3.3.2 Update `spec/properties_design.md`

Change status header:
```diff
- **Status:** Stable / Production Ready
+ **Status:** Implemented
```

### 3.4 Phase 4: Documentation Updates

#### 3.4.1 Update `README.md`

| Section | Changes |
|---------|---------|
| Note Properties | Add `file.` prefix section; clarify bare identifiers → frontmatter |
| Field Resolution | Rewrite to explain namespaces |
| Query Syntax Examples | Update all examples to use `file.` prefix for file metadata |
| Query Syntax Table | Add prefix column; show both old and new syntax |

**Example changes:**

```diff
- # Expression mode (WHERE clause only)
- markbase query "author == 'Tom'"
- markbase query "list_contains(tags, 'project')"
- markbase query "author == 'Tom' ORDER BY mtime DESC LIMIT 10"

+ # Expression mode (WHERE clause only)
+ markbase query "note.author == 'Tom'"          # frontmatter
+ markbase query "author == 'Tom'"               # shorthand for note.author
+ markbase query "file.mtime > '2024-01-01'"    # file metadata
+ markbase query "list_contains(file.tags, 'project')"
+ markbase query "author == 'Tom' ORDER BY file.mtime DESC LIMIT 10"
```

---

## 4. Migration Path

### 4.1 Recommended Approach: Big Bang with Breaking Change Notice

Since this is a CLI tool with version tracking, we can make this a breaking change:

1. **Implement** all changes in a single PR/branch
2. **Add deprecation warnings** at compile time (Rust lint) for old syntax
3. **Update documentation** with migration guide
4. **Release as major version** (e.g., v2.0.0)

### 4.2 Alternative: Gradual Migration

If backward compatibility is needed:

1. Keep both `is_reserved_field()` and `is_file_property()`
2. If identifier matches `is_reserved_field()` but has no prefix:
   - Log deprecation warning
   - Translate as file property
3. Add config option `--new-property-syntax` to opt-in
4. Remove old behavior in next major version

**Recommendation:** Use big bang for simplicity (matching Obsidian Bases design).

---

## 5. Breaking Changes Summary

### 5.1 Query Syntax Changes

| Old Syntax | New Syntax | Notes |
|------------|-----------|-------|
| `name == 'readme'` | `file.name == 'readme'` | Required `file.` prefix |
| `mtime > '2024-01-01'` | `file.mtime > '2024-01-01'` | Required `file.` prefix |
| `folder == './notes'` | `file.folder == './notes'` | Required `file.` prefix |
| `list_contains(tags, 'todo')` | `list_contains(file.tags, 'todo')` | Required `file.` prefix for arrays |
| `author == 'Tom'` | `author == 'Tom'` | Unchanged (bare = frontmatter) |
| `_schema.strict` | `_schema.strict` | Unchanged (bare = frontmatter) |

### 5.2 Removed Warnings

- Frontmatter field conflicts with reserved fields no longer produce warnings
- `tags` frontmatter field merging behavior unchanged

### 5.3 Removed Features

- `has(field, value)` → Use `list_contains(field, value)`
- `exists(field)` → Use `field IS NOT NULL`

---

## 6. Verification Checklist

### 6.1 Pre-Implementation

- [ ] Review and approve this technical plan
- [ ] Create implementation branch
- [ ] Set up test fixtures

### 6.2 Implementation

- [ ] Update `detector.rs` with new functions
- [ ] Update `translator.rs` with prefix handling
- [ ] Update error messages in `error_map.rs` if needed

### 6.3 Testing

- [ ] Run existing tests (expect failures due to syntax change)
- [ ] Add unit tests for new functions
- [ ] Rewrite translator tests
- [ ] Run full test suite: `cargo test`
- [ ] Run lint: `cargo clippy -- -D warnings`
- [ ] Run format: `cargo fmt --check`

### 6.4 Documentation

- [ ] Update `spec/properties_design.md` status
- [ ] Update `AGENTS.md` sections
- [ ] Update `README.md` with new syntax examples

### 6.5 Pre-Release

- [ ] Update version in `Cargo.toml` (major version bump)
- [ ] Update CHANGELOG.md (if exists)
- [ ] Final build: `cargo build --release`

---

## 7. Dependencies and Risks

### 7.1 Dependencies

- No new dependencies required
- All changes are refactoring within existing query module

### 7.2 Risks

| Risk | Mitigation |
|------|------------|
| Missed test cases for edge cases | Extensive test coverage; manual testing |
| User confusion about new syntax | Clear documentation; deprecation warnings |
| Regression in query translation | Run full test suite; integration tests |

---

## 8. Estimated Effort

| Phase | Effort |
|-------|--------|
| Core logic (detector + translator) | 4-6 hours |
| Test updates | 2-3 hours |
| Documentation updates | 1-2 hours |
| Verification and polish | 2-3 hours |
| **Total** | **9-14 hours** |

---

## 9. Open Questions

1. **Version bump strategy:** Should this be v2.0.0 or v1.x.x with deprecation warnings?
2. **Migration support:** Should we provide a CLI flag to enable old syntax temporarily?
3. **Deprecation timeline:** How long should old syntax be warned before removal?

---

## Appendix A: File-by-File Change Details

### A.1 `src/query/detector.rs`

```diff
+ pub fn is_file_property(field: &str) -> bool {
+     matches!(
+         field,
+         "file.path" | "file.folder" | "file.name" | "file.ext" | "file.size" |
+         "file.ctime" | "file.mtime" | "file.tags" | "file.links" |
+         "file.backlinks" | "file.embeds"
+     )
+ }
+
+ pub fn note_field_key(field: &str) -> &str {
+     field.strip_prefix("note.").unwrap_or(field)
+ }
+
  pub fn is_reserved_field(field: &str) -> bool {
      // Keep for backward compatibility during transition
      // Mark as deprecated in documentation
  }
```

### A.2 `src/query/translator.rs`

```diff
  fn translate_identifier(word: &str, ...) -> String {
+     // NEW: Handle file. prefix
+     if word.starts_with("file.") {
+         let field = &word[5..]; // strip "file."
+         return field.to_string();
+     }
+
+     // NEW: Handle note. prefix
+     if word.starts_with("note.") {
+         let key = &word[5..]; // strip "note."
+         return translate_to_json_path(key);
+     }
+
      // EXISTING: Bare identifier - frontmatter (unchanged)
      if is_sql_keyword(word) {
          return word.to_string();
      }
      return translate_to_json_path(word);
  }
```

### A.3 `README.md` Query Examples

See Section 3.4.1 for detailed changes.

---

## Appendix B: Test Case Examples

### B.1 New Unit Tests for `detector.rs`

```rust
#[test]
fn test_is_file_property() {
    assert!(is_file_property("file.path"));
    assert!(is_file_property("file.name"));
    assert!(is_file_property("file.mtime"));
    assert!(!is_file_property("file.author"));  // Not a file property
    assert!(!is_file_property("author"));        // Not a file property
}

#[test]
fn test_note_field_key() {
    assert_eq!(note_field_key("note.author"), "author");
    assert_eq!(note_field_key("author"), "author");
    assert_eq!(note_field_key("note._schema.strict"), "_schema.strict");
}
```

### B.2 Updated Translator Tests

```rust
#[test]
fn test_translate_file_property() {
    let result = translate("SELECT file.name FROM notes");
    assert!(result.contains("SELECT name FROM notes"));
}

#[test]
fn test_translate_note_prefix() {
    let result = translate("SELECT note.author FROM notes");
    assert!(result.contains("json_extract_string"));
    assert!(result.contains("author"));
}

#[test]
fn test_translate_file_array() {
    let result = translate("list_contains(file.tags, 'todo')");
    assert_eq!(result, "list_contains(tags, 'todo')");
}
```

---

*End of Technical Plan*
