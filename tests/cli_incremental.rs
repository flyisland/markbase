mod common;

use common::{assert_cli_success, parse_index_stats, stdout_contains, TestVault};

fn setup_basic_vault(vault: &TestVault) {
    vault.create_note("note1", "# Note 1");
    vault.create_note("note2", "# Note 2");
    vault.create_note("note3", "# Note 3");
    vault.index();
}

#[test]
fn test_incremental_new_file() {
    let vault = TestVault::new();
    setup_basic_vault(&vault);

    vault.create_note("note4", "# Note 4");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.new, 1);
    let query_output = vault.query("");
    let stdout = String::from_utf8_lossy(&query_output.stdout).to_string();
    assert!(stdout.contains("note4"));
}

#[test]
fn test_incremental_modified_file() {
    let vault = TestVault::new();
    setup_basic_vault(&vault);

    std::thread::sleep(std::time::Duration::from_millis(100));
    vault.create_note("note1", "# Note 1 Updated\n\nNew content here.");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.updated, 1);
}

#[test]
fn test_incremental_size_change() {
    let vault = TestVault::new();
    vault.create_note("note1", "# Short");
    vault.index();

    vault.create_note(
        "note1",
        "# Much longer content here\n\nWith multiple lines\n\nAnd more content",
    );

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.updated, 1);
}

#[test]
fn test_incremental_deleted_file() {
    let vault = TestVault::new();
    setup_basic_vault(&vault);

    vault.delete_file("note3.md");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.deleted, 1);
}

#[test]
fn test_incremental_unchanged_skipped() {
    let vault = TestVault::new();
    setup_basic_vault(&vault);

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.new, 0);
    assert_eq!(stats.updated, 0);
}

#[test]
fn test_incremental_backlinks_after_rename() {
    let vault = TestVault::new();
    vault.create_note("a", "Links to [[b]].");
    vault.create_note("b", "# B");
    vault.index();

    vault.note_rename("b", "b-new");

    vault.index();

    let output = vault.query("name == 'b-new'");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("b-new"));
}

#[test]
fn test_incremental_multiple_changes() {
    let vault = TestVault::new();
    setup_basic_vault(&vault);

    vault.create_note("note4", "# Note 4");
    vault.create_note("note5", "# Note 5");
    vault.delete_file("note2.md");
    vault.create_note("note1", "# Note 1 Modified");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert!(stats.new >= 1);
    assert!(stats.deleted >= 1);
    assert!(stats.updated >= 1);
}

#[test]
fn test_incremental_after_force_index() {
    let vault = TestVault::new();
    setup_basic_vault(&vault);

    vault.index_force();

    vault.create_note("note4", "# Note 4");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert!(stats.new >= 1);
}

#[test]
fn test_incremental_gitignore_change() {
    let vault = TestVault::new();
    vault.create_note("keep", "# Keep");
    vault.create_note("skip", "# Skip");
    vault.index();

    vault.create_gitignore("skip.md\n");
    vault.index_force();

    let _stats = parse_index_stats(&vault.index());
    assert!(stdout_contains(&vault.index(), "skip.md") == false);
}

#[test]
fn test_incremental_tags_update() {
    let vault = TestVault::new();
    vault.create_note("note", "# Note\nNo tags");
    vault.index();

    std::thread::sleep(std::time::Duration::from_millis(100));
    vault.create_note("note", "# Note\n\nNow with #newtag");

    vault.index();

    let output = vault.query("list_contains(tags, 'newtag')");
    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("note"));
}

#[test]
fn test_incremental_links_update() {
    let vault = TestVault::new();
    vault.create_note("a", "# A");
    vault.create_note("b", "# B");
    vault.index();

    std::thread::sleep(std::time::Duration::from_millis(100));
    vault.create_note("a", "# A\n\nSee [[b]].");

    vault.index();

    let output = vault.query("list_contains(links, 'b')");
    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("a"));
}

#[test]
fn test_incremental_large_vault() {
    let vault = TestVault::new();

    for i in 0..50 {
        vault.create_note(&format!("note{}", i), &format!("# Note {}", i));
    }
    vault.index();

    vault.create_note("note50", "# Note 50");
    vault.create_note("note51", "# Note 51");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert!(stats.new >= 2);
}

#[test]
fn test_incremental_empty_to_populated() {
    let vault = TestVault::new();
    vault.index();

    vault.create_note("first", "# First");
    vault.create_note("second", "# Second");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.total, 2);
}

#[test]
fn test_incremental_all_deleted() {
    let vault = TestVault::new();
    setup_basic_vault(&vault);

    vault.delete_file("note1.md");
    vault.delete_file("note2.md");
    vault.delete_file("note3.md");

    let output = vault.index();

    assert_cli_success(&output);
    let stats = parse_index_stats(&output);
    assert_eq!(stats.deleted, 3);
    assert_eq!(stats.total, 0);
}
