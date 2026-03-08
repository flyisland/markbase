mod common;

use common::{TestVault, assert_cli_success, stdout_contains};

fn setup_vault_with_notes(vault: &TestVault) {
    vault.create_note(
        "readme",
        r#"---
title: README
---

# README

#documentation #important
"#,
    );

    vault.create_note(
        "todo",
        r#"---
title: Todo List
status: in-progress
---

# Todo

#todo #work
"#,
    );

    vault.create_note(
        "architecture",
        r#"---
title: Architecture
author: team
---

# Architecture

#technical #design

See [[api-design]] for API details.
"#,
    );

    vault.create_note(
        "api-design",
        "# API Design\n\n#api #technical\n\nAPI documentation here.",
    );

    vault.create_note(
        "project-x",
        r#"---
title: Project X
year: 2023
---

# Project X

#project #archived #legacy
"#,
    );

    vault.index();
}

#[test]
fn test_query_all_notes() {
    let vault = TestVault::new();
    vault.create_note("test", "# Test");
    vault.index();

    let output = vault.query("");

    assert_cli_success(&output);
    assert!(stdout_contains(&output, "test.md") || stdout_contains(&output, "test"));
}

#[test]
fn test_query_expression_where() {
    let vault = TestVault::new();
    setup_vault_with_notes(&vault);

    let output = vault.query("file.name == 'readme'");

    assert_cli_success(&output);
    assert!(stdout_contains(&output, "readme"));
}

#[test]
fn test_query_sql_mode() {
    let vault = TestVault::new();
    setup_vault_with_notes(&vault);

    let output = vault.query("SELECT file.name, title FROM notes");

    assert_cli_success(&output);
    assert!(stdout_contains(&output, "readme"));
}

#[test]
fn test_query_reserved_fields() {
    let vault = TestVault::new();
    vault.create_note("test", "# Test");
    vault.index();

    let output = vault.query("file.name == 'test'");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test"));
}

#[test]
fn test_query_frontmatter_field() {
    let vault = TestVault::new();
    setup_vault_with_notes(&vault);

    let output = vault.query("title == 'README'");

    assert_cli_success(&output);
    assert!(stdout_contains(&output, "readme"));
}

#[test]
fn test_query_tags_array() {
    let vault = TestVault::new();
    setup_vault_with_notes(&vault);

    let output = vault.query("list_contains(file.tags, 'todo')");

    assert_cli_success(&output);
    assert!(stdout_contains(&output, "todo"));
}

#[test]
fn test_query_multiple_tags() {
    let vault = TestVault::new();
    setup_vault_with_notes(&vault);

    let output = vault.query("list_contains(file.tags, 'technical')");

    assert_cli_success(&output);
    assert!(stdout_contains(&output, "architecture"));
}

#[test]
fn test_query_output_table() {
    let vault = TestVault::new();
    vault.create_note("test", "# Test");
    vault.index();

    let output = vault.query_format("", "table");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("name") || stdout.contains("path") || stdout.contains("test"));
}

#[test]
fn test_query_output_json() {
    let vault = TestVault::new();
    vault.create_note("test", "# Test");
    vault.index();

    let output = vault.query_format("", "json");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"file.path\"")
            || stdout.contains("\"file.name\"")
            || stdout.contains("\"description\"")
    );
}

#[test]
fn test_query_rejects_list_output() {
    let vault = TestVault::new();
    vault.create_note("test", "# Test");
    vault.index();

    let output = vault.query_format("", "list");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value 'list'"));
    assert!(stderr.contains("possible values: json, table"));
}

#[test]
fn test_query_abs_path() {
    let vault = TestVault::new();
    vault.create_note("test", "# Test");
    vault.index();

    let output = vault.query_abs_path("");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("/") || stdout.contains("\\"));
}

#[test]
fn test_query_dry_run() {
    let vault = TestVault::new();
    vault.create_note("test", "# Test");
    vault.index();

    let output = vault.query_dry_run("file.name == 'test'");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SELECT") || stdout.contains("json_extract"));
}

#[test]
fn test_query_nested_frontmatter_field() {
    let vault = TestVault::new();
    vault.create_note(
        "nested",
        r#"---
meta:
  author: John
  version: 1.0
---

# Nested
"#,
    );
    vault.index();

    let output = vault.query("meta.author == 'John'");

    assert_cli_success(&output);
    assert!(stdout_contains(&output, "nested"));
}

#[test]
fn test_query_order_by() {
    let vault = TestVault::new();
    vault.create_note("aaa", "# AAA");
    vault.create_note("zzz", "# ZZZ");
    vault.index();

    let output = vault.query("ORDER BY file.name ASC");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let aaa_pos = stdout.find("aaa").unwrap_or(usize::MAX);
    let zzz_pos = stdout.find("zzz").unwrap_or(0);
    assert!(aaa_pos < zzz_pos, "Expected aaa before zzz: {}", stdout);
}

#[test]
fn test_query_limit() {
    let vault = TestVault::new();
    vault.create_note("note1", "# Note 1");
    vault.create_note("note2", "# Note 2");
    vault.create_note("note3", "# Note 3");
    vault.index();

    let output = vault.query("SELECT file.name FROM notes LIMIT 2");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let rows: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let count = rows.as_array().map(|items| items.len()).unwrap_or_default();
    assert!(count <= 2, "unexpected output: {}", stdout);
}

#[test]
fn test_query_no_results() {
    let vault = TestVault::new();
    vault.create_note("test", "# Test");
    vault.index();

    let output = vault.query("file.name == 'nonexistent'");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "[]", "unexpected output: {}", stdout);
}

#[test]
fn test_query_with_type_cast() {
    let vault = TestVault::new();
    vault.create_note(
        "with-year",
        r#"---
year: 2024
---

# With Year
"#,
    );
    vault.index();

    let output = vault.query("year::INTEGER >= 2024");

    assert_cli_success(&output);
}

#[test]
fn test_query_links_field() {
    let vault = TestVault::new();
    vault.create_note("a", "See [[b]] and [[c]].");
    vault.create_note("b", "# B");
    vault.create_note("c", "# C");
    vault.index();

    let output = vault.query("list_contains(file.links, 'b')");

    assert_cli_success(&output);
    assert!(stdout_contains(&output, "a"));
}

#[test]
fn test_query_backlinks() {
    let vault = TestVault::new();
    vault.create_note("source", "See [[target]].");
    vault.create_note("target", "# Target");
    vault.index();

    let output = vault.query_with_backlinks("list_contains(file.backlinks, 'source')");

    assert_cli_success(&output);
    assert!(stdout_contains(&output, "target"));
}

#[test]
fn test_query_is_null() {
    let vault = TestVault::new();
    vault.create_note("with-author", r#"---author: John---"#);
    vault.create_note("no-author", "# No Author");
    vault.index();

    let output = vault.query("author IS NULL");

    assert_cli_success(&output);
}

#[test]
fn test_query_not_equal() {
    let vault = TestVault::new();
    setup_vault_with_notes(&vault);

    let output = vault.query("file.name != 'readme'");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("readme") || stdout.matches("readme").count() < 4);
}

#[test]
fn test_query_default_output_includes_description() {
    let vault = TestVault::new();
    vault.create_note(
        "test",
        r#"---
description: Test description
---

# Test
"#,
    );
    vault.index();

    let output = vault.query("file.name == 'test'");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("description"));
    assert!(stdout.contains("Test description"));
}
