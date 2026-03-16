use gray_matter::Matter;
use gray_matter::engine::YAML;
use std::fs;
use std::path::Path;

use crate::link_syntax::{LinkKind, ScanContext, rebuild_link_token, scan_link_tokens};
use crate::name_validator::validate_path_free_name;

pub struct RenameResult {
    pub old_path: String,
    pub new_path: String,
    pub updated_files: Vec<String>,
}

pub fn rename_note(
    base_dir: &Path,
    old_name: &str,
    new_name: &str,
) -> Result<RenameResult, Box<dyn std::error::Error>> {
    validate_path_free_name(old_name, "old_name")?;
    validate_path_free_name(new_name, "new_name")?;

    // Find the old note file by name (search filesystem)
    let old_file_path = find_note_by_name(base_dir, old_name)?;

    if old_file_path.is_none() {
        return Err(format!("Note '{}' not found", old_name).into());
    }
    let old_file_path = old_file_path.unwrap();

    // Handle extension: add .md if no extension present, preserve existing extension
    let new_file_name = if new_name.contains('.') {
        // Already has an extension, use as-is
        new_name.to_string()
    } else {
        // No extension, add .md
        format!("{}.md", new_name)
    };

    let parent = old_file_path.parent().unwrap_or(base_dir);
    let new_file_path = parent.join(&new_file_name);

    if new_file_path.exists() {
        return Err(format!("File '{}' already exists on disk", new_file_path.display()).into());
    }

    let new_relative_path = if parent == base_dir {
        new_file_name.clone()
    } else {
        format!(
            "{}/{}",
            parent.strip_prefix(base_dir).unwrap().to_string_lossy(),
            new_file_name
        )
    };

    // Update all links in vault before renaming
    let updated_files = update_links_in_vault(base_dir, old_name, new_name)?;

    // Perform the rename
    fs::rename(&old_file_path, &new_file_path)?;

    Ok(RenameResult {
        old_path: old_file_path
            .strip_prefix(base_dir)
            .unwrap()
            .to_string_lossy()
            .to_string(),
        new_path: new_relative_path,
        updated_files,
    })
}

fn find_note_by_name(
    base_dir: &Path,
    name: &str,
) -> Result<Option<std::path::PathBuf>, Box<dyn std::error::Error>> {
    for entry in walkdir::WalkDir::new(base_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // Match by file stem (name without extension) - for standard .md files
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        // Match by full file name (with extension) - for files with custom extensions like .base
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        if file_stem == name || file_name == name {
            return Ok(Some(path.to_path_buf()));
        }
    }
    Ok(None)
}

fn update_links_in_vault(
    base_dir: &Path,
    old_name: &str,
    new_name: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut updated_files = Vec::new();

    for entry in walkdir::WalkDir::new(base_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }

        let rel_path = match path.strip_prefix(base_dir) {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => continue,
        };

        let raw = fs::read_to_string(path)?;

        if !raw.contains(old_name) {
            continue;
        }

        let new_content = update_links_in_content(&raw, old_name, new_name);

        if new_content != raw {
            fs::write(path, &new_content)?;
            updated_files.push(rel_path);
        }
    }

    Ok(updated_files)
}

fn update_links_in_content(content: &str, old_name: &str, new_name: &str) -> String {
    let parsed = parse_frontmatter(content);
    let body_str = rewrite_markdown_body_links(&parsed.body, old_name, new_name);
    let body_changed = body_str != parsed.body;

    let Some(frontmatter_json) = parsed.frontmatter_json.as_deref() else {
        return if body_changed {
            body_str
        } else {
            content.to_string()
        };
    };

    let Some(new_frontmatter) = rewrite_frontmatter_links(frontmatter_json, old_name, new_name)
    else {
        return if body_changed {
            body_str
        } else {
            content.to_string()
        };
    };

    let frontmatter_changed = new_frontmatter != frontmatter_json;

    if !body_changed && !frontmatter_changed {
        return content.to_string();
    }

    if !frontmatter_changed {
        if let Some(prefix) = parsed.raw_frontmatter_prefix {
            return format!("{}{}", prefix, body_str);
        }
        if body_changed {
            return body_str;
        }
    }

    reconstruct_file(&new_frontmatter, &body_str)
}

struct ParsedFrontmatter<'a> {
    raw_frontmatter_prefix: Option<&'a str>,
    frontmatter_json: Option<String>,
    body: String,
}

fn parse_frontmatter(content: &str) -> ParsedFrontmatter<'_> {
    let matter = Matter::<YAML>::new();
    match matter.parse::<serde_json::Value>(content) {
        Ok(result) => {
            if let Some(data) = result.data {
                let raw_frontmatter = split_raw_frontmatter_prefix(content);
                ParsedFrontmatter {
                    raw_frontmatter_prefix: raw_frontmatter.map(|(prefix, _)| prefix),
                    frontmatter_json: serde_json::to_string(&data).ok(),
                    body: raw_frontmatter
                        .map(|(_, body)| body.to_string())
                        .unwrap_or(result.content),
                }
            } else {
                ParsedFrontmatter {
                    raw_frontmatter_prefix: None,
                    frontmatter_json: None,
                    body: content.to_string(),
                }
            }
        }
        Err(_) => ParsedFrontmatter {
            raw_frontmatter_prefix: None,
            frontmatter_json: None,
            body: content.to_string(),
        },
    }
}

fn split_raw_frontmatter_prefix(content: &str) -> Option<(&str, &str)> {
    let mut line_start = 0;
    let first_line_end = next_line_end(content, line_start);
    if trim_line_breaks(&content[line_start..first_line_end]) != "---" {
        return None;
    }

    line_start = first_line_end;
    while line_start < content.len() {
        let line_end = next_line_end(content, line_start);
        if trim_line_breaks(&content[line_start..line_end]) == "---" {
            return Some((&content[..line_end], &content[line_end..]));
        }
        line_start = line_end;
    }

    None
}

fn next_line_end(content: &str, start: usize) -> usize {
    content[start..]
        .find('\n')
        .map(|offset| start + offset + 1)
        .unwrap_or(content.len())
}

fn trim_line_breaks(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn reconstruct_file(frontmatter_json: &str, body: &str) -> String {
    let fm: serde_json::Value = match serde_json::from_str(frontmatter_json) {
        Ok(v) => v,
        Err(_) => return body.to_string(),
    };

    if fm.is_null() || fm.as_object().is_none_or(|o| o.is_empty()) {
        return body.to_string();
    }

    let yaml = serde_yaml::to_string(&fm).unwrap_or_default();
    format!("---\n{}\n---\n\n{}", yaml.trim(), body)
}

fn rewrite_frontmatter_links(
    frontmatter_json: &str,
    old_name: &str,
    new_name: &str,
) -> Option<String> {
    let mut fm: serde_json::Value = serde_json::from_str(frontmatter_json).ok()?;

    rewrite_value_links(&mut fm, old_name, new_name);

    serde_json::to_string(&fm).ok()
}

fn rewrite_value_links(value: &mut serde_json::Value, old_name: &str, new_name: &str) {
    match value {
        serde_json::Value::String(s) => {
            *s = rewrite_frontmatter_string_links(s, old_name, new_name);
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                rewrite_value_links(item, old_name, new_name);
            }
        }
        serde_json::Value::Object(obj) => {
            for (_, v) in obj.iter_mut() {
                rewrite_value_links(v, old_name, new_name);
            }
        }
        _ => {}
    }
}

fn rewrite_markdown_body_links(content: &str, old_name: &str, new_name: &str) -> String {
    rewrite_with_tokens(
        content,
        ScanContext::MarkdownBody,
        old_name,
        new_name,
        |kind| matches!(kind, LinkKind::WikiLink | LinkKind::Embed),
    )
}

fn rewrite_frontmatter_string_links(content: &str, old_name: &str, new_name: &str) -> String {
    rewrite_with_tokens(
        content,
        ScanContext::FrontmatterString,
        old_name,
        new_name,
        |kind| kind == LinkKind::WikiLink,
    )
}

fn rewrite_with_tokens<F>(
    content: &str,
    context: ScanContext,
    old_name: &str,
    new_name: &str,
    allow_kind: F,
) -> String
where
    F: Fn(LinkKind) -> bool,
{
    let tokens = scan_link_tokens(content, context);
    let mut out = String::with_capacity(content.len());
    let mut cursor = 0;

    for token in tokens {
        if !allow_kind(token.kind) || token.parsed.normalized_target != old_name {
            continue;
        }

        out.push_str(&content[cursor..token.full_span.start]);
        out.push_str(&rewrite_token(&token, new_name));
        cursor = token.full_span.end;
    }

    out.push_str(&content[cursor..]);
    out
}

fn rewrite_token(token: &crate::link_syntax::LinkToken, new_name: &str) -> String {
    let target = if token.parsed.is_markdown_note {
        new_name.strip_suffix(".md").unwrap_or(new_name)
    } else {
        new_name
    };
    let alias_separator = original_alias_separator(&token.raw_inner);

    if alias_separator == Some("\\|") {
        rebuild_link_token_with_separator(
            token.kind,
            target,
            token.parsed.anchor.as_deref(),
            token.parsed.alias_or_size.as_deref(),
            "\\|",
        )
    } else {
        rebuild_link_token(
            token.kind,
            target,
            token.parsed.anchor.as_deref(),
            token.parsed.alias_or_size.as_deref(),
        )
    }
}

fn original_alias_separator(raw_inner: &str) -> Option<&'static str> {
    let bytes = raw_inner.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() && bytes[i + 1] == b'|' => return Some("\\|"),
            b'|' => return Some("|"),
            _ => i += 1,
        }
    }
    None
}

fn rebuild_link_token_with_separator(
    kind: LinkKind,
    target: &str,
    anchor: Option<&str>,
    alias: Option<&str>,
    alias_separator: &str,
) -> String {
    let mut out = match kind {
        LinkKind::WikiLink => String::from("[["),
        LinkKind::Embed => String::from("![["),
    };
    out.push_str(target);
    if let Some(anchor) = anchor {
        out.push('#');
        out.push_str(anchor);
    }
    if let Some(alias) = alias {
        out.push_str(alias_separator);
        out.push_str(alias);
    }
    out.push_str("]]");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_links_simple() {
        let content = "See [[old-note]] for details.";
        let result = update_links_in_content(content, "old-note", "new-note");
        assert_eq!(result, "See [[new-note]] for details.");
    }

    #[test]
    fn test_update_links_with_alias() {
        let content = "See [[old-note|My Alias]] for details.";
        let result = update_links_in_content(content, "old-note", "new-note");
        assert_eq!(result, "See [[new-note|My Alias]] for details.");
    }

    #[test]
    fn test_update_links_with_section() {
        let content = "See [[old-note#Overview]] for details.";
        let result = update_links_in_content(content, "old-note", "new-note");
        assert_eq!(result, "See [[new-note#Overview]] for details.");
    }

    #[test]
    fn test_update_links_with_section_and_alias() {
        let content = "See [[old-note#Overview|Overview Section]] for details.";
        let result = update_links_in_content(content, "old-note", "new-note");
        assert_eq!(
            result,
            "See [[new-note#Overview|Overview Section]] for details."
        );
    }

    #[test]
    fn test_update_links_multiple() {
        let content = "See [[old-note]] and [[old-note#Section]] and [[other]].";
        let result = update_links_in_content(content, "old-note", "new-note");
        assert_eq!(
            result,
            "See [[new-note]] and [[new-note#Section]] and [[other]]."
        );
    }

    #[test]
    fn test_update_links_preserves_other_links() {
        let content = "See [[other-note]] and [[old-note]] and [[another]].";
        let result = update_links_in_content(content, "old-note", "new-note");
        assert_eq!(
            result,
            "See [[other-note]] and [[new-note]] and [[another]]."
        );
    }

    #[test]
    fn test_update_links_no_match() {
        let content = "See [[other-note]] for details.";
        let result = update_links_in_content(content, "old-note", "new-note");
        assert_eq!(result, content);
    }

    #[test]
    fn test_update_links_with_special_chars() {
        let content = "See [[my-note-1]] for details.";
        let result = update_links_in_content(content, "my-note-1", "my-note-2");
        assert_eq!(result, "See [[my-note-2]] for details.");
    }

    #[test]
    fn test_update_links_partial_name_not_matched() {
        let content = "See [[old-note-extra]] for details.";
        let result = update_links_in_content(content, "old-note", "new-note");
        assert_eq!(result, "See [[old-note-extra]] for details.");
    }

    #[test]
    fn test_update_embeds() {
        let content = "See ![[old-note]] and ![[old-image.png]].";
        let result = update_links_in_content(content, "old-note", "new-note");
        assert_eq!(result, "See ![[new-note]] and ![[old-image.png]].");
    }

    #[test]
    fn test_update_embeds_with_section() {
        let content = "![[old-note#Section]]";
        let result = update_links_in_content(content, "old-note", "new-note");
        assert_eq!(result, "![[new-note#Section]]");
    }

    #[test]
    fn test_update_frontmatter_links() {
        let content = r#"---
related: "[[old-note]]"
---

Content"#;
        let result = update_links_in_content(content, "old-note", "new-note");
        assert!(result.contains("[[new-note]]"));
        assert!(!result.contains("[[old-note]]"));
    }

    #[test]
    fn test_update_frontmatter_links_with_alias() {
        let content = r#"---
see: "[[old-note|Alias]]"
---

Content"#;
        let result = update_links_in_content(content, "old-note", "new-note");
        assert!(result.contains("[[new-note|Alias]]"));
    }

    #[test]
    fn test_update_frontmatter_links_multiple() {
        let content = r#"---
related: "[[old-note]]"
seeAlso: "[[another-old]]"
---

Content"#;
        let result = update_links_in_content(content, "old-note", "new-note");
        assert!(result.contains("[[new-note]]"));
        assert!(result.contains("[[another-old]]"));
    }

    #[test]
    fn test_plain_old_name_text_does_not_rewrite_or_reformat_frontmatter() {
        let content = r#"---
title: Old Note Mention
tags:
  - old-note
---

This mentions old-note in plain text only.
"#;
        let result = update_links_in_content(content, "old-note", "new-note");
        assert_eq!(result, content);
    }

    #[test]
    fn test_update_embeds_and_wikilinks_together() {
        let content = r#"Body with [[old-note]] and ![[old-note]]."#;
        let result = update_links_in_content(content, "old-note", "new-note");
        assert!(result.contains("[[new-note]]"));
        assert!(result.contains("![[new-note]]"));
    }

    #[test]
    fn test_does_not_affect_unrelated_content() {
        let content = "Some text without links here.";
        let result = update_links_in_content(content, "old-note", "new-note");
        assert_eq!(result, content);
    }

    #[test]
    fn test_normalize_link_name_in_updates() {
        let content = "See [[notes/old-note]] for details.";
        let result = update_links_in_content(content, "old-note", "new-note");
        assert_eq!(result, "See [[new-note]] for details.");
    }
}
