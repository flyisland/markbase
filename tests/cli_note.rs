mod common;

use common::{TestVault, assert_cli_error, assert_cli_success, stderr_contains, stdout_contains};
use serde_json::Value;

#[test]
fn test_note_verify_note_not_found() {
    let vault = TestVault::new();
    vault.index();

    let output = vault.note_verify("nonexistent");

    assert_cli_error(&output);
    assert!(stderr_contains(&output, "not found"));
}

#[test]
fn test_note_verify_rejects_path_like_name() {
    let vault = TestVault::new();

    let output = vault.note_verify("logs/opportunities/test-note");

    assert_cli_error(&output);
    assert!(stderr_contains(&output, "must not include directories"));
    assert!(!stderr_contains(&output, "not found in the vault"));
}

#[test]
fn test_note_verify_rejects_extension_in_name() {
    let vault = TestVault::new();

    let output = vault.note_verify("test-note.md");

    assert_cli_error(&output);
    assert!(stderr_contains(
        &output,
        "must not include a file extension"
    ));
    assert!(!stderr_contains(&output, "not found in the vault"));
}

#[test]
fn test_note_verify_no_templates_field() {
    let vault = TestVault::new();
    vault.create_note("test-note", "# Test Note");
    vault.index();

    let output = vault.note_verify("test-note");

    assert_cli_error(&output);
    assert!(stderr_contains(&output, "no 'templates'"));
}

#[test]
fn test_note_verify_invalid_template_link_format() {
    let vault = TestVault::new();
    vault.create_note(
        "test-note",
        r#"---
templates: ["company_customer"]
---

# Test
"#,
    );
    vault.index();

    let output = vault.note_verify("test-note");

    assert_cli_error(&output);
    assert!(stderr_contains(&output, "invalid link"));
}

#[test]
fn test_note_verify_template_file_not_found() {
    let vault = TestVault::new();
    vault.create_note(
        "test-note",
        r#"---
templates: ["[[ghost_template]]"]
---

# Test
"#,
    );
    vault.index();

    let output = vault.note_verify("test-note");

    assert_cli_error(&output);
    assert!(stderr_contains(&output, "not found"));
}

#[test]
fn test_note_verify_location_mismatch() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company_customer.md"),
        r#"---
type: company
_schema:
  location: company/
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "acme",
        r#"---
templates: ["[[company_customer]]"]
type: company
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "requires location"));
}

#[test]
fn test_note_verify_required_field_missing() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company_customer.md"),
        r#"---
type: company
_schema:
  required:
    - industry
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "acme",
        r#"---
templates: ["[[company_customer]]"]
type: company
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "required field 'industry'"));
}

#[test]
fn test_note_verify_type_mismatch() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company_customer.md"),
        r#"---
type: company
_schema:
  properties:
    count:
      type: number
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "acme",
        r#"---
templates: ["[[company_customer]]"]
type: company
count: "not-a-number"
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "type mismatch"));
}

#[test]
fn test_note_verify_enum_failure() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company_customer.md"),
        r#"---
type: company
_schema:
  properties:
    size:
      type: text
      enum: [startup, smb, enterprise]
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "acme",
        r#"---
templates: ["[[company_customer]]"]
type: company
size: invalid
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "invalid value"));
    assert!(stderr_contains(&output, "startup"));
}

#[test]
fn test_note_verify_link_target_type_mismatch() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company_customer.md"),
        r#"---
type: company
_schema:
  properties:
    related:
      type: text
      format: link
      target: person
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "david-chen",
        r#"---
type: meeting
---

# David Chen
"#,
    );
    vault.create_note(
        "acme",
        r#"---
templates: ["[[company_customer]]"]
type: company
related: "[[david-chen]]"
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "requires target type 'person'"));
}

#[test]
fn test_note_verify_template_without_schema() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("simple_template.md"),
        r#"---
type: simple
---

# Simple Template
"#,
    )
    .unwrap();

    vault.create_note(
        "test-note",
        r#"---
templates: ["[[simple_template]]"]
type: simple
description: Test note
---

# Test
"#,
    );
    vault.index();

    let output = vault.note_verify("test-note");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("passed all checks"));
}

#[test]
fn test_note_verify_templates_field_not_array() {
    let vault = TestVault::new();
    vault.create_note(
        "test-note",
        r#"---
templates: not-an-array
---

# Test
"#,
    );
    vault.index();

    let output = vault.note_verify("test-note");

    assert_cli_error(&output);
    assert!(stderr_contains(&output, "no 'templates'"));
}

#[test]
fn test_note_verify_multi_template_type_conflict() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("template_a.md"),
        r#"---
type: template_a
_schema:
  properties:
    industry:
      type: text
---

# Template A
"#,
    )
    .unwrap();
    std::fs::write(
        templates_dir.join("template_b.md"),
        r#"---
type: template_b
_schema:
  properties:
    industry:
      type: list
---

# Template B
"#,
    )
    .unwrap();

    vault.create_note(
        "test-note",
        r#"---
templates: ["[[template_a]]", "[[template_b]]"]
type: test
---

# Test
"#,
    );
    vault.index();

    let output = vault.note_verify("test-note");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "conflicting type definitions"));
}

#[test]
fn test_note_verify_required_field_empty_string() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company_customer.md"),
        r#"---
type: company
_schema:
  required:
    - industry
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "acme",
        r#"---
templates: ["[[company_customer]]"]
type: company
industry: ""
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "required field 'industry'"));
}

#[test]
fn test_note_verify_boolean_type_validation() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("task.md"),
        r#"---
type: task
_schema:
  properties:
    completed:
      type: boolean
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "my-task",
        r#"---
templates: ["[[task]]"]
type: task
completed: "not-a-boolean"
---

# My Task
"#,
    );
    vault.index();

    let output = vault.note_verify("my-task");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "type mismatch"));
}

#[test]
fn test_note_verify_date_format_validation() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("event.md"),
        r#"---
type: event
_schema:
  properties:
    date:
      type: date
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "my-event",
        r#"---
templates: ["[[event]]"]
type: event
date: "not-a-date"
---

# My Event
"#,
    );
    vault.index();

    let output = vault.note_verify("my-event");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "type mismatch"));
}

#[test]
fn test_note_verify_valid_date_format() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("event.md"),
        r#"---
type: event
_schema:
  properties:
    date:
      type: date
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "my-event",
        r#"---
templates: ["[[event]]"]
type: event
description: Holiday event
date: "2024-12-25"
---

# My Event
"#,
    );
    vault.index();

    let output = vault.note_verify("my-event");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("passed all checks"));
}

#[test]
fn test_note_verify_datetime_format_validation() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("meeting.md"),
        r#"---
type: meeting
_schema:
  properties:
    scheduled_at:
      type: datetime
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "my-meeting",
        r#"---
templates: ["[[meeting]]"]
type: meeting
scheduled_at: "not-a-datetime"
---

# My Meeting
"#,
    );
    vault.index();

    let output = vault.note_verify("my-meeting");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "type mismatch"));
}

#[test]
fn test_note_verify_list_type_validation() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("project.md"),
        r#"---
type: project
_schema:
  properties:
    tags:
      type: list
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "my-project",
        r#"---
templates: ["[[project]]"]
type: project
tags: "not-a-list"
---

# My Project
"#,
    );
    vault.index();

    let output = vault.note_verify("my-project");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "type mismatch"));
}

#[test]
fn test_note_verify_enum_for_list_type() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("project.md"),
        r#"---
type: project
_schema:
  properties:
    priorities:
      type: list
      enum: [high, medium, low]
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "my-project",
        r#"---
templates: ["[[project]]"]
type: project
priorities: [high, invalid-priority, low]
---

# My Project
"#,
    );
    vault.index();

    let output = vault.note_verify("my-project");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "invalid value"));
}

#[test]
fn test_note_verify_invalid_wiki_link_format() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company.md"),
        r#"---
type: company
_schema:
  properties:
    related:
      type: text
      format: link
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "acme",
        r#"---
templates: ["[[company]]"]
type: company
related: "not-a-wikilink"
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "invalid link format"));
}

#[test]
fn test_note_verify_link_field_accepts_md_anchor_alias_form() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company.md"),
        r#"---
type: company
_schema:
  properties:
    related:
      type: text
      format: link
---

# Template
"#,
    )
    .unwrap();

    vault.create_note("target-note", "---\ntype: company\n---\n");
    vault.create_note(
        "acme",
        r#"---
templates: ["[[company]]"]
type: company
related: "[[folder/target-note.md#Heading|Alias]]"
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("invalid link format"));
    assert!(!stderr.contains("not found in the vault"));
}

#[test]
fn test_note_verify_link_field_rejects_non_pure_wikilink_string() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company.md"),
        r#"---
type: company
_schema:
  properties:
    related:
      type: text
      format: link
---

# Template
"#,
    )
    .unwrap();

    vault.create_note("target-note", "---\ntype: company\n---\n");
    vault.create_note(
        "acme",
        r#"---
templates: ["[[company]]"]
type: company
related: "prefix [[target-note]] suffix"
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "invalid link format"));
}

#[test]
fn test_note_verify_link_field_accepts_trimmed_pure_wikilink() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company.md"),
        r#"---
type: company
_schema:
  properties:
    related:
      type: text
      format: link
---

# Template
"#,
    )
    .unwrap();

    vault.create_note("target-note", "---\ntype: company\n---\n");
    vault.create_note(
        "acme",
        r#"---
templates: ["[[company]]"]
type: company
related: "  [[target-note]]  "
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("invalid link format"));
    assert!(!stderr.contains("not found in the vault"));
}

#[test]
fn test_note_verify_link_to_nonexistent_note() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company.md"),
        r#"---
type: company
_schema:
  properties:
    related:
      type: text
      format: link
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "acme",
        r#"---
templates: ["[[company]]"]
type: company
related: "[[nonexistent-note]]"
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "not found in the vault"));
}

#[test]
fn test_note_verify_dangling_reference() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company.md"),
        r#"---
type: company
_schema:
  properties:
    related:
      type: text
      format: link
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "acme",
        r#"---
templates: ["[[company]]"]
type: company
related: "[?[[some-note]]]"
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "dangling reference"));
}

#[test]
fn test_note_verify_list_field_with_link_format() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company.md"),
        r#"---
type: company
_schema:
  properties:
    contacts:
      type: list
      format: link
      target: person
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "person-a",
        r#"---
type: person
---

# Person A
"#,
    );
    vault.create_note(
        "person-b",
        r#"---
type: company
---

# Person B (wrong type)
"#,
    );
    vault.create_note(
        "acme",
        r#"---
templates: ["[[company]]"]
type: company
contacts: ["[[person-a]]", "[[person-b]]"]
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "requires target type 'person'"));
}

#[test]
fn test_note_verify_list_field_value_containment() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("task.md"),
        r#"---
type: task
tags: [todo, important]
_schema:
  properties:
    tags:
      type: list
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "my-task",
        r#"---
templates: ["[[task]]"]
type: task
description: Task note
tags: [todo, important, extra-tag]
---

# My Task
"#,
    );
    vault.index();

    let output = vault.note_verify("my-task");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("passed all checks"));
}

#[test]
fn test_note_create_simple() {
    let vault = TestVault::new();

    let output = vault.note_new("my-note");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "inbox/my-note.md");
    assert!(vault.path.join("inbox").join("my-note.md").exists());
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
    assert_eq!(stdout.trim(), "inbox/today.md");
    assert!(vault.path.join("inbox").join("today.md").exists());
}

#[test]
fn test_note_create_invalid_template() {
    let vault = TestVault::new();

    let output = vault.note_new_with_template("test", "nonexistent");

    assert_cli_error(&output);
}

#[test]
fn test_note_create_rejects_directory_in_name() {
    let vault = TestVault::new();

    let output = vault.note_new("notes/my-note");

    assert_cli_error(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("must not include directories"));
}

#[test]
fn test_note_create_rejects_extension_in_name() {
    let vault = TestVault::new();

    let output = vault.note_new("my-note.md");

    assert_cli_error(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("must not include a file extension"));
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
fn test_note_rename_preserves_heading_block_and_alias_suffix() {
    let vault = TestVault::new();
    vault.create_note(
        "source",
        "See [[old-note#Heading|Alias Text]] and ![[old-note#^block-1]].",
    );
    vault.create_note("old-note", "# Old Note");
    vault.index();

    let output = vault.note_rename("old-note", "new-note");

    assert_cli_success(&output);
    let content = std::fs::read_to_string(vault.path.join("source.md")).unwrap();
    assert!(content.contains("[[new-note#Heading|Alias Text]]"));
    assert!(content.contains("![[new-note#^block-1]]"));
}

#[test]
fn test_note_rename_skips_code_context_links() {
    let vault = TestVault::new();
    vault.create_note(
        "source",
        "See [[old-note]].\n\n`[[old-note]]`\n\n```md\n![[old-note]]\n```\n",
    );
    vault.create_note("old-note", "# Old Note");
    vault.index();

    let output = vault.note_rename("old-note", "new-note");

    assert_cli_success(&output);
    let content = std::fs::read_to_string(vault.path.join("source.md")).unwrap();
    assert!(content.contains("See [[new-note]]."));
    assert!(content.contains("`[[old-note]]`"));
    assert!(content.contains("```md\n![[old-note]]\n```"));
}

#[test]
fn test_note_rename_updates_md_extension_and_path_forms() {
    let vault = TestVault::new();
    vault.create_note("source", "See [[folder/old-note.md#Section]] for details.");
    vault.create_note("old-note", "# Old Note");
    vault.index();

    let output = vault.note_rename("old-note", "new-note");

    assert_cli_success(&output);
    let content = std::fs::read_to_string(vault.path.join("source.md")).unwrap();
    assert!(content.contains("[[new-note#Section]]"));
    assert!(!content.contains("old-note.md"));
}

#[test]
fn test_note_rename_updates_frontmatter_wikilink_strings_with_shared_parser() {
    let vault = TestVault::new();
    vault.create_note(
        "note-a",
        r#"---
related: "  [[note-b|Alias]]  "
---

Content
"#,
    );
    vault.create_note("note-b", "# Note B");
    vault.index();

    let output = vault.note_rename("note-b", "note-b-new");

    assert_cli_success(&output);
    let content = std::fs::read_to_string(vault.path.join("note-a.md")).unwrap();
    assert!(content.contains("[[note-b-new|Alias]]"));
}

#[test]
fn test_note_rename_preserves_escaped_pipe_in_table_cells() {
    let vault = TestVault::new();
    vault.create_note("source", "| ref |\n| --- |\n| [[old-note\\|Alias]] |\n");
    vault.create_note("old-note", "# Old Note");
    vault.index();

    let output = vault.note_rename("old-note", "new-note");

    assert_cli_success(&output);
    let content = std::fs::read_to_string(vault.path.join("source.md")).unwrap();
    assert!(content.contains("[[new-note\\|Alias]]"));
    assert!(!content.contains("[[new-note|Alias]]"));
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
fn test_note_rename_rejects_path_like_old_name() {
    let vault = TestVault::new();

    let output = vault.note_rename("notes/old-name", "new-name");

    assert_cli_error(&output);
    assert!(stderr_contains(
        &output,
        "Invalid old_name 'notes/old-name'"
    ));
}

#[test]
fn test_note_rename_rejects_path_like_new_name() {
    let vault = TestVault::new();
    vault.create_file("aaa.jpeg", "image-bytes");

    let output = vault.note_rename("aaa.jpeg", "assets/bbb.jpeg");

    assert_cli_error(&output);
    assert!(stderr_contains(
        &output,
        "Invalid new_name 'assets/bbb.jpeg'"
    ));
}

#[test]
fn test_note_rename_allows_extension_bearing_resource_name() {
    let vault = TestVault::new();
    vault.create_file("aaa.jpeg", "image-bytes");

    let output = vault.note_rename("aaa.jpeg", "bbb.jpeg");

    assert_cli_success(&output);
    assert!(vault.path.join("bbb.jpeg").exists());
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

    let query_output = vault.query("file.name == 'renamed-target'");
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

    let query_output = vault.query("file.name == 'brand-new'");
    let stdout = String::from_utf8_lossy(&query_output.stdout);
    assert!(stdout.contains("brand-new") || stdout.contains("brand-new.md"));
}

#[test]
fn test_note_render_note_not_found() {
    let vault = TestVault::new();
    vault.index();

    let output = vault.run_cli(&["note", "render", "nonexistent"]);

    assert_cli_error(&output);
    assert!(stderr_contains(&output, "not found"));
}

#[test]
fn test_note_render_rejects_path_like_name() {
    let vault = TestVault::new();

    let output = vault.run_cli(&["note", "render", "logs/test-note"]);

    assert_cli_error(&output);
    assert!(stderr_contains(&output, "must not include directories"));
    assert!(!stderr_contains(&output, "not found in the vault"));
}

#[test]
fn test_note_render_accepts_base_filename() {
    let vault = TestVault::new();
    vault.create_file(
        "tasks.base",
        "views:
  - type: table
    name: Tasks
    order:
      - file.name
",
    );
    vault.index();

    let output = vault.run_cli(&["note", "render", "tasks.base"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rendered from tasks.base") || stdout.contains("```json"));
}

#[test]
fn test_note_render_rejects_non_base_extension() {
    let vault = TestVault::new();

    let output = vault.run_cli(&["note", "render", "diagram.png"]);

    assert_cli_error(&output);
    assert!(stderr_contains(
        &output,
        "render target must be a note name or .base filename"
    ));
}

#[test]
fn test_note_render_no_base_embeds() {
    let vault = TestVault::new();
    vault.create_note("test-note", "# Test Note\n\nSome content here.");
    vault.index();

    let output = vault.run_cli(&["note", "render", "test-note"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("Some content here"));
    assert!(stderr.is_empty());
}

#[test]
fn test_note_render_base_not_found() {
    let vault = TestVault::new();
    vault.create_note("test-note", "![[missing.base]]");
    vault.index();

    let output = vault.run_cli(&["note", "render", "test-note"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stderr_contains(&output, "not found"));
    assert!(stdout.contains("not found"));
}

#[test]
fn test_note_render_link_this_filter() {
    let vault = TestVault::new();
    vault.create_note("acme", "---\ntype: company\n---\n![[opps.base]]\n");
    vault.create_note(
        "deal1",
        "---\ntype: opportunity\nrelated_customer: \"[[acme]]\"\n---\n",
    );
    vault.create_file(
        "opps.base",
        "views:\n  - type: table\n    name: Opportunities\n    filters:\n      and:\n        - related_customer == link(this)\n    order:\n      - file.name\n",
    );
    vault.index();

    let output = vault.run_cli(&["note", "render", "acme"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("rendered from opps.base"));
    assert!(stdout.contains("[[deal1]]"));
    assert!(stderr.is_empty(), "unexpected stderr output: {}", stderr);
}

#[test]
fn test_note_render_base_embed_with_view_selector() {
    let vault = TestVault::new();
    vault.create_note("host", "![[tasks.base#Open Tasks]]");
    vault.create_note("task-open", "---\nstatus: open\n---\n");
    vault.create_note("task-done", "---\nstatus: closed\n---\n");
    vault.create_file(
        "tasks.base",
        "views:\n  - type: table\n    name: Open Tasks\n    filters:\n      and:\n        - status == \"open\"\n    order:\n      - file.name\n  - type: table\n    name: Closed Tasks\n    filters:\n      and:\n        - status == \"closed\"\n    order:\n      - file.name\n",
    );
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Open Tasks"));
    assert!(stdout.contains("[[task-open]]"));
    assert!(!stdout.contains("Closed Tasks"));
    assert!(!stdout.contains("[[task-done]]"));
}

#[test]
fn test_note_render_inline_base_embed_is_expanded() {
    let vault = TestVault::new();
    vault.create_note("host", "Before ![[tasks.base#Open Tasks]] After");
    vault.create_note("task-open", "---\nstatus: open\n---\n");
    vault.create_note("task-done", "---\nstatus: closed\n---\n");
    vault.create_file(
        "tasks.base",
        "views:\n  - type: table\n    name: Open Tasks\n    filters:\n      and:\n        - status == \"open\"\n    order:\n      - file.name\n  - type: table\n    name: Closed Tasks\n    filters:\n      and:\n        - status == \"closed\"\n    order:\n      - file.name\n",
    );
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Before "));
    assert!(stdout.contains(" After"));
    assert!(stdout.contains("Open Tasks"));
    assert!(stdout.contains("[[task-open]]"));
    assert!(!stdout.contains("Closed Tasks"));
    assert!(!stdout.contains("[[task-done]]"));
}

#[test]
fn test_note_render_embedded_note_uses_body_without_frontmatter() {
    let vault = TestVault::new();
    vault.create_note(
        "note-a",
        r#"---
status: active
tags: [alpha]
---
Embedded body line 1
Embedded body line 2
"#,
    );
    vault.create_note("host", "Before\n![[note-a]]\nAfter\n");
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Before"));
    assert!(stdout.contains("Embedded body line 1"));
    assert!(stdout.contains("Embedded body line 2"));
    assert!(stdout.contains("After"));
    assert!(!stdout.contains("status: active"));
    assert!(!stdout.contains("tags: [alpha]"));
    assert!(!stdout.contains("---"));
}

#[test]
fn test_note_render_inline_note_embed_splits_surrounding_text() {
    let vault = TestVault::new();
    vault.create_note("note-a", "Embedded line 1\nEmbedded line 2\n");
    vault.create_note("host", "Before![[note-a]]After");
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let before_idx = stdout.find("Before").unwrap();
    let embed_idx = stdout.find("Embedded line 1").unwrap();
    let after_idx = stdout.find("After").unwrap();
    assert!(before_idx < embed_idx);
    assert!(embed_idx < after_idx);
    assert!(stdout.contains("Before\nEmbedded line 1"));
    assert!(stdout.contains("Embedded line 2\nAfter"));
}

#[test]
fn test_note_render_note_embed_alias_does_not_change_body_expansion() {
    let vault = TestVault::new();
    vault.create_note("note-a", "Shared embedded body\n");
    vault.create_note("host", "![[note-a]]\n![[note-a|Shown Name]]\n");
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.matches("Shared embedded body").count(), 2);
    assert!(!stdout.contains("Shown Name"));
}

#[test]
fn test_note_render_note_embed_with_selector_is_passthrough() {
    let vault = TestVault::new();
    vault.create_note("note-a", "Rendered body should stay hidden\n");
    vault.create_note("host", "![[note-a#Heading]]\n![[note-a#^blockid]]\n");
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("![[note-a#Heading]]"));
    assert!(stdout.contains("![[note-a#^blockid]]"));
    assert!(!stdout.contains("Rendered body should stay hidden"));
}

#[test]
fn test_note_render_missing_embedded_note_is_soft_failure() {
    let vault = TestVault::new();
    vault.create_note("host", "Before\n![[missing-note]]\nAfter\n");
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    assert!(stderr_contains(
        &output,
        "embedded note 'missing-note' not found in index, skipping."
    ));
    assert!(stdout_contains(
        &output,
        "<!-- [markbase] note 'missing-note' not found -->"
    ));
    assert!(stdout_contains(&output, "After"));
}

#[test]
fn test_note_render_embedded_note_read_failure_is_soft_failure() {
    let vault = TestVault::new();
    let embedded = vault.create_note("note-a", "Hidden body\n");
    vault.create_note("host", "Before\n![[note-a]]\nAfter\n");
    vault.index();
    std::fs::remove_file(embedded).unwrap();

    let output = vault.run_cli(&["note", "render", "host", "--dry-run"]);

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "failed to read 'note-a'"));
    assert!(stdout_contains(
        &output,
        "<!-- [markbase] failed to read 'note-a' -->"
    ));
    assert!(stdout_contains(&output, "After"));
}

#[test]
fn test_note_render_recursive_note_embeds_are_expanded() {
    let vault = TestVault::new();
    vault.create_note("note-c", "Line from C\n");
    vault.create_note("note-b", "Line from B\n![[note-c]]\nTail B\n");
    vault.create_note("note-a", "Line from A\n![[note-b]]\nTail A\n");
    vault.create_note("host", "Top\n![[note-a]]\nBottom\n");
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let a_idx = stdout.find("Line from A").unwrap();
    let b_idx = stdout.find("Line from B").unwrap();
    let c_idx = stdout.find("Line from C").unwrap();
    assert!(a_idx < b_idx);
    assert!(b_idx < c_idx);
    assert!(stdout.contains("Tail B"));
    assert!(stdout.contains("Tail A"));
    assert!(stdout.contains("Bottom"));
}

#[test]
fn test_note_render_recursive_note_embed_expands_nested_base_embed() {
    let vault = TestVault::new();
    vault.create_note("customer", "---\ntype: company\n---\n![[opps.base]]\n");
    vault.create_note("host", "![[customer]]\n");
    vault.create_note(
        "deal-for-customer",
        "---\ntype: opportunity\nrelated_customer: \"[[customer]]\"\n---\n",
    );
    vault.create_note(
        "deal-for-host",
        "---\ntype: opportunity\nrelated_customer: \"[[host]]\"\n---\n",
    );
    vault.create_file(
        "opps.base",
        "views:\n  - type: table\n    name: Opportunities\n    filters:\n      and:\n        - related_customer == link(this)\n    order:\n      - file.name\n",
    );
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rendered from opps.base"));
    assert!(stdout.contains("[[deal-for-customer]]"));
    assert!(!stdout.contains("[[deal-for-host]]"));
}

#[test]
fn test_note_render_recursive_note_embed_cycle_is_soft_failure() {
    let vault = TestVault::new();
    vault.create_note("note-a", "A-start\n![[note-b]]\nA-end\n");
    vault.create_note("note-b", "B-start\n![[note-a]]\nB-end\n");
    vault.create_note("host", "Top\n![[note-a]]\nBottom\n");
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    assert!(stderr_contains(
        &output,
        "WARN: recursive note embed skipped for 'note-a' to avoid cycle."
    ));
    assert!(stdout_contains(
        &output,
        "<!-- [markbase] recursive note embed skipped for 'note-a' -->"
    ));
    assert!(stdout_contains(&output, "B-end"));
    assert!(stdout_contains(&output, "A-end"));
    assert!(stdout_contains(&output, "Bottom"));
}

#[test]
fn test_note_render_dry_run() {
    let vault = TestVault::new();
    vault.create_note("acme", "---\ntype: company\n---\n![[opps.base]]\n");
    vault.create_note(
        "deal1",
        "---\ntype: opportunity\nrelated_customer: \"[[acme]]\"\n---\n",
    );
    vault.create_file(
        "opps.base",
        "views:\n  - type: table\n    name: Opportunities\n    filters:\n      and:\n        - related_customer == link(this)\n    order:\n      - file.name\n",
    );
    vault.index();

    let output = vault.run_cli(&["note", "render", "acme", "--dry-run"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("dry-run from opps.base"));
    assert!(stdout.contains("FROM notes"));
    assert!(stdout.contains("[[acme]]"));
    assert!(stderr.is_empty(), "unexpected stderr output: {}", stderr);
}

#[test]
fn test_note_render_base_embed_missing_view_selector() {
    let vault = TestVault::new();
    vault.create_note("host", "![[tasks.base#Missing View]]");
    vault.create_file(
        "tasks.base",
        "views:\n  - type: table\n    name: Open Tasks\n    order:\n      - file.name\n",
    );
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    assert!(stderr_contains(
        &output,
        "view 'Missing View' not found in 'tasks.base', skipping."
    ));
    assert!(stdout_contains(
        &output,
        "<!-- [markbase] view 'Missing View' not found in 'tasks.base' -->"
    ));
}

#[test]
fn test_note_render_table_format() {
    let vault = TestVault::new();
    vault.create_note("test-note", "---\nname: test\n---\n![[test.base]]");
    vault.create_note("linked-note", "---\nname: linked\n---\n");
    vault.create_file(
        "test.base",
        "views:\n  - type: table\n    name: Test View\n    order:\n      - file.name\n",
    );
    vault.index();

    let output = vault.run_cli(&["note", "render", "test-note", "-o", "table"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("| name |"));
    assert!(stderr.is_empty(), "unexpected stderr output: {}", stderr);
}

#[test]
fn test_note_render_empty_results() {
    let vault = TestVault::new();
    vault.create_note("test-note", "![[empty.base]]");
    vault.create_file(
        "empty.base",
        "views:\n  - type: table\n    name: Empty\n    filters:\n      and:\n        - file.name == \"nonexistent\"\n    order:\n      - file.name\n",
    );
    vault.index();

    let output = vault.run_cli(&["note", "render", "test-note"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("```json"));
    assert!(stdout.contains("[]"));
    assert!(stderr.is_empty(), "unexpected stderr output: {}", stderr);
}

#[test]
fn test_note_render_json_field() {
    let vault = TestVault::new();
    vault.create_note("test-note", "---\ntags: [tag1, tag2]\n---\n![[tags.base]]");
    vault.create_file(
        "tags.base",
        "views:\n  - type: table\n    name: Tags View\n    order:\n      - file.tags\n",
    );
    vault.index();

    let output = vault.run_cli(&["note", "render", "test-note"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("```json"));
    assert!(stdout.contains("\"tags\""));
    assert!(stdout.contains("\"tag1\""));
    assert!(stdout.contains("\"tag2\""));
}

#[test]
fn test_note_render_sort() {
    let vault = TestVault::new();
    vault.create_note("test-note", "![[sorted.base]]");
    vault.create_note("a-note", "---\npriority: 1\n---\n");
    vault.create_note("b-note", "---\npriority: 2\n---\n");
    vault.create_file(
        "sorted.base",
        "views:\n  - type: table\n    name: Sorted\n    order:\n      - file.name\n    sort:\n      - property: file.name\n        direction: DESC\n",
    );
    vault.index();

    let output = vault.run_cli(&["note", "render", "test-note", "--dry-run"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ORDER BY"));
    assert!(stdout.contains("DESC"));
}

#[test]
fn test_note_render_non_base_embed_passthrough_after_parser_change() {
    let vault = TestVault::new();
    vault.create_file("image.png", "binary");
    vault.create_note("test-note", "Before\n![[image.png]]\nAfter");
    vault.index();

    let output = vault.run_cli(&["note", "render", "test-note"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Before"));
    assert!(stdout.contains("![[image.png]]"));
    assert!(stdout.contains("After"));
}

#[test]
fn test_note_render_base_embed_inside_fenced_code_is_not_expanded() {
    let vault = TestVault::new();
    vault.create_note("task-open", "---\nstatus: open\n---\n");
    vault.create_file(
        "tasks.base",
        "views:\n  - type: table\n    name: Open Tasks\n    filters:\n      and:\n        - status == \"open\"\n    order:\n      - file.name\n",
    );
    vault.create_note("host", "```md\n![[tasks.base#Open Tasks]]\n```\n");
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("```md"));
    assert!(stdout.contains("![[tasks.base#Open Tasks]]"));
    assert!(!stdout.contains("rendered from tasks.base"));
    assert!(!stdout.contains("[[task-open]]"));
}

#[test]
fn test_note_render_note_embed_inside_code_context_is_not_expanded() {
    let vault = TestVault::new();
    vault.create_note("note-a", "Expanded note body\n");
    vault.create_note(
        "host",
        "Outside\n![[note-a]]\n\nInline `![[note-a]]`\n\n```md\n![[note-a]]\n```\n",
    );
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Outside\nExpanded note body"));
    assert!(stdout.contains("Inline `![[note-a]]`"));
    assert!(stdout.contains("```md\n![[note-a]]\n```"));
    assert_eq!(stdout.matches("Expanded note body").count(), 1);
}

#[test]
fn test_render_note_embed_behavior_matches_readme_examples() {
    let vault = TestVault::new();
    vault.create_note("snippet", "Snippet body\n");
    vault.create_note("note-a", "A start\n![[note-b]]\nA end\n");
    vault.create_note("note-b", "B start\n![[note-a]]\nB end\n");
    vault.create_note("host", "Before![[snippet]]After\n\nLoop:\n![[note-a]]\n");
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    assert!(stdout_contains(&output, "Before\nSnippet body\nAfter"));
    assert!(stderr_contains(
        &output,
        "WARN: recursive note embed skipped for 'note-a' to avoid cycle."
    ));
    assert!(stdout_contains(
        &output,
        "<!-- [markbase] recursive note embed skipped for 'note-a' -->"
    ));
}

#[test]
fn test_render_note_embed_behavior_matches_design_contract() {
    let vault = TestVault::new();
    vault.create_note("note-c", "Inner body\n");
    vault.create_note("note-b", "Middle body\n![[note-c]]\n");
    vault.create_note(
        "note-a",
        "---\nstatus: active\n---\nOuter body\n![[note-b]]\n",
    );
    vault.create_note("host", "Before![[note-a]]After\n");
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Before\nOuter body"));
    assert!(stdout.contains("Middle body"));
    assert!(stdout.contains("Inner body\nAfter"));
    assert!(!stdout.contains("status: active"));
}

#[test]
fn test_note_verify_required_field_with_definition() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("activity_log.md"),
        r#"---
type: activity
_schema:
  required:
    - description
  properties:
    description:
      type: text
      description: "一句话总结活动内容"
---

# Activity Log
"#,
    )
    .unwrap();

    vault.create_note(
        "meeting-2026-01-01",
        r#"---
templates: ["[[activity_log]]"]
type: activity
---

# Meeting
"#,
    );
    vault.index();

    let output = vault.note_verify("meeting-2026-01-01");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "required field 'description'"));
    assert!(stderr_contains(&output, "→ Definition:"));
    assert!(stderr_contains(&output, "type=text"));
    assert!(stderr_contains(
        &output,
        "description=\"一句话总结活动内容\""
    ));
}

#[test]
fn test_note_verify_type_mismatch_with_definition() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company_customer.md"),
        r#"---
type: company
_schema:
  properties:
    employee_count:
      type: number
      description: "员工数量"
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "acme",
        r#"---
templates: ["[[company_customer]]"]
type: company
employee_count: "not-a-number"
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "type mismatch"));
    assert!(stderr_contains(&output, "→ Definition:"));
    assert!(stderr_contains(&output, "type=number"));
    assert!(stderr_contains(&output, "description=\"员工数量\""));
}

#[test]
fn test_note_verify_enum_failure_with_definition() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company_customer.md"),
        r#"---
type: company
_schema:
  properties:
    size:
      type: text
      enum: [startup, smb, enterprise]
      description: "公司规模"
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "acme",
        r#"---
templates: ["[[company_customer]]"]
type: company
size: giant
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "invalid value"));
    assert!(stderr_contains(&output, "→ Definition:"));
    assert!(stderr_contains(&output, "enum=[startup, smb, enterprise]"));
    assert!(stderr_contains(&output, "description=\"公司规模\""));
}

#[test]
fn test_note_verify_link_field_with_definition() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company_customer.md"),
        r#"---
type: company
_schema:
  properties:
    related_person:
      type: text
      format: link
      target: person
      description: "主要联系人"
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "acme",
        r#"---
templates: ["[[company_customer]]"]
type: company
related_person: "[[nonexistent-person]]"
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "not found in the vault"));
    assert!(stderr_contains(&output, "→ Definition:"));
    assert!(stderr_contains(&output, "format=link"));
    assert!(stderr_contains(&output, "target=person"));
}

#[test]
fn test_note_resolve_exact_match() {
    let vault = TestVault::new();
    vault.create_note(
        "acme",
        r#"---
type: company
aliases: ["ACME Corp"]
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_resolve(&["acme"]);

    assert_cli_success(&output);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json[0]["query"], "acme");
    assert_eq!(json[0]["status"], "exact");
    assert_eq!(json[0]["matches"][0]["name"], "acme");
    assert_eq!(json[0]["matches"][0]["type"], "company");
    assert_eq!(json[0]["matches"][0]["matched_by"], "name");
}

#[test]
fn test_note_resolve_alias_match() {
    let vault = TestVault::new();
    vault.create_note(
        "acme",
        r#"---
type: company
aliases: ["阿里"]
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_resolve(&["阿里"]);

    assert_cli_success(&output);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json[0]["status"], "alias");
    assert_eq!(json[0]["matches"][0]["name"], "acme");
    assert_eq!(json[0]["matches"][0]["matched_by"], "alias");
}

#[test]
fn test_note_resolve_multiple_matches() {
    let vault = TestVault::new();
    vault.create_note(
        "zhangwei-person",
        r#"---
type: person
aliases: ["张伟"]
---

# Zhang Wei
"#,
    );
    vault.create_note(
        "zhangwei-shanghai",
        r#"---
type: person
aliases: ["张伟"]
---

# Zhang Wei Shanghai
"#,
    );
    vault.index();

    let output = vault.note_resolve(&["张伟"]);

    assert_cli_success(&output);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json[0]["status"], "multiple");
    assert_eq!(json[0]["matches"].as_array().unwrap().len(), 2);
}

#[test]
fn test_note_resolve_missing() {
    let vault = TestVault::new();
    vault.index();

    let output = vault.note_resolve(&["ghost"]);

    assert_cli_success(&output);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json[0]["status"], "missing");
    assert_eq!(json[0]["matches"].as_array().unwrap().len(), 0);
}

#[test]
fn test_note_resolve_rejects_path_like_input() {
    let vault = TestVault::new();

    let output = vault.note_resolve(&["logs/acme"]);

    assert_cli_error(&output);
    assert!(stderr_contains(
        &output,
        "Invalid resolve input 'logs/acme'"
    ));
}

#[test]
fn test_note_resolve_rejects_extension_in_input() {
    let vault = TestVault::new();

    let output = vault.note_resolve(&["acme.md"]);

    assert_cli_error(&output);
    assert!(stderr_contains(
        &output,
        "resolve input must not include a file extension"
    ));
}

#[test]
fn test_note_resolve_rejects_path_like_input_in_multi_query() {
    let vault = TestVault::new();
    vault.create_note(
        "acme",
        r#"---
type: company
aliases: ["ACME Corp"]
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_resolve(&["acme", "logs/ghost"]);

    assert_cli_error(&output);
    assert!(stderr_contains(
        &output,
        "Invalid resolve input 'logs/ghost'"
    ));
}

#[test]
fn test_note_resolve_multiple_queries() {
    let vault = TestVault::new();
    vault.create_note(
        "acme",
        r#"---
type: company
aliases: ["阿里"]
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_resolve(&["acme", "ghost"]);

    assert_cli_success(&output);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 2);
    assert_eq!(json[0]["status"], "exact");
    assert_eq!(json[1]["status"], "missing");
}

#[test]
fn test_note_verify_no_templates_reports_description_warning_first() {
    let vault = TestVault::new();
    vault.create_note("test-note", "# Test Note");
    vault.index();

    let output = vault.note_verify("test-note");

    assert_cli_error(&output);
    assert!(stderr_contains(
        &output,
        "Verifying note 'test-note' (file.path: test-note.md) against template(s):"
    ));
    assert!(stderr_contains(
        &output,
        "missing global field 'description'"
    ));
    assert!(stderr_contains(&output, "→ Definition:"));
    assert!(stderr_contains(&output, "scope=global"));
    assert!(stderr_contains(&output, "required=true"));
    assert!(stderr_contains(&output, "type=text"));
    assert!(stderr_contains(&output, "nonempty=true"));
    assert!(stderr_contains(
        &output,
        "description=\"一句话说明这个 note 是什么\""
    ));
    assert!(stderr_contains(&output, "no 'templates'"));
}

#[test]
fn test_note_verify_empty_description_warns() {
    let vault = TestVault::new();
    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company_customer.md"),
        r#"---
type: company
_schema:
  required: [description]
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "acme",
        r#"---
templates: ["[[company_customer]]"]
type: company
description: ""
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(&output, "empty global field 'description'"));
}

#[test]
fn test_note_verify_non_string_description_warns() {
    let vault = TestVault::new();
    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("company_customer.md"),
        r#"---
type: company
---

# Template
"#,
    )
    .unwrap();

    vault.create_note(
        "acme",
        r#"---
templates: ["[[company_customer]]"]
type: company
description:
  nested: true
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_verify("acme");

    assert_cli_success(&output);
    assert!(stderr_contains(
        &output,
        "invalid global field 'description'. Expected non-empty text, got 'unknown'"
    ));
    assert!(stderr_contains(&output, "→ Definition:"));
    assert!(stderr_contains(&output, "scope=global"));
    assert!(stderr_contains(
        &output,
        "description=\"一句话说明这个 note 是什么\""
    ));
}

#[test]
fn test_note_create_simple_adds_default_description() {
    let vault = TestVault::new();

    let output = vault.note_new("my-note");

    assert_cli_success(&output);
    let content = std::fs::read_to_string(vault.path.join("inbox").join("my-note.md")).unwrap();
    assert!(content.contains("description: 临时笔记"));
}

#[test]
fn test_note_create_with_template_adds_description_when_template_omits_it() {
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
    let content = std::fs::read_to_string(vault.path.join("inbox").join("today.md")).unwrap();
    assert!(content.contains("description:"));
    assert!(!content.contains("_schema"));
}

#[test]
fn test_note_resolve_includes_description() {
    let vault = TestVault::new();
    vault.create_note(
        "acme",
        r#"---
type: company
description: Smart home customer
aliases: ["ACME Corp"]
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_resolve(&["acme"]);

    assert_cli_success(&output);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json[0]["matches"][0]["description"], "Smart home customer");
}

#[test]
fn test_note_resolve_missing_description_returns_null() {
    let vault = TestVault::new();
    vault.create_note(
        "acme",
        r#"---
type: company
aliases: ["ACME Corp"]
---

# ACME
"#,
    );
    vault.index();

    let output = vault.note_resolve(&["acme"]);

    assert_cli_success(&output);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json[0]["matches"][0]["description"].is_null());
}
