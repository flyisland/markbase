use crate::constants::RESERVED_FIELDS;
use crate::db::{Database, Document};
use crate::extractor::Extractor;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct IndexStats {
    pub new: usize,
    pub updated: usize,
    pub errors: usize,
    pub skipped: Vec<(String, String)>,
    pub new_files: Vec<String>,
    pub updated_files: Vec<String>,
    pub duration_ms: u64,
    pub base_dir: PathBuf,
}

impl IndexStats {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            new: 0,
            updated: 0,
            errors: 0,
            skipped: Vec::new(),
            new_files: Vec::new(),
            updated_files: Vec::new(),
            duration_ms: 0,
            base_dir: base_dir.to_path_buf(),
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
            errors: 0,
            skipped: Vec::new(),
            new_files: Vec::new(),
            updated_files: Vec::new(),
            duration_ms: 0,
            base_dir: PathBuf::new(),
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
    let mut all_docs: Vec<Document> = Vec::new();

    for entry in WalkDir::new(abs_base_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            let rel_path = path.strip_prefix(abs_base_dir).map_err(|_| {
                format!(
                    "Path {} is not under base dir {}",
                    path.display(),
                    abs_base_dir.display()
                )
            })?;
            let path_str = rel_path.to_string_lossy().to_string();
            let exists = db.get_mtime(&path_str)?.is_some();

            match index_single_file(path, abs_base_dir, db, force) {
                Ok(Some(doc)) => {
                    all_docs.push(doc.clone());
                    if exists {
                        stats.updated += 1;
                        stats.updated_files.push(doc.path.clone());
                    } else {
                        stats.new += 1;
                        stats.new_files.push(doc.path.clone());
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

    update_backlinks(db, &all_docs)?;

    stats.duration_ms = start.elapsed().as_millis() as u64;

    Ok(stats)
}

fn index_single_file(
    path: &Path,
    base_dir: &Path,
    db: &Database,
    force: bool,
) -> Result<Option<Document>, Box<dyn std::error::Error>> {
    let rel_path = path.strip_prefix(base_dir).map_err(|_| {
        format!(
            "Path {} is not under base dir {}",
            path.display(),
            base_dir.display()
        )
    })?;
    let path_str = rel_path.to_string_lossy().to_string();

    if !force && let Some(db_mtime) = db.get_mtime(&path_str)? {
        let file_mtime = fs::metadata(path)?
            .modified()?
            .duration_since(UNIX_EPOCH)?
            .as_secs() as i64;
        if file_mtime <= db_mtime {
            return Ok(None);
        }
    }

    let metadata = fs::metadata(path)?;
    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
    let parent = rel_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

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

    let size = metadata.len();
    let ctime = metadata.created()?.duration_since(UNIX_EPOCH)?.as_secs() as i64;
    let mtime = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs() as i64;

    let doc = Document {
        path: path_str,
        folder: parent,
        name: file_name.trim_end_matches(".md").to_string(),
        ext: "md".to_string(),
        size,
        ctime,
        mtime,
        tags: extracted.tags,
        links: extracted.links,
        backlinks: vec![],
        embeds: extracted.embeds,
        properties: extracted.frontmatter,
    };

    db.upsert_document(&doc)?;

    Ok(Some(doc))
}

fn update_backlinks(
    db: &Database,
    indexed_docs: &[Document],
) -> Result<(), Box<dyn std::error::Error>> {
    let link_map = db.get_all_links()?;
    let mut backlinks: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for (path, links) in &link_map {
        for link in links {
            let link_name = link.trim_end_matches(['|', '#']).to_string();
            backlinks.entry(link_name).or_default().push(path.clone());
        }
    }

    for doc in indexed_docs {
        if let Some(back_links) = backlinks.get(&doc.name) {
            let mut updated_doc = doc.clone();
            updated_doc.backlinks = back_links.clone();
            db.upsert_document(&updated_doc)?;
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

        let mtime = db.get_mtime("test.md").unwrap();
        assert!(mtime.is_some());

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
    fn test_index_skips_non_md_files() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, "readme.md", "# README");
        create_test_file(&test_dir, "notes.txt", "Some notes");
        create_test_file(&test_dir, "data.json", "{}");

        let db = Database::new(&db_path).unwrap();
        let result = index_directory(&test_dir, &db, false);
        assert!(result.is_ok());

        let link_map = db.get_all_links().unwrap();
        assert_eq!(link_map.len(), 1);

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

        let mtime = db.get_mtime("tagged.md").unwrap();
        assert!(mtime.is_some());

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

        let mtime = db.get_mtime("with_embeds.md").unwrap();
        assert!(mtime.is_some());

        cleanup(&test_dir, &db_path);
    }

    #[test]
    fn test_index_force_reindex() {
        let (test_dir, db_path) = create_test_directory();

        create_test_file(&test_dir, "test.md", "# Original");

        let db = Database::new(&db_path).unwrap();

        // First index
        index_directory(&test_dir, &db, false).unwrap();
        let mtime1 = db.get_mtime("test.md").unwrap();

        // Wait a bit and update file
        std::thread::sleep(std::time::Duration::from_millis(100));
        create_test_file(&test_dir, "test.md", "# Updated");

        // Re-index with force
        index_directory(&test_dir, &db, true).unwrap();
        let mtime2 = db.get_mtime("test.md").unwrap();

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
}
