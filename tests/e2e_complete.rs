mod common;

use common::{TestVault, assert_cli_success, stdout_contains};

#[test]
fn test_full_workflow() {
    let vault = TestVault::new();

    vault.create_note(
        "readme",
        r#"---
title: README
---

# README

This is the main readme file.

#documentation #important
"#,
    );

    vault.create_note(
        "architecture",
        r#"---
title: Architecture
---

# Architecture

#technical #design

See [[readme]] for overview.
See [[api-design]] for API details.
"#,
    );

    vault.create_note(
        "api-design",
        "# API Design\n\n#api #technical\n\nAPI documentation here.",
    );

    let index_output = vault.index();
    assert_cli_success(&index_output);

    let query_all = vault.query("");
    assert!(stdout_contains(&query_all, "readme"));
    assert!(stdout_contains(&query_all, "architecture"));
    assert!(stdout_contains(&query_all, "api-design"));

    let query_documentation = vault.query("list_contains(file.tags, 'documentation')");
    assert!(stdout_contains(&query_documentation, "readme"));

    let query_technical = vault.query("list_contains(file.tags, 'technical')");
    assert!(stdout_contains(&query_technical, "architecture"));
    assert!(stdout_contains(&query_technical, "api-design"));
}

#[test]
fn test_backlink_cycle() {
    let vault = TestVault::new();

    vault.create_note("a", "See [[b]] and [[c]].");
    vault.create_note("b", "Links to [[a]].");
    vault.create_note("c", "Also links to [[a]] and [[b]].");

    vault.index();

    let output = vault.query("file.name == 'a'");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("a"));

    let output = vault.query("file.name == 'b'");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("b"));

    let output = vault.query("file.name == 'c'");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("c"));
}

#[test]
fn test_gitignore_full_cycle() {
    let vault = TestVault::new();

    vault.create_note("public1", "# Public 1");
    vault.create_note("public2", "# Public 2");
    vault.create_note("secret", "# Secret");

    vault.create_gitignore("secret.md\n");

    vault.index();

    let output = vault.query("");
    let _stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout_contains(&output, "public1"));
    assert!(stdout_contains(&output, "public2"));
    assert!(!_stdout.contains("secret"));

    vault.create_gitignore("public2.md\n!public2.md\n");

    vault.index_force();

    let output = vault.query("");
    assert!(stdout_contains(&output, "public1"));
    assert!(stdout_contains(&output, "public2"));
}

#[test]
fn test_template_lifecycle() {
    let vault = TestVault::new();

    let templates_dir = vault.path.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();

    std::fs::write(
        templates_dir.join("daily.md"),
        r#"---
_schema:
  instance:
    template: daily
    type: journal
---

# {{name}}

Date: {{date}}
Time: {{time}}

## Notes


"#,
    )
    .unwrap();

    std::fs::write(
        templates_dir.join("project.md"),
        r#"---
_schema:
  instance:
    template: project
    type: project
    status: planning
---

# {{name}}

## Overview


## Tasks

- [ ]

## Notes


"#,
    )
    .unwrap();

    vault.index();

    let list_output = vault.template_list();
    assert_cli_success(&list_output);
    // Verify template list actually shows the templates
    assert!(stdout_contains(&list_output, "daily"));
    assert!(stdout_contains(&list_output, "project"));

    let describe_output = vault.template_describe("daily");
    assert_cli_success(&describe_output);
    let stdout = String::from_utf8_lossy(&describe_output.stdout);
    assert!(stdout.contains("{{name}}") || stdout.contains("{{date}}"));

    let new_output = vault.note_new_with_template("2024-01-15", "daily");
    assert_cli_success(&new_output);
    assert!(vault.path.join("inbox").join("2024-01-15.md").exists());
}

#[test]
fn test_template_list_shows_templates_in_folder() {
    let vault = TestVault::new();

    // Create templates in the templates folder
    vault.create_note_in_subdir(
        "templates",
        "meeting",
        "# Meeting Template\n\n## Attendees\n\n## Agenda\n\n## Notes\n",
    );
    vault.create_note_in_subdir(
        "templates",
        "bug-report",
        "# Bug Report\n\n## Description\n\n## Steps to Reproduce\n",
    );

    // Create a regular note outside templates folder (should not appear in list)
    vault.create_note(
        "regular-note",
        "# Regular Note\n\nThis should not appear in template list.",
    );

    vault.index();

    // Test template list command
    let list_output = vault.template_list();
    assert_cli_success(&list_output);

    // Verify templates are displayed
    assert!(stdout_contains(&list_output, "meeting"));
    assert!(stdout_contains(&list_output, "bug-report"));

    // Verify regular note is NOT in the template list
    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(!stdout.contains("regular-note"));
}

#[test]
fn test_complex_note_structure() {
    let vault = TestVault::new();

    vault.create_note(
        "complex",
        r#"---
title: Complex Note
author:
  name: John Doe
  email: john@example.com
---

# Complex Note

This note has *rich* **formatting** and more.

#work #important #reviewed

## Section 1

See [[another-note]] for details.

### Subsection

More content here.

## Section 2

- Item 1
- Item 2
- Item 3

> A blockquote here.

```rust
fn main() {
    println!("Hello");
}
```

[[another-note]] also appears here.

Check ![[diagram.png]] for visual.
"#,
    );

    vault.create_note("another-note", "# Another Note\n\nLinked from complex.");
    vault.create_note("related", "# Related\n\nRelated content.");

    vault.index();

    let output = vault.query("file.name == 'complex'");
    assert_cli_success(&output);

    let output = vault.query("list_contains(file.tags, 'important')");
    assert!(stdout_contains(&output, "complex"));

    let output = vault.query("list_contains(file.links, 'another-note')");
    assert!(stdout_contains(&output, "complex"));
}

#[test]
fn test_mixed_file_types() {
    let vault = TestVault::new();

    vault.create_note("readme", "# README");
    vault.create_file("config.json", r#"{"key": "value"}"#);
    vault.create_file("data.yaml", "key: value");
    vault.create_file("todo.txt", "Buy milk");
    vault.create_file("image.svg", "<svg></svg>");
    vault.create_file("script.sh", "#!/bin/bash\necho hello");

    vault.index();

    let output = vault.query("");
    assert_cli_success(&output);
}

#[test]
fn test_unicode_content() {
    let vault = TestVault::new();

    vault.create_note(
        "unicode-notes",
        r#"---
title: Unicode 测试
tags: [测试, 中文]
---

# Unicode 测试

Unicode characters: é à ü ñ 中文 日本語 🇺🇸

Emoji: 🎉 👋 🚀

Special: <>&"'
"#,
    );

    vault.index();

    let output = vault.query("title == 'Unicode 测试'");
    assert_cli_success(&output);
}

#[test]
fn test_nested_directory_structure() {
    let vault = TestVault::new();

    vault.create_note("root", "# Root");
    vault.create_note_in_subdir("level1", "note1", "# Level 1");
    vault.create_note_in_subdir("level1/level2", "note2", "# Level 2");
    vault.create_note_in_subdir("level1/level2/level3", "note3", "# Level 3");
    vault.create_note_in_subdir("docs/api", "endpoint", "# API Endpoint");
    vault.create_note_in_subdir("docs/guides", "guide", "# Guide");

    vault.index();

    let output = vault.query("");
    assert_cli_success(&output);

    let query_docs = vault.query("file.name == 'endpoint' OR file.name == 'guide'");
    assert!(stdout_contains(&query_docs, "endpoint") || stdout_contains(&query_docs, "guide"));
}

#[test]
fn test_query_chaining_operations() {
    let vault = TestVault::new();

    vault.create_note("note1", "# Note 1\n#tag-a #tag-b");
    vault.create_note("note2", "# Note 2\n#tag-b #tag-c");
    vault.create_note("note3", "# Note 3\n#tag-c #tag-d");

    vault.index();

    let output = vault.query("list_contains(file.tags, 'tag-a')");
    assert!(stdout_contains(&output, "note1"));

    let output = vault.query("list_contains(file.tags, 'tag-b')");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("note1") || stdout.contains("note2"));

    let output = vault.query("ORDER BY file.name DESC LIMIT 2");
    assert_cli_success(&output);
}

#[test]
fn test_error_recovery() {
    let vault = TestVault::new();

    vault.create_note("valid", "# Valid Note");

    vault.index();

    let output = vault.query("file.name == 'nonexistent'");
    assert_cli_success(&output);

    vault.create_note("valid2", "# Valid 2");
    vault.index();

    let output = vault.query("");
    assert!(stdout_contains(&output, "valid"));
    assert!(stdout_contains(&output, "valid2"));
}

#[test]
fn test_large_frontmatter() {
    let vault = TestVault::new();

    let large_frontmatter = r#"---
title: Large Note
author: Test Author
date: 2024-01-01
tags: [tag1, tag2, tag3, tag4, tag5]
metadata:
  version: 1.0
  status: active
  priority: high
  category: test
  subcategory: integration
  labels:
    - label1
    - label2
  config:
    setting1: value1
    setting2: value2
    setting3: value3
  links:
    - link1
    - link2
  embeds:
    - embed1
    - embed2
---

# Large Note

Content here.
"#;

    vault.create_note("large", large_frontmatter);

    vault.index();

    let output = vault.query("title == 'Large Note'");
    assert_cli_success(&output);
    assert!(stdout_contains(&output, "large"));
}

#[test]
fn test_link_semantics_are_consistent_across_index_rename_and_render() {
    let vault = TestVault::new();
    vault.create_note(
        "source",
        r#"---
related: "[[folder/old-note.md#Heading|Alias]]"
---

| view |
| --- |
| [[folder/old-note.md\|Alias]] |

![[tasks.base#Open Tasks]]
"#,
    );
    vault.create_note("old-note", "# Old Note");
    vault.create_note("task-open", "---\nstatus: open\n---\n");
    vault.create_note("task-closed", "---\nstatus: closed\n---\n");
    vault.create_file(
        "tasks.base",
        "views:\n  - type: table\n    name: Open Tasks\n    filters:\n      and:\n        - status == \"open\"\n    order:\n      - file.name\n  - type: table\n    name: Closed Tasks\n    filters:\n      and:\n        - status == \"closed\"\n    order:\n      - file.name\n",
    );

    vault.index();

    let before = vault.query(
        "file.name == 'source' AND list_contains(file.links, 'old-note') AND list_contains(file.embeds, 'tasks.base')",
    );
    assert_cli_success(&before);
    assert!(stdout_contains(&before, "source"));

    let rename = vault.note_rename("old-note", "new-note");
    assert_cli_success(&rename);

    let content = std::fs::read_to_string(vault.path.join("source.md")).unwrap();
    assert!(content.contains("[[new-note#Heading|Alias]]"));
    assert!(content.contains("[[new-note\\|Alias]]"));

    let after = vault.query(
        "file.name == 'source' AND list_contains(file.links, 'new-note') AND list_contains(file.embeds, 'tasks.base')",
    );
    assert_cli_success(&after);
    assert!(stdout_contains(&after, "source"));

    let render = vault.run_cli(&["note", "render", "source"]);
    assert_cli_success(&render);
    assert!(stdout_contains(&render, "Open Tasks"));
    assert!(stdout_contains(&render, "[[task-open]]"));
    assert!(!stdout_contains(&render, "Closed Tasks"));
}

#[test]
fn test_code_context_links_are_ignored_across_features() {
    let vault = TestVault::new();
    vault.create_note(
        "source",
        "See [[target]].\n\n`[[target]]`\n\n```md\n![[target]]\n[[target]]\n![[tasks.base#Open Tasks]]\n```\n",
    );
    vault.create_note("target", "# Target");
    vault.create_note("task-open", "---\nstatus: open\n---\n");
    vault.create_file(
        "tasks.base",
        "views:\n  - type: table\n    name: Open Tasks\n    filters:\n      and:\n        - status == \"open\"\n    order:\n      - file.name\n",
    );

    vault.index();

    let indexed = vault.query("file.name == 'source' AND list_contains(file.links, 'target')");
    assert_cli_success(&indexed);
    assert!(stdout_contains(&indexed, "source"));

    let rename = vault.note_rename("target", "renamed-target");
    assert_cli_success(&rename);

    let content = std::fs::read_to_string(vault.path.join("source.md")).unwrap();
    assert!(content.contains("See [[renamed-target]]."));
    assert!(content.contains("`[[target]]`"));
    assert!(content.contains("```md\n![[target]]\n[[target]]\n![[tasks.base#Open Tasks]]\n```"));

    let render = vault.run_cli(&["note", "render", "source"]);
    assert_cli_success(&render);
    assert!(stdout_contains(&render, "```md"));
    assert!(stdout_contains(&render, "![[tasks.base#Open Tasks]]"));
    assert!(!stdout_contains(&render, "rendered from tasks.base"));
    assert!(!stdout_contains(&render, "[[task-open]]"));
}

#[test]
fn test_render_view_selector_matches_documented_behavior() {
    let vault = TestVault::new();
    vault.create_note("host", "![[tasks.base#Missing View]]");
    vault.create_file(
        "tasks.base",
        "views:\n  - type: table\n    name: Open Tasks\n    order:\n      - file.name\n",
    );
    vault.index();

    let output = vault.run_cli(&["note", "render", "host"]);

    assert_cli_success(&output);
    assert!(stdout_contains(
        &output,
        "<!-- [markbase] view 'Missing View' not found in 'tasks.base' -->"
    ));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("view 'Missing View' not found in 'tasks.base', skipping."));
    assert!(!stdout_contains(&output, "Open Tasks"));
}
