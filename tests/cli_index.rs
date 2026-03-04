mod common;

use common::{assert_cli_success, parse_index_stats, stdout_contains, TestVault};

#[test]
fn test_index_empty_vault() {
    let vault = TestVault::new();
    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 0);
    assert!(stdout_contains(&output, "0 files indexed"));
}

#[test]
fn test_index_single_note() {
    let vault = TestVault::new();
    vault.create_note("test", "# Test Note\n\nThis is content.");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 1);
    assert_eq!(stats.new, 1);
}

#[test]
fn test_index_multiple_notes() {
    let vault = TestVault::new();
    vault.create_note("note1", "# Note 1");
    vault.create_note("note2", "# Note 2");
    vault.create_note("note3", "# Note 3");
    vault.create_note("note4", "# Note 4");
    vault.create_note("note5", "# Note 5");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 5);
    assert_eq!(stats.new, 5);
}

#[test]
fn test_index_with_subdirectories() {
    let vault = TestVault::new();
    vault.create_note("root", "# Root");
    vault.create_note_in_subdir("subdir1", "sub1", "# Sub 1");
    vault.create_note_in_subdir("subdir2/deep", "deep", "# Deep");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 3);
}

#[test]
fn test_index_non_md_files() {
    let vault = TestVault::new();
    vault.create_note("readme", "# README");
    vault.create_file("notes.txt", "Some text notes");
    vault.create_file("data.json", r#"{"key": "value"}"#);
    vault.create_file("image.png", "fake png content");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 4);
}

#[test]
fn test_index_with_frontmatter() {
    let vault = TestVault::new();
    let content = r#"---
title: My Document
author: John Doe
tags: [work, project]
---

# Content
"#;
    vault.create_note("doc", content);

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 1);
}

#[test]
fn test_index_with_tags() {
    let vault = TestVault::new();
    vault.create_note(
        "tagged",
        "# Tagged Note\n\nThis has #tag1 and #tag-2 and #nested/tag in content.",
    );

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 1);
}

#[test]
fn test_index_with_wikilinks() {
    let vault = TestVault::new();
    vault.create_note("source", "See [[target-note]] for details.");
    vault.create_note("target", "# Target Note");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 2);
}

#[test]
fn test_index_with_embeds() {
    let vault = TestVault::new();
    vault.create_note(
        "with-embeds",
        "Check ![[diagram.png]] and ![[chart.svg]] for visuals.",
    );

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 1);
}

#[test]
fn test_index_force_rebuild() {
    let vault = TestVault::new();
    vault.create_note("test", "# Original");

    vault.index();

    std::thread::sleep(std::time::Duration::from_millis(100));
    vault.create_note("test", "# Updated Content");

    let output = vault.index_force();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert!(stats.updated >= 1 || stats.new >= 1);
}

#[test]
fn test_index_gitignore_file_pattern() {
    let vault = TestVault::new();
    vault.create_gitignore("skip.md\n");
    vault.create_note("test", "# Test");
    vault.create_note("skip", "# Skip this");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 1);
}

#[test]
fn test_index_gitignore_directory_pattern() {
    let vault = TestVault::new();
    vault.create_gitignore("private/\n");
    vault.create_note("main", "# Main");
    vault.create_note_in_subdir("private", "secret", "# Secret");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 1);
}

#[test]
fn test_index_gitignore_negation_pattern() {
    let vault = TestVault::new();
    vault.create_gitignore("*.md\n!important.md\n");
    vault.create_note("test", "# Test");
    vault.create_note("important", "# Important");
    vault.create_note("other", "# Other");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 1);
}

#[test]
fn test_index_markbaseignore_respected() {
    let vault = TestVault::new();
    vault.create_markbaseignore("skip.md\n");
    vault.create_note("test", "# Test");
    vault.create_note("skip", "# Skip this");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 1);
}

#[test]
fn test_index_markbaseignore_takes_precedence() {
    let vault = TestVault::new();
    vault.create_gitignore("keep.md\n");
    vault.create_markbaseignore("skip.md\n");
    vault.create_note("keep", "# Keep");
    vault.create_note("skip", "# Skip");
    vault.create_note("other", "# Other");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 1);
}

#[test]
fn test_index_verbose_output() {
    let vault = TestVault::new();
    vault.create_note("test", "# Test");

    let (output, stdout, stderr) = vault.run_cli_verbose(&["index", "--verbose"]);

    assert_cli_success(&output);
    let stdout_str = stdout.as_str();
    let stderr_str = stderr.as_str();
    assert!(stdout_str.contains("+ test.md") || stderr_str.contains("+ test.md"));
}

#[test]
fn test_index_name_conflict_warning() {
    let vault = TestVault::new();
    vault.create_note("same", "# Note 1");
    vault.create_note_in_subdir("subdir", "same", "# Note 2");

    let (output, _stdout, stderr) = vault.run_cli_verbose(&["index"]);

    assert_cli_success(&output);
    let stderr_str = stderr.as_str();
    assert!(stderr_str.contains("conflict"));
}

#[test]
fn test_index_creates_database() {
    let vault = TestVault::new();
    vault.create_note("test", "# Test");

    vault.index();

    assert!(vault.db_path().exists());
}
