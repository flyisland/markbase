use regex::Regex;
use std::fs;
use std::path::Path;

use crate::db::Database;
use crate::extractor::Extractor;

pub struct RenameResult {
    pub old_path: String,
    pub new_path: String,
    pub updated_links: usize,
}

pub fn rename_note(
    base_dir: &Path,
    db: &Database,
    old_name: &str,
    new_name: &str,
) -> Result<RenameResult, Box<dyn std::error::Error>> {
    let docs = db.get_documents_by_name(old_name)?;
    if docs.is_empty() {
        return Err(format!("Note '{}' not found", old_name).into());
    }
    if docs.len() > 1 {
        let paths: Vec<&str> = docs.iter().map(|d| d.path.as_str()).collect();
        return Err(format!(
            "Multiple notes named '{}' found: {}",
            old_name,
            paths.join(", ")
        )
        .into());
    }

    if db.name_exists(new_name)? {
        return Err(format!("Note '{}' already exists", new_name).into());
    }

    let doc = &docs[0];
    let old_file_path = base_dir.join(&doc.path);

    if !old_file_path.exists() {
        return Err(format!("File '{}' not found on disk", old_file_path.display()).into());
    }

    let new_file_name = format!("{}.md", new_name);
    let parent = old_file_path.parent().unwrap_or(base_dir);
    let new_file_path = parent.join(&new_file_name);

    if new_file_path.exists() {
        return Err(format!("File '{}' already exists on disk", new_file_path.display()).into());
    }

    let new_relative_path = if doc.folder.is_empty() || doc.folder == "." {
        new_file_name.clone()
    } else {
        format!("{}/{}", doc.folder, new_file_name)
    };

    let updated_files =
        update_links_in_backlinked_files(base_dir, &doc.backlinks, old_name, new_name)?;

    fs::rename(&old_file_path, &new_file_path)?;

    db.delete_document(&doc.path)?;

    let new_content = fs::read_to_string(&new_file_path)?;
    let extracted = Extractor::extract(&new_content);
    let new_doc = crate::db::Document {
        path: new_relative_path.clone(),
        folder: doc.folder.clone(),
        name: new_name.to_string(),
        ext: "md".to_string(),
        size: new_content.len() as u64,
        ctime: doc.ctime,
        mtime: std::fs::metadata(&new_file_path)?
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64,
        content: new_content,
        tags: extracted.tags,
        links: extracted.links,
        backlinks: vec![],
        embeds: extracted.embeds,
        properties: extracted.frontmatter,
    };
    db.upsert_document(&new_doc)?;

    for file_path in &updated_files {
        reindex_file(base_dir, db, file_path)?;
    }

    Ok(RenameResult {
        old_path: doc.path.clone(),
        new_path: new_relative_path,
        updated_links: updated_files.len(),
    })
}

fn update_links_in_backlinked_files(
    base_dir: &Path,
    backlinks: &[String],
    old_name: &str,
    new_name: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut updated_files = Vec::new();

    for backlink_path in backlinks {
        let full_path = base_dir.join(backlink_path);
        if !full_path.exists() {
            continue;
        }

        let content = fs::read_to_string(&full_path)?;
        let new_content = update_links_in_content(&content, old_name, new_name);

        if new_content != content {
            fs::write(&full_path, &new_content)?;
            updated_files.push(backlink_path.clone());
        }
    }

    Ok(updated_files)
}

fn update_links_in_content(content: &str, old_name: &str, new_name: &str) -> String {
    let escaped_old = regex::escape(old_name);

    let patterns = vec![
        format!(r"\[\[{}(\|[^\]]*)?\]\]", escaped_old),
        format!(r"\[\[{}(#[^\]\|]*)?(\|[^\]]*)?\]\]", escaped_old),
    ];

    let mut result = content.to_string();

    for pattern in patterns {
        let re = Regex::new(&pattern).unwrap();
        result = re
            .replace_all(&result, |caps: &regex::Captures| {
                let mut replacement = format!("[[{}", new_name);

                for i in 1..caps.len() {
                    if let Some(m) = caps.get(i) {
                        replacement.push_str(m.as_str());
                    }
                }
                replacement.push_str("]]");
                replacement
            })
            .to_string();
    }

    result
}

fn reindex_file(
    base_dir: &Path,
    db: &Database,
    relative_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let full_path = base_dir.join(relative_path);
    if !full_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&full_path)?;
    let metadata = fs::metadata(&full_path)?;
    let extracted = Extractor::extract(&content);

    let parent = full_path.parent().unwrap_or(base_dir);
    let folder = if parent == base_dir {
        String::new()
    } else {
        parent.strip_prefix(base_dir)?.to_string_lossy().to_string()
    };

    let name = full_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let doc = crate::db::Document {
        path: relative_path.to_string(),
        folder,
        name,
        ext: "md".to_string(),
        size: content.len() as u64,
        ctime: metadata
            .created()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64,
        mtime: metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64,
        content,
        tags: extracted.tags,
        links: extracted.links,
        backlinks: vec![],
        embeds: extracted.embeds,
        properties: extracted.frontmatter,
    };

    db.upsert_document(&doc)?;
    Ok(())
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
}
