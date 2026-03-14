mod common;

use common::{TestVault, assert_cli_success, parse_index_stats, stderr_contains, stdout_contains};

#[test]
fn test_auto_index_empty_vault() {
    let vault = TestVault::new();
    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 0);
}

#[test]
fn test_auto_index_single_note() {
    let vault = TestVault::new();
    vault.create_note("test", "# Test Note\n\nThis is content.");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 1);
    assert_eq!(stats.new, 1);
}

#[test]
fn test_auto_index_multiple_notes() {
    let vault = TestVault::new();
    vault.create_note("note1", "# Note 1");
    vault.create_note("note2", "# Note 2");
    vault.create_note("note3", "# Note 3");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 3);
    assert_eq!(stats.new, 3);
}

#[test]
fn test_auto_index_respects_ignores() {
    let vault = TestVault::new();
    vault.create_gitignore("skip.md\n");
    vault.create_markbaseignore("secret/\n");
    vault.create_note("keep", "# Keep");
    vault.create_note("skip", "# Skip");
    vault.create_note_in_subdir("secret", "hidden", "# Hidden");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 1);
}

#[test]
fn test_auto_index_verbose_output_lists_changes() {
    let vault = TestVault::new();
    vault.create_note("test", "# Test");

    let (output, _stdout, stderr) = vault.index_verbose();

    assert_cli_success(&output);
    assert!(
        stderr.contains("+ test.md"),
        "unexpected stderr: {}",
        stderr
    );
}

#[test]
fn test_auto_index_verbose_output_shows_name_conflict_warning() {
    let vault = TestVault::new();
    vault.create_note("same", "# Note 1");
    vault.create_note_in_subdir("subdir", "same", "# Note 2");

    let (output, _stdout, stderr) = vault.index_verbose();

    assert_cli_success(&output);
    assert!(stderr.contains("conflict"), "unexpected stderr: {}", stderr);
}

#[test]
fn test_auto_index_creates_database() {
    let vault = TestVault::new();
    vault.create_note("test", "# Test");

    let output = vault.index();

    assert_cli_success(&output);
    assert!(vault.db_path().exists());
}

#[test]
fn test_auto_index_summary_is_silent_for_regular_query() {
    let vault = TestVault::new();
    vault.create_note("test", "# Test");

    let output = vault.query("file.name == 'test'");

    assert_cli_success(&output);
    assert!(stdout_contains(&output, "test"));
    assert!(!stderr_contains(&output, "Indexed:"));
}

#[test]
fn test_index_command_removed() {
    let vault = TestVault::new();
    let output = vault.run_cli(&["index"]);
    assert!(!output.status.success());
    assert!(stderr_contains(&output, "unrecognized subcommand 'index'"));
}

#[test]
fn test_auto_index_extracts_escaped_pipe_and_mixed_frontmatter_links() {
    let vault = TestVault::new();
    vault.create_note(
        "source",
        r#"---
related: "see [[front-note]] and ![[ignored-note]]"
---

| ref |
| --- |
| [[body-note\|Alias]] |

![[diagram.png\|200]]
"#,
    );
    vault.index();

    let output =
        vault.query("file.name == 'source' AND list_contains(file.links, 'front-note') AND list_contains(file.links, 'body-note') AND list_contains(file.embeds, 'diagram.png')");

    assert_cli_success(&output);
    assert!(stdout_contains(&output, "source"));
}
