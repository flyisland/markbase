use crate::constants::RESERVED_FIELDS;
use crate::db::{Database, Note};
use crate::extractor::{Extractor, normalize_tag};
use ignore::WalkBuilder;
use ignore::gitignore::Gitignore;
use ignore::gitignore::GitignoreBuilder;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct IndexStats {
    pub new: usize,
    pub updated: usize,
    pub deleted: usize,
    pub errors: usize,
    pub skipped: Vec<(String, String)>,
    pub new_files: Vec<String>,
    pub updated_files: Vec<String>,
    pub deleted_files: Vec<String>,
    pub name_conflicts: Vec<(String, String)>,
    pub duration_ms: u64,
    pub base_dir: PathBuf,
    pub total: usize,
}

impl IndexStats {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            new: 0,
            updated: 0,
            deleted: 0,
            errors: 0,
            skipped: Vec::new(),
            new_files: Vec::new(),
            updated_files: Vec::new(),
            deleted_files: Vec::new(),
            name_conflicts: Vec::new(),
            duration_ms: 0,
            base_dir: base_dir.to_path_buf(),
            total: 0,
        }
    }

    pub fn relative_path(&self, full_path: &str) -> String {
        let full = Path::new(full_path);
        if let Ok(rel) = full.strip_prefix(&self.base_dir) {
            rel.to_string_lossy().to_string()
        } else {
            full_path.to_string()
        }
    }
}

impl Default for IndexStats {
    fn default() -> Self {
        Self {
            new: 0,
            updated: 0,
            deleted: 0,
            errors: 0,
            skipped: Vec::new(),
            new_files: Vec::new(),
            updated_files: Vec::new(),
            deleted_files: Vec::new(),
            name_conflicts: Vec::new(),
            duration_ms: 0,
            base_dir: PathBuf::new(),
            total: 0,
        }
    }
}

pub fn index_directory(
    abs_base_dir: &Path,
    db: &Database,
    force: bool,
) -> Result<IndexStats, Box<dyn std::error::Error>> {
    let start = Instant::now();
    let mut stats = IndexStats::new(abs_base_dir);
    let mut all_notes: Vec<Note> = Vec::new();
    let mut seen_paths: HashSet<String> = HashSet::new();
    let mut name_to_path: HashMap<String, String> = HashMap::new();

    let existing_records: HashMap<String, (i64, u64)> = if !force {
        db.get_all_mtime_and_size()?
    } else {
        HashMap::new()
    };

    let ignore_patterns: Option<Gitignore> = {
        let mut builder = GitignoreBuilder::new(abs_base_dir);

        let gitignore_path = abs_base_dir.join(".gitignore");
        if gitignore_path.exists()
            && let Ok(content) = fs::read_to_string(&gitignore_path)
        {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    let _ = builder.add_line(None, trimmed);
                }
            }
        }

        let markbaseignore_path = abs_base_dir.join(".markbaseignore");
        if markbaseignore_path.exists()
            && let Ok(content) = fs::read_to_string(&markbaseignore_path)
        {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    let _ = builder.add_line(None, trimmed);
                }
            }
        }

        builder.build().ok()
    };

    for entry in WalkBuilder::new(abs_base_dir).follow_links(true).build() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Warning: error walking directory: {}", e);
                continue;
            }
        };
        let path = entry.path();

        if let Some(ref ignore_patterns) = ignore_patterns
            && let Ok(rel_path) = path.strip_prefix(abs_base_dir)
        {
            let is_dir = path.is_dir();
            let match_result = ignore_patterns.matched(rel_path, is_dir);
            if match_result.is_ignore() {
                continue;
            }

            let mut parent_ignored = false;
            for parent in rel_path.ancestors() {
                if parent.as_os_str().is_empty() {
                    break;
                }
                let parent_match = ignore_patterns.matched(parent, true);
                if parent_match.is_ignore() {
                    parent_ignored = true;
                    break;
                }
            }
            if parent_ignored {
                continue;
            }
        }

        if path.is_file() && path.extension().is_some() {
            let rel_path = path.strip_prefix(abs_base_dir).map_err(|_| {
                format!(
                    "Path {} is not under base dir {}",
                    path.display(),
                    abs_base_dir.display()
                )
            })?;
            let path_str = rel_path.to_string_lossy().to_string();
            seen_paths.insert(path_str.clone());

            let file_name = path.file_name().unwrap().to_string_lossy();
            let ext = path.extension().unwrap().to_string_lossy().to_string();
            let name = if ext == "md" {
                file_name.trim_end_matches(".md").to_string()
            } else {
                file_name.to_string()
            };

            if let Some(existing_path) = name_to_path.get(&name) {
                stats
                    .name_conflicts
                    .push((path_str.clone(), existing_path.clone()));
                eprintln!(
                    "⚠ Skipped: {} — name conflict with {}",
                    path_str, existing_path
                );
                continue;
            }
            name_to_path.insert(name, path_str.clone());

            let existing_record = existing_records.get(&path_str).copied();

            match index_single_file(path, abs_base_dir, db, existing_record) {
                Ok(Some(note)) => {
                    all_notes.push(note.clone());
                    if existing_records.contains_key(&note.path) {
                        stats.updated += 1;
                        stats.updated_files.push(note.path.clone());
                    } else {
                        stats.new += 1;
                        stats.new_files.push(note.path.clone());
                    }
                }
                Ok(None) => {
                    stats
                        .skipped
                        .push((path_str.clone(), "unchanged".to_string()));
                }
                Err(e) => {
                    stats.errors += 1;
                    eprintln!("Error indexing {}: {}", path.display(), e);
                }
            }
        }
    }

    for path in existing_records.keys() {
        if !seen_paths.contains(path) {
            db.delete_note(path)?;
            stats.deleted += 1;
            stats.deleted_files.push(path.clone());
        }
    }

    update_backlinks(db, &all_notes)?;

    stats.total = db.count_notes()?;

    stats.duration_ms = start.elapsed().as_millis() as u64;

    Ok(stats)
}

fn index_single_file(
    path: &Path,
    base_dir: &Path,
    db: &Database,
    existing_record: Option<(i64, u64)>,
) -> Result<Option<Note>, Box<dyn std::error::Error>> {
    let rel_path = path.strip_prefix(base_dir).map_err(|_| {
        format!(
            "Path {} is not under base dir {}",
            path.display(),
            base_dir.display()
        )
    })?;
    let path_str = rel_path.to_string_lossy().to_string();

    let metadata = fs::metadata(path)?;
    let file_mtime = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs() as i64;
    let file_size = metadata.len();

    if let Some((db_mtime, db_size)) = existing_record
        && file_mtime <= db_mtime
        && file_size == db_size
    {
        return Ok(None);
    }

    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
    let parent = rel_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let ext = path.extension().unwrap().to_string_lossy().to_string();
    let is_md = ext == "md";

    let (tags, links, embeds, properties) = if is_md {
        let content = fs::read_to_string(path)?;
        let extracted = Extractor::extract(&content);

        if let Some(obj) = extracted.frontmatter.as_object() {
            for key in obj.keys() {
                if RESERVED_FIELDS.contains(&key.as_str()) && *key != "tags" {
                    eprintln!(
                        "⚠ {}: frontmatter field '{}' conflicts with a reserved field and will be ignored.",
                        path.display(),
                        key
                    );
                }
            }
        }

        // Merge frontmatter tags with content tags
        // Apply Obsidian Tag Format validation to frontmatter tags:
        // - Must contain at least one non-numerical character (reject pure numeric like "1984")
        // - Case-insensitive: normalize to lowercase
        let mut merged_tags = extracted.tags;
        if let Some(fm_tags) = extracted.frontmatter.get("tags")
            && let Some(tag_array) = fm_tags.as_array()
        {
            for tag in tag_array {
                if let Some(tag_str) = tag.as_str()
                    && let Some(normalized) = normalize_tag(tag_str)
                {
                    merged_tags.push(normalized);
                }
            }
        }
        merged_tags.sort();
        merged_tags.dedup();

        (
            merged_tags,
            extracted.links,
            extracted.embeds,
            extracted.frontmatter,
        )
    } else {
        (vec![], vec![], vec![], serde_json::json!(null))
    };

    let size = metadata.len();
    let ctime = metadata.created()?.duration_since(UNIX_EPOCH)?.as_secs() as i64;
    let mtime = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs() as i64;

    let name = if is_md {
        file_name.trim_end_matches(".md").to_string()
    } else {
        file_name.clone()
    };

    let note = Note {
        path: path_str,
        folder: parent,
        name,
        ext,
        size,
        ctime,
        mtime,
        tags,
        links,
        backlinks: vec![],
        embeds,
        properties,
    };

    db.upsert_note(&note)?;

    Ok(Some(note))
}

fn update_backlinks(
    db: &Database,
    indexed_notes: &[Note],
) -> Result<(), Box<dyn std::error::Error>> {
    let link_map = db.get_all_links()?;

    let path_to_name: std::collections::HashMap<String, String> = indexed_notes
        .iter()
        .map(|note| (note.path.clone(), note.name.clone()))
        .collect();

    let mut backlinks: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for (path, links) in &link_map {
        let source_name = path_to_name.get(path);
        for link in links {
            let link_name = Extractor::normalize_link_name(link);
            if !link_name.is_empty()
                && let Some(name) = source_name
            {
                backlinks.entry(link_name).or_default().push(name.clone());
            }
        }
    }

    for note in indexed_notes {
        if let Some(back_links) = backlinks.get(&note.name) {
            let mut updated_note = note.clone();
            updated_note.backlinks = back_links.clone();
            db.upsert_note(&updated_note)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn get_unique_id() -> u64 {
        TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    fn create_test_directory() -> (PathBuf, PathBuf) {
        let temp_dir = std::env::temp_dir();
        let unique_id = get_unique_id();
        let test_dir = temp_dir.join(format!("test_scanner_{}_{}", std::process::id(), unique_id));
        let db_path = temp_dir.join(format!(
            "test_scanner_db_{}_{}.duckdb",
            std::process::id(),
            unique_id
        ));

        // Clean up if exists
        let _ = fs::remove_dir_all(&test_dir);
        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_file(temp_dir.join(format!(
            "test_scanner_db_{}_{}.duckdb.wal",
            std::process::id(),
            unique_id
        )));
        fs::create_dir_all(&test_dir).unwrap();

        (test_dir, db_path)
    }

    fn cleanup(test_dir: &Path, db_path: &Path) {
        let _ = fs::remove_dir_all(test_dir);
        let _ = fs::remove_file(db_path);
        // Also remove WAL file if it exists
        let wal_path = db_path.with_extension("duckdb.wal");
        let _ = fs::remove_file(&wal_path);
    }

    #[test]
    fn test_index_single_file() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, "test.md", "# Test\n\nContent here.");

        let db = Database::new(&db_path).unwrap();
        let result = index_directory(&test_dir, &db, false);
        assert!(result.is_ok());

        let records = db.get_all_mtime_and_size().unwrap();
        assert!(records.contains_key("test.md"));

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_multiple_files() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, "file1.md", "# File 1");
        create_test_file(&test_dir, "file2.md", "# File 2");
        create_test_file(&test_dir, "file3.md", "# File 3");

        let db = Database::new(&db_path).unwrap();
        let result = index_directory(&test_dir, &db, false);
        assert!(result.is_ok());

        let link_map = db.get_all_links().unwrap();
        assert_eq!(link_map.len(), 3);

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_with_subdirectories() {
        let (test_dir, db_path) = create_test_directory();

        let subdir = test_dir.join("subdir");
        fs::create_dir(&subdir).unwrap();

        create_test_file(&test_dir, "root.md", "# Root");
        create_test_file(&subdir, "sub.md", "# Sub");

        let db = Database::new(&db_path).unwrap();
        let result = index_directory(&test_dir, &db, false);
        assert!(result.is_ok());

        let link_map = db.get_all_links().unwrap();
        assert_eq!(link_map.len(), 2);

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_all_files() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, "readme.md", "# README");
        create_test_file(&test_dir, "notes.txt", "Some notes");
        create_test_file(&test_dir, "data.json", "{}");

        let db = Database::new(&db_path).unwrap();
        let result = index_directory(&test_dir, &db, false);
        assert!(result.is_ok());

        let records = db.get_all_mtime_and_size().unwrap();
        assert_eq!(records.len(), 3);
        assert!(records.contains_key("readme.md"));
        assert!(records.contains_key("notes.txt"));
        assert!(records.contains_key("data.json"));

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_with_frontmatter() {
        let (test_dir, db_path) = create_test_directory();

        let content = r#"---
title: Test
tags: [test, example]
---

# Test Content

See [[other]] for more."#;

        create_test_file(&test_dir, "with_frontmatter.md", content);

        let db = Database::new(&db_path).unwrap();
        let result = index_directory(&test_dir, &db, false);
        assert!(result.is_ok());

        let link_map = db.get_all_links().unwrap();
        assert_eq!(link_map.len(), 1);

        let links = &link_map["with_frontmatter.md"];
        assert!(links.contains(&"other".to_string()));

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_with_backlinks() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, "target.md", "# Target");
        create_test_file(&test_dir, "referrer.md", "See [[target]] for info.");

        let db = Database::new(&db_path).unwrap();
        let result = index_directory(&test_dir, &db, false);
        assert!(result.is_ok());

        // Verify both files are indexed
        let link_map = db.get_all_links().unwrap();
        assert_eq!(link_map.len(), 2);

        // Verify backlinks contain note names (not paths)
        let target_notes = db.get_notes_by_name("target").unwrap();
        assert_eq!(target_notes.len(), 1);
        assert_eq!(target_notes[0].backlinks, vec!["referrer"]);

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_with_tags() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(
            &test_dir,
            "tagged.md",
            "# Title\n\nContent with #tag1 and #tag2.",
        );

        let db = Database::new(&db_path).unwrap();
        let result = index_directory(&test_dir, &db, false);
        assert!(result.is_ok());

        let records = db.get_all_mtime_and_size().unwrap();
        assert!(records.contains_key("tagged.md"));

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_with_embeds() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(
            &test_dir,
            "with_embeds.md",
            "See ![[image.png]] and ![[diagram.svg]].",
        );

        let db = Database::new(&db_path).unwrap();
        let result = index_directory(&test_dir, &db, false);
        assert!(result.is_ok());

        let records = db.get_all_mtime_and_size().unwrap();
        assert!(records.contains_key("with_embeds.md"));

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_force_reindex() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, "test.md", "# Original");

        let db = Database::new(&db_path).unwrap();

        // First index
        index_directory(&test_dir, &db, false).unwrap();
        let records1 = db.get_all_mtime_and_size().unwrap();
        let mtime1 = records1.get("test.md").map(|(m, _)| *m);

        // Wait a bit and update file
        std::thread::sleep(std::time::Duration::from_millis(100));
        create_test_file(&test_dir, "test.md", "# Updated");

        // Re-index with force
        index_directory(&test_dir, &db, true).unwrap();
        let records2 = db.get_all_mtime_and_size().unwrap();
        let mtime2 = records2.get("test.md").map(|(m, _)| *m);

        // Should have been updated
        assert!(mtime2.unwrap() >= mtime1.unwrap());

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_empty_directory() {
        let (test_dir, db_path) = create_test_directory();

        let db = Database::new(&db_path).unwrap();
        let result = index_directory(&test_dir, &db, false);
        assert!(result.is_ok());

        let link_map = db.get_all_links().unwrap();
        assert!(link_map.is_empty());

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_detects_deleted_files() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, "keep.md", "# Keep");
        create_test_file(&test_dir, "delete.md", "# Delete");

        let db = Database::new(&db_path).unwrap();
        index_directory(&test_dir, &db, false).unwrap();

        let records = db.get_all_mtime_and_size().unwrap();
        assert_eq!(records.len(), 2);

        fs::remove_file(test_dir.join("delete.md")).unwrap();

        let result = index_directory(&test_dir, &db, false).unwrap();
        assert_eq!(result.deleted, 1);
        assert!(result.deleted_files.contains(&"delete.md".to_string()));

        let records = db.get_all_mtime_and_size().unwrap();
        assert_eq!(records.len(), 1);
        assert!(records.contains_key("keep.md"));

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_detects_name_conflict() {
        let (test_dir, db_path) = create_test_directory();

        let subdir = test_dir.join("subdir");
        fs::create_dir(&subdir).unwrap();

        create_test_file(&test_dir, "note.md", "# Note 1");
        create_test_file(&subdir, "note.md", "# Note 2");

        let db = Database::new(&db_path).unwrap();
        let result = index_directory(&test_dir, &db, false).unwrap();

        assert_eq!(result.name_conflicts.len(), 1);
        assert_eq!(result.new, 1);

        let records = db.get_all_mtime_and_size().unwrap();
        assert_eq!(records.len(), 1);

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_size_change_triggers_update() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, "test.md", "# Original content");

        let db = Database::new(&db_path).unwrap();
        let result1 = index_directory(&test_dir, &db, false).unwrap();
        assert_eq!(result1.new, 1);
        assert_eq!(result1.updated, 0);

        std::thread::sleep(std::time::Duration::from_millis(100));
        create_test_file(&test_dir, "test.md", "# Much longer content here");

        let result2 = index_directory(&test_dir, &db, false).unwrap();
        assert_eq!(result2.new, 0);
        assert_eq!(result2.updated, 1);

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_unchanged_size_and_mtime_skipped() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, "test.md", "# Content");

        let db = Database::new(&db_path).unwrap();
        let result1 = index_directory(&test_dir, &db, false).unwrap();
        assert_eq!(result1.new, 1);

        let result2 = index_directory(&test_dir, &db, false).unwrap();
        assert_eq!(result2.new, 0);
        assert_eq!(result2.updated, 0);
        assert!(!result2.skipped.is_empty());

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_respects_gitignore_file_pattern() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, ".gitignore", "skip.md\n");
        create_test_file(&test_dir, "test.md", "# Test");
        create_test_file(&test_dir, "skip.md", "# Skip");

        let db = Database::new(&db_path).unwrap();
        let _result = index_directory(&test_dir, &db, false).unwrap();

        let records = db.get_all_mtime_and_size().unwrap();
        assert!(records.contains_key("test.md"));
        assert!(!records.contains_key("skip.md"));

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_respects_gitignore_directory_pattern() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, ".gitignore", "subdir\n");

        let subdir = test_dir.join("subdir");
        fs::create_dir(&subdir).unwrap();

        create_test_file(&test_dir, "main.md", "# Main");
        create_test_file(&subdir, "sub.md", "# Sub");

        let db = Database::new(&db_path).unwrap();
        let _result = index_directory(&test_dir, &db, false).unwrap();

        let records = db.get_all_mtime_and_size().unwrap();
        assert!(records.contains_key("main.md"));
        assert!(!records.contains_key("subdir/sub.md"));

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_respects_gitignore_negation_pattern() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, ".gitignore", "test.md\n!important.md\n");
        create_test_file(&test_dir, "test.md", "# Test");
        create_test_file(&test_dir, "important.md", "# Important");
        create_test_file(&test_dir, "other.md", "# Other");

        let db = Database::new(&db_path).unwrap();
        let _result = index_directory(&test_dir, &db, false).unwrap();

        let records = db.get_all_mtime_and_size().unwrap();
        assert!(!records.contains_key("test.md"));
        assert!(records.contains_key("important.md"));
        assert!(records.contains_key("other.md"));

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_respects_gitignore_hidden_files() {
        let (test_dir, db_path) = create_test_directory();

        let hidden = test_dir.join(".hidden");
        fs::create_dir(&hidden).unwrap();

        create_test_file(&test_dir, ".gitignore", ".hidden/\n");
        create_test_file(&test_dir, "main.md", "# Main");
        create_test_file(&hidden, "secret.md", "# Secret");

        let db = Database::new(&db_path).unwrap();
        let _result = index_directory(&test_dir, &db, false).unwrap();

        let records = db.get_all_mtime_and_size().unwrap();
        assert!(records.contains_key("main.md"));
        assert!(!records.contains_key(".hidden/secret.md"));

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_no_gitignore() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, "test.md", "# Test");
        create_test_file(&test_dir, "other.md", "# Other");

        let db = Database::new(&db_path).unwrap();
        let _result = index_directory(&test_dir, &db, false).unwrap();

        let records = db.get_all_mtime_and_size().unwrap();
        assert!(records.contains_key("test.md"));
        assert!(records.contains_key("other.md"));

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_respects_markbaseignore() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, ".markbaseignore", "skip.md\n");
        create_test_file(&test_dir, "test.md", "# Test");
        create_test_file(&test_dir, "skip.md", "# Skip");

        let db = Database::new(&db_path).unwrap();
        let _result = index_directory(&test_dir, &db, false).unwrap();

        let records = db.get_all_mtime_and_size().unwrap();
        assert!(records.contains_key("test.md"));
        assert!(!records.contains_key("skip.md"));

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_markbaseignore_takes_precedence() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, ".gitignore", "keep.md\n");
        create_test_file(&test_dir, ".markbaseignore", "skip.md\n");
        create_test_file(&test_dir, "keep.md", "# Keep");
        create_test_file(&test_dir, "skip.md", "# Skip");
        create_test_file(&test_dir, "other.md", "# Other");

        let db = Database::new(&db_path).unwrap();
        let _result = index_directory(&test_dir, &db, false).unwrap();

        let records = db.get_all_mtime_and_size().unwrap();
        assert!(!records.contains_key("keep.md"));
        assert!(!records.contains_key("skip.md"));
        assert!(records.contains_key("other.md"));

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_markbaseignore_with_gitignore() {
        let (test_dir, db_path) = create_test_directory();

        let private_dir = test_dir.join("private");
        fs::create_dir(&private_dir).unwrap();

        create_test_file(&test_dir, ".gitignore", "skip.md\n");
        create_test_file(&test_dir, ".markbaseignore", "private/\n");
        create_test_file(&test_dir, "main.md", "# Main");
        create_test_file(&test_dir, "skip.md", "# Skip");
        create_test_file(&private_dir, "secret.md", "# Secret");

        let db = Database::new(&db_path).unwrap();
        let _result = index_directory(&test_dir, &db, false).unwrap();

        let records = db.get_all_mtime_and_size().unwrap();
        assert!(records.contains_key("main.md"));
        assert!(!records.contains_key("skip.md"));
        assert!(!records.contains_key("private/secret.md"));

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_file_tags_merge_content_and_frontmatter() {
        // Test that file.tags contains tags from both content (#tag) and frontmatter
        let (test_dir, db_path) = create_test_directory();

        let content = r#"---
title: Test Note
tags: [fm-tag1, fm-tag2]
---
# Test Note

This has #content-tag1 and #content-tag2 in the body."#;

        create_test_file(&test_dir, "test.md", content);

        let db = Database::new(&db_path).unwrap();
        let result = index_directory(&test_dir, &db, false);
        assert!(result.is_ok());

        // Query the note and verify all tags are present
        let notes = db.get_notes_by_name("test").unwrap();
        assert_eq!(notes.len(), 1);

        let note = &notes[0];
        assert!(
            note.tags.contains(&"fm-tag1".to_string()),
            "Missing frontmatter tag fm-tag1"
        );
        assert!(
            note.tags.contains(&"fm-tag2".to_string()),
            "Missing frontmatter tag fm-tag2"
        );
        assert!(
            note.tags.contains(&"content-tag1".to_string()),
            "Missing content tag content-tag1"
        );
        assert!(
            note.tags.contains(&"content-tag2".to_string()),
            "Missing content tag content-tag2"
        );
        assert_eq!(
            note.tags.len(),
            4,
            "Expected exactly 4 tags, got: {:?}",
            note.tags
        );

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_frontmatter_tags_validation() {
        // Test Obsidian Tag Format validation for frontmatter tags:
        // - Pure numeric tags (e.g., "1984", "123") should be rejected
        // - Tags should be normalized to lowercase (case-insensitive)
        let (test_dir, db_path) = create_test_directory();

        let content = r#"---
title: Test Note
tags: [valid-tag, 1984, TAG-Upper, y2024, 123abc, "007"]
---
# Test Note

Content here."#;

        create_test_file(&test_dir, "test.md", content);

        let db = Database::new(&db_path).unwrap();
        let result = index_directory(&test_dir, &db, false);
        assert!(result.is_ok());

        let notes = db.get_notes_by_name("test").unwrap();
        assert_eq!(notes.len(), 1);

        let note = &notes[0];

        // Valid tags should be present (normalized to lowercase)
        assert!(
            note.tags.contains(&"valid-tag".to_string()),
            "Missing valid tag 'valid-tag'"
        );
        assert!(
            note.tags.contains(&"tag-upper".to_string()),
            "Tag should be normalized to lowercase: 'tag-upper'"
        );
        assert!(
            note.tags.contains(&"y2024".to_string()),
            "Missing valid tag 'y2024'"
        );
        assert!(
            note.tags.contains(&"123abc".to_string()),
            "Missing valid tag '123abc'"
        );

        // Invalid tags should be rejected
        assert!(
            !note.tags.contains(&"1984".to_string()),
            "Pure numeric tag '1984' should be rejected"
        );
        assert!(
            !note.tags.contains(&"007".to_string()),
            "Pure numeric tag '007' should be rejected"
        );

        // Total should be 4 valid tags only
        assert_eq!(
            note.tags.len(),
            4,
            "Expected exactly 4 valid tags, got: {:?}",
            note.tags
        );

        cleanup(&test_dir, &db_path);
    }
}
