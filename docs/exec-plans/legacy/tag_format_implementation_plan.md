# Tag Format Implementation Plan

## 1. Current Implementation Analysis

**Current Regex:** `r"#[\w\-/]+"` in [`src/extractor.rs`](src/extractor.rs:7)

**Issues:**
- Uses `\w` (equivalent to `[a-zA-Z0-9_]`) which allows pure numeric tags like `#1984`
- Does not enforce Obsidian's requirement: "must contain at least one non-numerical character"
- Case sensitivity is handled at storage level, not validation level

## 2. Obsidian Specification Requirements

From [Obsidian Help - Tags](https://help.obsidian.md/tags):

1. **Allowed characters:**
   - Alphabetical letters (a-z, A-Z)
   - Numbers (0-9)
   - Underscore (_)
   - Hyphen (-)
   - Forward slash (/) for Nested tags

2. **Mandatory rule:**
   - Must contain at least one non-numerical character
   - Example: `#1984` is INVALID, `#y1984` is VALID

3. **Case handling:**
   - Tags are case-insensitive (e.g., `#tag` and `#TAG` are identical)

## 3. Implementation Changes Required

### 3.1 Update TAG_REGEX

**Current:**
```rust
static TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"#[\w\-/]+").unwrap());
```

**New:**
```rust
// Tag regex with validation: must contain at least one non-digit character
// Pattern explanation:
//   #                    - literal hash
//   (?=.*[a-zA-Z_])      - positive lookahead: must contain at least one letter or underscore
//   [\w\-/]+             - allowed characters: word chars, hyphen, forward slash
static TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"#(?=.*[a-zA-Z_])[\w\-/]+").unwrap()
});
```

**Alternative (if regex crate doesn't support lookahead):**
```rust
// Validate in code instead of regex
static TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"#[\w\-/]+").unwrap());

fn is_valid_tag(tag: &str) -> bool {
    // Remove leading # and check if contains at least one non-digit character
    let content = tag.trim_start_matches('#');
    content.chars().any(|c| !c.is_ascii_digit())
}
```

### 3.2 Update extract_tags Function

**Current:**
```rust
fn extract_tags(content: &str) -> Vec<String> {
    TAG_REGEX
        .find_iter(content)
        .map(|m| m.as_str().trim_start_matches('#').to_string())
        .collect()
}
```

**New (with validation):**
```rust
fn extract_tags(content: &str) -> Vec<String> {
    TAG_REGEX
        .find_iter(content)
        .filter_map(|m| {
            let tag = m.as_str();
            // Additional validation: must contain at least one non-digit
            let content = &tag[1..]; // Skip the # prefix
            if content.chars().any(|c| !c.is_ascii_digit()) {
                Some(content.to_lowercase()) // Convert to lowercase for case-insensitivity
            } else {
                None
            }
        })
        .collect()
}
```

### 3.3 Case Insensitivity Handling

Obsidian treats `#tag` and `#TAG` as identical. Options:

**Option A: Normalize to lowercase during extraction**
```rust
Some(content.to_lowercase())
```

**Option B: Keep original case but store normalized version**
- Store both original and normalized versions
- Query using normalized version

**Recommendation:** Option A - normalize to lowercase for storage, consistent with Obsidian behavior

## 4. Test Cases Required

Add to [`src/extractor.rs`](src/extractor.rs) test module:

```rust
#[test]
fn test_valid_tags() {
    let content = "#tag #TAG #Tag #nested/tag #with-dash #with_underscore #y1984";
    let extracted = Extractor::extract(content);
    assert!(extracted.tags.contains(&"tag".to_string())); // normalized to lowercase
    assert!(extracted.tags.contains(&"nested/tag".to_string()));
    assert!(extracted.tags.contains(&"with-dash".to_string()));
    assert!(extracted.tags.contains(&"with_underscore".to_string()));
    assert!(extracted.tags.contains(&"y1984".to_string()));
}

#[test]
fn test_invalid_pure_numeric_tags() {
    // These should NOT be extracted as tags per Obsidian spec
    let content = "#1984 #123 #007";
    let extracted = Extractor::extract(content);
    assert!(extracted.tags.is_empty());
}

#[test]
fn test_tag_case_normalization() {
    // All should be normalized to lowercase
    let content = "#MyTag #MYTAG #mytag";
    let extracted = Extractor::extract(content);
    assert_eq!(extracted.tags.len(), 3);
    assert!(extracted.tags.iter().all(|t| t == "mytag"));
}

#[test]
fn test_tag_with_non_ascii() {
    // Test edge cases
    let content = "#tag123 #123tag #tag-123 #123-tag";
    let extracted = Extractor::extract(content);
    assert!(extracted.tags.contains(&"tag123".to_string()));
    assert!(extracted.tags.contains(&"123tag".to_string()));
    assert!(extracted.tags.contains(&"tag-123".to_string()));
    assert!(extracted.tags.contains(&"123-tag".to_string()));
}
```

## 5. File Changes Summary

| File | Change |
|------|--------|
| `src/extractor.rs` | Update TAG_REGEX and extract_tags() function |
| `src/extractor.rs` | Add new test cases for tag validation |

## 6. Implementation Steps

1. **Modify TAG_REGEX** - Add positive lookahead or use code-level validation
2. **Update extract_tags()** - Add validation for at least one non-digit character
3. **Add case normalization** - Convert to lowercase for storage
4. **Add tests** - Verify valid/invalid tags and case handling
5. **Run tests** - Ensure `cargo test` passes
6. **Update documentation** - Verify README.md and spec docs are accurate

## 7. Backward Compatibility Considerations

- **Breaking change:** Pure numeric tags like `#1984` will no longer be extracted
- **Impact:** Low - pure numeric tags are rare and not valid per Obsidian spec
- **Mitigation:** Document in release notes

## 8. Verification Checklist

- [ ] `#1984` is NOT extracted as a tag
- [ ] `#y1984` IS extracted as a tag
- [ ] `#Tag` is normalized to `tag`
- [ ] `#nested/deep/tag` is correctly extracted
- [ ] All existing tests pass
- [ ] New test cases added and passing
