mod common;

use common::{assert_cli_error, assert_cli_success, TestVault};

#[test]
fn test_note_create_simple() {
    let vault = TestVault::new();

    let output = vault.note_new("my-note");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("my-note.md"));
    assert!(vault.path.join("my-note.md").exists());
}

#[test]
fn test_note_create_duplicate() {
    let vault = TestVault::new();
    vault.create_note("existing", "# Existing");

    let output = vault.note_new("existing");

    assert_cli_error(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("exists") || stderr.contains("already"));
}

#[test]
fn test_note_create_with_template() {
    let vault = TestVault::new();
    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("daily.md"),
        r#"---
template: daily
---

# {{name}}
Date: {{date}}
"#,
    )
    .unwrap();
    vault.index();

    let output = vault.note_new_with_template("today", "daily");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("today") || stdout.contains("Date:"));
}

#[test]
fn test_note_create_invalid_template() {
    let vault = TestVault::new();

    let output = vault.note_new_with_template("test", "nonexistent");

    assert_cli_error(&output);
}

#[test]
fn test_note_rename_simple() {
    let vault = TestVault::new();
    vault.create_note("old-name", "# Old Name");
    vault.index();

    let output = vault.note_rename("old-name", "new-name");

    assert_cli_success(&output);
    assert!(vault.path.join("old-name.md").exists() == false);
    assert!(vault.path.join("new-name.md").exists());
}

#[test]
fn test_note_rename_updates_links() {
    let vault = TestVault::new();
    vault.create_note("page-a", "See [[page-b]] for details.");
    vault.create_note("page-b", "# Page B");
    vault.index();

    let output = vault.note_rename("page-b", "page-b-new");

    assert_cli_success(&output);
    let content = std::fs::read_to_string(vault.path.join("page-a.md")).unwrap();
    assert!(content.contains("[[page-b-new]]"));
    assert!(!content.contains("[[page-b]]"));
}

#[test]
fn test_note_rename_updates_embeds() {
    let vault = TestVault::new();
    vault.create_note("page-a", "![[diagram-old]]");
    vault.create_note("diagram-old", "# Diagram");
    vault.index();

    let output = vault.note_rename("diagram-old", "diagram-new");

    assert_cli_success(&output);
    let content = std::fs::read_to_string(vault.path.join("page-a.md")).unwrap();
    assert!(content.contains("![[diagram-new]]"));
    assert!(!content.contains("![[diagram-old]]"));
}

#[test]
fn test_note_rename_frontmatter_links() {
    let vault = TestVault::new();
    vault.create_note(
        "note-a",
        r#"---
related: "[[note-b]]"
---

Content
"#,
    );
    vault.create_note("note-b", "# Note B");
    vault.index();

    let output = vault.note_rename("note-b", "note-b-new");

    assert_cli_success(&output);
    let content = std::fs::read_to_string(vault.path.join("note-a.md")).unwrap();
    assert!(content.contains("[[note-b-new]]"));
    assert!(!content.contains("[[note-b]]"));
}

#[test]
fn test_note_rename_preserves_aliases() {
    let vault = TestVault::new();
    vault.create_note("source", "See [[old-note|Old Alias]] for info.");
    vault.create_note("old-note", "# Old Note");
    vault.index();

    let output = vault.note_rename("old-note", "new-note");

    assert_cli_success(&output);
    let content = std::fs::read_to_string(vault.path.join("source.md")).unwrap();
    assert!(content.contains("[[new-note|Old Alias]]"));
}

#[test]
fn test_note_rename_preserves_sections() {
    let vault = TestVault::new();
    vault.create_note("source", "See [[old-note#Section]] for info.");
    vault.create_note("old-note", "# Old Note");
    vault.index();

    let output = vault.note_rename("old-note", "new-note");

    assert_cli_success(&output);
    let content = std::fs::read_to_string(vault.path.join("source.md")).unwrap();
    assert!(content.contains("[[new-note#Section]]"));
}

#[test]
fn test_note_rename_preserves_section_and_alias() {
    let vault = TestVault::new();
    vault.create_note("source", "See [[old-note#Heading|Alias Text]] for info.");
    vault.create_note("old-note", "# Old Note");
    vault.index();

    let output = vault.note_rename("old-note", "new-note");

    assert_cli_success(&output);
    let content = std::fs::read_to_string(vault.path.join("source.md")).unwrap();
    assert!(content.contains("[[new-note#Heading|Alias Text]]"));
}

#[test]
fn test_note_rename_not_found() {
    let vault = TestVault::new();
    vault.create_note("existing", "# Existing");
    vault.index();

    let output = vault.note_rename("nonexistent", "new-name");

    assert_cli_error(&output);
}

#[test]
fn test_note_rename_target_exists() {
    let vault = TestVault::new();
    vault.create_note("note-a", "# Note A");
    vault.create_note("note-b", "# Note B");
    vault.index();

    let output = vault.note_rename("note-a", "note-b");

    assert_cli_error(&output);
}

#[test]
fn test_note_rename_updates_backlinks() {
    let vault = TestVault::new();
    vault.create_note("linking", "See [[target]].");
    vault.create_note("target", "# Target");
    vault.index();

    vault.note_rename("target", "renamed-target");

    let query_output = vault.query("name == 'renamed-target'");
    let stdout = String::from_utf8_lossy(&query_output.stdout);
    assert!(stdout.contains("renamed-target"));
}

#[test]
fn test_note_rename_multiple_linkers() {
    let vault = TestVault::new();
    vault.create_note("a", "Link to [[target]].");
    vault.create_note("b", "Also [[target]].");
    vault.create_note("c", "And [[target]] too.");
    vault.create_note("target", "# Target");
    vault.index();

    let output = vault.note_rename("target", "new-target");

    assert_cli_success(&output);

    for name in &["a", "b", "c"] {
        let content = std::fs::read_to_string(vault.path.join(format!("{}.md", name))).unwrap();
        assert!(content.contains("[[new-target]]"));
    }
}

#[test]
fn test_note_rename_case_change() {
    let vault = TestVault::new();
    vault.create_note("oldnote", "# Old Note");
    vault.index();

    let output = vault.note_rename("oldnote", "newnote");

    assert_cli_success(&output);
    assert!(vault.path.join("newnote.md").exists());
}

#[test]
fn test_note_new_updates_index() {
    let vault = TestVault::new();
    vault.index();

    vault.note_new("brand-new");
    vault.index();

    let query_output = vault.query("name == 'brand-new'");
    let stdout = String::from_utf8_lossy(&query_output.stdout);
    assert!(stdout.contains("brand-new") || stdout.contains("brand-new.md"));
}
