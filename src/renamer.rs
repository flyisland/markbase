use crate::extractor::{EMBED_RE, Extractor, WIKILINK_RE};
use gray_matter::Matter;
use gray_matter::engine::YAML;
use std::fs;
use std::path::Path;

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
    // Find the old note file by name (search filesystem)
    let old_file_path = find_note_by_name(base_dir, old_name)?;

    if old_file_path.is_none() {
        return Err(format!("Note '{}' not found", old_name).into());
    }
    let old_file_path = old_file_path.unwrap();

    // Handle extension: add .md if not present, preserve if already there
    let new_file_name = if new_name.ends_with(".md") {
        new_name.to_string()
    } else {
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

        let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        if file_name == name {
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
    let (frontmatter, body) = parse_frontmatter(content);

    let body = EMBED_RE.replace_all(&body, |caps: &regex::Captures| {
        let target = caps.get(1).map_or("", |m| m.as_str());
        let normalized_target = Extractor::normalize_link_name(target);
        if normalized_target == old_name {
            let normalized_suffix = if let Some((_, tail)) = target.rsplit_once('/') {
                tail
            } else {
                target
            };
            let suffix = normalized_suffix
                .strip_prefix(old_name)
                .unwrap_or(normalized_suffix);
            format!("![[{}{}]]", new_name, suffix)
        } else {
            caps.get(0).map_or("", |m| m.as_str()).to_string()
        }
    });

    let body = WIKILINK_RE.replace_all(&body, |caps: &regex::Captures| {
        let target = caps.get(1).map_or("", |m| m.as_str());
        let normalized_target = Extractor::normalize_link_name(target);
        if normalized_target == old_name {
            let normalized_suffix = if let Some((_, tail)) = target.rsplit_once('/') {
                tail
            } else {
                target
            };
            let suffix = normalized_suffix
                .strip_prefix(old_name)
                .unwrap_or(normalized_suffix);
            format!("[[{}{}]]", new_name, suffix)
        } else {
            caps.get(0).map_or("", |m| m.as_str()).to_string()
        }
    });

    let body_str = body.to_string();

    if let Some(ref fm) = frontmatter {
        let new_frontmatter = rewrite_frontmatter_links(fm, old_name, new_name);
        if let Some(fm) = new_frontmatter {
            reconstruct_file(&fm, &body_str)
        } else {
            body_str
        }
    } else {
        body_str
    }
}

fn parse_frontmatter(content: &str) -> (Option<String>, String) {
    let matter = Matter::<YAML>::new();
    match matter.parse::<serde_json::Value>(content) {
        Ok(result) => {
            if let Some(data) = result.data {
                let fm_json = serde_json::to_string(&data).ok();
                (fm_json, result.content)
            } else {
                (None, content.to_string())
            }
        }
        Err(_) => (None, content.to_string()),
    }
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
            let new_str = WIKILINK_RE
                .replace_all(s, |caps: &regex::Captures| {
                    let original = caps.get(0).map_or("", |m| m.as_str());
                    let target = caps.get(1).map_or("", |m| m.as_str());
                    let normalized_target = Extractor::normalize_link_name(target);
                    if normalized_target == old_name {
                        let normalized_suffix = if let Some((_, tail)) = target.rsplit_once('/') {
                            tail
                        } else {
                            target
                        };
                        let suffix = normalized_suffix
                            .strip_prefix(old_name)
                            .unwrap_or(normalized_suffix);
                        format!("[[{}{}]]", new_name, suffix)
                    } else {
                        original.to_string()
                    }
                })
                .to_string();
            *s = new_str;
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
