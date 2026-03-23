mod common;

use common::{
    TestVault, assert_cli_error, assert_cli_success, docsify_shell_stub,
    docsify_shell_stub_with_homepage, http_get, pick_free_port, stderr_contains, stdout_contains,
};
use serde_json::Value as JsonValue;
use std::fs;

fn create_home_note(vault: &TestVault) {
    vault.create_note("HOME", "# Home\n");
}

fn create_base_homepage(vault: &TestVault, file_name: &str) {
    vault.create_file(file_name, "views:\n  - type: table\n    name: Demo\n");
}

fn create_multiline_callout_note(vault: &TestVault, name: &str) {
    vault.create_note(
        name,
        concat!(
            "# Callout Demo\n\n",
            "> [!agent-update]- Overwrite\n",
            "> 优先基于 `web-search` skill 联网检索补写并定期刷新。\n",
            "> 固定使用以下结构，且每个主项下必须继续使用子列表，避免把多个事实挤在同一行：\n",
            "> `- 官网`\n",
            "> `  - 官方首页：<官方首页 URL>`\n",
            "> `- 信息来源`\n",
            "> `  - <来源名称或页面标题>：<URL>`\n",
            "> \n",
            "> - 第一项\n",
            "> - 第二项\n",
        ),
    );
}

fn parse_cli_json(output: &std::process::Output) -> JsonValue {
    serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON")
}

#[test]
fn test_web_route_resolves_note_path_to_internal_note_name() {
    let vault = TestVault::new();
    vault.create_note_in_subdir("entities/person", "张三", "# Person\n");

    let output = vault.web_get("/entities/person/%E5%BC%A0%E4%B8%89.md");

    assert_cli_success(&output);
    assert!(stdout_contains(&output, "# Person"));
}

#[test]
fn test_web_route_matches_decoded_file_path() {
    let vault = TestVault::new();
    vault.create_note_in_subdir("entities/person", "张 三", "Decoded path match\n");

    let output = vault.web_get("/entities/person/%E5%BC%A0%20%E4%B8%89.md");

    assert_cli_success(&output);
    assert!(stdout_contains(&output, "Decoded path match"));
}

#[test]
fn test_web_request_refreshes_index_before_route_resolution() {
    let vault = TestVault::new();
    vault.create_file("index.html", &docsify_shell_stub());
    let port = pick_free_port();
    let _server = vault.spawn_web_server("127.0.0.1", port);

    vault.create_note_in_subdir("entities/person", "alice", "Freshly indexed\n");

    let response = http_get(port, "/entities/person/alice.md");

    assert_eq!(response.status_code, 200);
    assert_eq!(String::from_utf8_lossy(&response.body), "Freshly indexed");
}

#[test]
fn test_web_request_closes_request_scoped_db_handle() {
    let vault = TestVault::new();
    vault.create_note_in_subdir("entities/person", "alice", "One\n");
    vault.create_file("index.html", &docsify_shell_stub());
    let port = pick_free_port();
    let _server = vault.spawn_web_server("127.0.0.1", port);

    let first = http_get(port, "/entities/person/alice.md");
    assert_eq!(first.status_code, 200);

    let db_path = vault.db_path();
    if db_path.exists() {
        std::fs::remove_file(&db_path).unwrap();
    }
    let wal_path = db_path.with_extension("duckdb.wal");
    if wal_path.exists() {
        std::fs::remove_file(&wal_path).unwrap();
    }

    vault.create_note_in_subdir("entities/person", "bob", "Two\n");
    let second = http_get(port, "/entities/person/bob.md");

    assert_eq!(second.status_code, 200);
    assert_eq!(String::from_utf8_lossy(&second.body), "Two");
}

#[test]
fn test_web_http_route_returns_404_for_miss_and_400_for_bad_path() {
    let vault = TestVault::new();
    vault.create_file("index.html", &docsify_shell_stub());
    let port = pick_free_port();
    let _server = vault.spawn_web_server("127.0.0.1", port);

    let missing = http_get(port, "/missing.md");
    let bad = http_get(port, "/broken/%ZZ.md");

    assert_eq!(missing.status_code, 404);
    assert_eq!(bad.status_code, 400);
}

#[test]
fn test_web_get_returns_cli_failure_for_miss_and_bad_path() {
    let vault = TestVault::new();

    let missing = vault.web_get("/missing.md");
    let bad = vault.web_get("/broken/%ZZ.md");

    assert_cli_error(&missing);
    assert_cli_error(&bad);
}

#[test]
fn test_web_note_route_without_fields_returns_markdown_body() {
    let vault = TestVault::new();
    vault.create_note("host", "Plain markdown body\n");

    let output = vault.web_get("/host.md");

    assert_cli_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Plain markdown body"
    );
}

#[test]
fn test_web_note_fields_mode_only_supports_markdown_note_routes() {
    let vault = TestVault::new();
    vault.create_note("host", "Host\n");
    vault.create_file(
        "views/tasks.base",
        "views:\n  - type: table\n    name: Tasks\n",
    );
    vault.create_file("assets/image.png", "png-bytes");
    vault.create_file("index.html", &docsify_shell_stub());
    let port = pick_free_port();
    let _server = vault.spawn_web_server("127.0.0.1", port);

    let markdown = http_get(port, "/host.md?fields=properties");
    let base = http_get(port, "/views/tasks.base?fields=properties");
    let resource = http_get(port, "/assets/image.png?fields=properties");

    assert_eq!(markdown.status_code, 200);
    assert_eq!(
        markdown.headers.get("content-type").map(String::as_str),
        Some("application/json; charset=utf-8")
    );
    assert_eq!(base.status_code, 400);
    assert_eq!(resource.status_code, 400);
}

#[test]
fn test_web_note_fields_mode_validates_requested_fields_and_query_params() {
    let vault = TestVault::new();
    vault.create_note("host", "Host\n");
    vault.create_file("index.html", &docsify_shell_stub());
    let port = pick_free_port();
    let _server = vault.spawn_web_server("127.0.0.1", port);

    for path in [
        "/host.md?fields=properties",
        "/host.md?fields=links",
        "/host.md?fields=properties,links",
    ] {
        let response = http_get(port, path);
        assert_eq!(response.status_code, 200, "path: {}", path);
        assert_eq!(
            response.headers.get("content-type").map(String::as_str),
            Some("application/json; charset=utf-8"),
            "path: {}",
            path
        );
    }

    for path in [
        "/host.md?fields=unknown",
        "/host.md?foo=bar",
        "/host.md?fields=",
        "/host.md?fields=properties,",
        "/host.md?fields=properties,,links",
        "/host.md?fields=%20properties%20",
    ] {
        let response = http_get(port, path);
        assert_eq!(response.status_code, 400, "path: {}", path);
    }
}

#[test]
fn test_web_note_metadata_mode_returns_expected_response_envelope() {
    let vault = TestVault::new();
    vault.create_note_in_subdir("people", "alice", "---\ndescription: Test\n---\nBody\n");

    let properties = parse_cli_json(&vault.web_get("/people/alice.md?fields=properties"));
    let links = parse_cli_json(&vault.web_get("/people/alice.md?fields=links"));
    let both = parse_cli_json(&vault.web_get("/people/alice.md?fields=properties,links"));

    for value in [&properties, &links, &both] {
        assert_eq!(value["file"]["path"], "people/alice.md");
        assert_eq!(value["file"]["name"], "alice");
    }

    assert!(properties.get("properties").is_some());
    assert!(properties.get("links").is_none());
    assert!(links.get("links").is_some());
    assert!(links.get("properties").is_none());
    assert!(both.get("properties").is_some());
    assert!(both.get("links").is_some());
}

#[test]
fn test_web_note_metadata_properties_returns_ordered_semantic_fields() {
    let vault = TestVault::new();
    vault.create_note(
        "host",
        "---\nz_last: 1\nalpha:\n  - x\n  - 2\nmiddle:\n  reviewer: Bob\nbeta: null\n---\nBody\n",
    );

    let output = vault.web_get("/host.md?fields=properties");

    assert_cli_success(&output);
    let json = parse_cli_json(&output);
    let fields = json["properties"]["fields"].as_array().unwrap();
    let keys: Vec<_> = fields
        .iter()
        .map(|field| field["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, vec!["z_last", "alpha", "middle", "beta"]);
    assert_eq!(fields[0]["value"]["kind"], "scalar");
    assert_eq!(fields[1]["value"]["kind"], "list");
    assert_eq!(fields[2]["value"]["kind"], "object");
    assert_eq!(fields[3]["value"]["kind"], "null");
}

#[test]
fn test_web_note_metadata_properties_resolves_frontmatter_wikilinks() {
    let vault = TestVault::new();
    vault.create_note_in_subdir("people", "alice", "Alice\n");
    vault.create_note_in_subdir("projects", "apollo", "Apollo\n");
    vault.create_note(
        "host",
        "---\nmanager: \"Owner [[alice|Alice A.]] ok\"\nrelated:\n  - \"[[apollo]]\"\nmeta:\n  reviewer: \"[[missing-reviewer]]\"\n---\nBody\n",
    );

    let output = vault.web_get("/host.md?fields=properties");

    assert_cli_success(&output);
    let json = parse_cli_json(&output);
    let fields = json["properties"]["fields"].as_array().unwrap();

    let manager = fields
        .iter()
        .find(|field| field["key"] == "manager")
        .unwrap();
    let manager_segments = manager["value"]["segments"].as_array().unwrap();
    assert_eq!(manager["value"]["kind"], "rich_text");
    assert_eq!(manager_segments[0]["type"], "text");
    assert_eq!(manager_segments[1]["type"], "wikilink");
    assert_eq!(manager_segments[1]["target"], "alice");
    assert_eq!(manager_segments[1]["text"], "Alice A.");
    assert_eq!(manager_segments[1]["href"], "/people/alice.md");
    assert_eq!(manager_segments[1]["exists"], true);

    let related = fields
        .iter()
        .find(|field| field["key"] == "related")
        .unwrap();
    assert_eq!(related["value"]["kind"], "list");
    assert_eq!(
        related["value"]["items"][0]["segments"][0]["href"],
        "/projects/apollo.md"
    );

    let meta = fields.iter().find(|field| field["key"] == "meta").unwrap();
    let nested = meta["value"]["fields"].as_array().unwrap();
    assert_eq!(nested[0]["key"], "reviewer");
    assert_eq!(
        nested[0]["value"]["segments"][0]["target"],
        "missing-reviewer"
    );
    assert_eq!(nested[0]["value"]["segments"][0]["exists"], false);
    assert!(nested[0]["value"]["segments"][0].get("href").is_none());
}

#[test]
fn test_web_note_metadata_properties_includes_template_schema_enrichment() {
    let vault = TestVault::new();
    vault.create_file(
        "templates/template_a.md",
        r#"---
_schema:
  properties:
    owner:
      type: text
      format: link
      target: person
      description: Owner from template A
  required:
    - owner
---
"#,
    );
    vault.create_file(
        "templates/template_b.md",
        r#"---
_schema:
  properties:
    owner:
      type: number
      description: Owner from template B
---
"#,
    );
    vault.create_note(
        "host",
        "---\ntemplates:\n  - \"[[template_a]]\"\n  - \"[[template_b]]\"\nowner: \"[[alice]]\"\n---\nBody\n",
    );
    vault.create_note("alice", "Alice\n");

    let output = vault.web_get("/host.md?fields=properties");

    assert_cli_success(&output);
    let json = parse_cli_json(&output);
    let owner = json["properties"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|field| field["key"] == "owner")
        .unwrap();

    assert_eq!(owner["schema"]["template"], "template_a");
    assert_eq!(owner["schema"]["required"], true);
    assert_eq!(owner["schema"]["type"], "text");
    assert_eq!(owner["schema"]["format"], "link");
    assert_eq!(owner["schema"]["target"], "person");
    assert_eq!(owner["schema"]["description"], "Owner from template A");
}

#[test]
fn test_web_note_metadata_properties_does_not_invent_schema_type_for_ordinary_field() {
    let vault = TestVault::new();
    vault.create_file(
        "templates/template_a.md",
        r#"---
_schema:
  properties:
    owner:
      description: Owner without explicit type
---
"#,
    );
    vault.create_note(
        "host",
        "---\ntemplates:\n  - \"[[template_a]]\"\nowner: \"[[alice]]\"\n---\nBody\n",
    );
    vault.create_note("alice", "Alice\n");

    let output = vault.web_get("/host.md?fields=properties");

    assert_cli_success(&output);
    let json = parse_cli_json(&output);
    let owner = json["properties"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|field| field["key"] == "owner")
        .unwrap();

    assert_eq!(owner["schema"]["template"], "template_a");
    assert_eq!(
        owner["schema"]["description"],
        "Owner without explicit type"
    );
    assert!(owner["schema"].get("type").is_none());
}

#[test]
fn test_web_note_metadata_links_returns_resolved_and_unresolved_targets() {
    let vault = TestVault::new();
    vault.create_note_in_subdir("people", "alice", "Alice\n");
    vault.create_file(
        "views/tasks.base",
        "views:\n  - type: table\n    name: Tasks\n",
    );
    vault.create_file("assets/image.png", "png-bytes");
    vault.create_note(
        "host",
        "[[alice]]\n[[tasks.base]]\n[[missing-note]]\n![[image.png]]\n",
    );

    let output = vault.web_get("/host.md?fields=links");

    assert_cli_success(&output);
    let json = parse_cli_json(&output);
    let links = json["links"].as_array().unwrap();

    let alice = links
        .iter()
        .find(|entry| entry["target"] == "alice")
        .unwrap();
    assert_eq!(alice["kind"], "note");
    assert_eq!(alice["exists"], true);
    assert_eq!(alice["href"], "/people/alice.md");

    let base = links
        .iter()
        .find(|entry| entry["target"] == "tasks.base")
        .unwrap();
    assert_eq!(base["kind"], "base");
    assert_eq!(base["exists"], true);
    assert_eq!(base["href"], "/views/tasks.base");

    let image = links
        .iter()
        .find(|entry| entry["target"] == "image.png")
        .unwrap();
    assert_eq!(image["kind"], "resource");
    assert_eq!(image["exists"], true);
    assert_eq!(image["href"], "/assets/image.png");

    let missing = links
        .iter()
        .find(|entry| entry["target"] == "missing-note")
        .unwrap();
    assert_eq!(missing["kind"], "note");
    assert_eq!(missing["exists"], false);
    assert!(missing.get("href").is_none());
}

#[test]
fn test_web_get_matches_web_serve_for_note_metadata_mode() {
    let vault = TestVault::new();
    vault.create_note(
        "host",
        "---\ndescription: Test\nfriend: \"[[alice]]\"\n---\nBody\n",
    );
    vault.create_note("alice", "Alice\n");
    vault.create_file("index.html", &docsify_shell_stub());
    let port = pick_free_port();
    let _server = vault.spawn_web_server("127.0.0.1", port);

    let cli = vault.web_get("/host.md?fields=properties,links");
    let http = http_get(port, "/host.md?fields=properties,links");

    assert_cli_success(&cli);
    assert_eq!(http.status_code, 200);
    assert_eq!(
        serde_json::from_slice::<JsonValue>(&cli.stdout).unwrap(),
        serde_json::from_slice::<JsonValue>(&http.body).unwrap()
    );
}

#[test]
fn test_web_render_mode_returns_plain_markdown_body() {
    let vault = TestVault::new();
    vault.create_note("note-a", "Embedded body\n");
    vault.create_note("host", "Before\n![[note-a]]\nAfter\n");

    let output = vault.web_get("/host.md");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Before\nEmbedded body\nAfter"));
    assert!(!stdout.contains("rendered from"));
    assert!(!stdout.contains("```json"));
}

#[test]
fn test_web_render_mode_base_output_defaults_to_markdown_table() {
    let vault = TestVault::new();
    vault.create_note("task-open", "---\nstatus: open\n---\n");
    vault.create_file(
        "tasks.base",
        "views:\n  - type: table\n    name: Open Tasks\n    filters:\n      and:\n        - status == \"open\"\n    order:\n      - file.name\n",
    );
    vault.create_note("host", "![[tasks.base]]\n");

    let output = vault.web_get("/host.md");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("| name |"));
    assert!(stdout.contains("| [task-open](/task-open.md) |"));
    assert!(!stdout.contains("```json"));
    assert!(!stdout.contains("rendered from tasks.base"));
}

#[test]
fn test_web_route_renders_base_targets_as_markdown() {
    let vault = TestVault::new();
    vault.create_note("task-open", "---\nstatus: open\n---\n");
    vault.create_file(
        "All Opputunities Logs.base",
        "views:\n  - type: table\n    name: Open Tasks\n    filters:\n      and:\n        - status == \"open\"\n    order:\n      - file.name\n",
    );

    let output = vault.web_get("/All%20Opputunities%20Logs.base");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("| name |"));
    assert!(stdout.contains("| [task-open](/task-open.md) |"));
    assert!(!stdout.contains("views:"));
}

#[test]
fn test_web_render_mode_reuses_recursive_note_and_base_expansion() {
    let vault = TestVault::new();
    vault.create_note("task-open", "---\nstatus: open\n---\n");
    vault.create_file(
        "tasks.base",
        "views:\n  - type: table\n    name: Open Tasks\n    filters:\n      and:\n        - status == \"open\"\n    order:\n      - file.name\n",
    );
    vault.create_note("note-a", "Nested\n![[tasks.base]]\n");
    vault.create_note("host", "![[note-a]]\n");

    let output = vault.web_get("/host.md");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Nested"));
    assert!(stdout.contains("| [task-open](/task-open.md) |"));
}

#[test]
fn test_web_render_mode_preserves_placeholders_and_quote_containers() {
    let vault = TestVault::new();
    vault.create_note("host", "> ![[missing-note]]\n");

    let output = vault.web_get("/host.md");

    assert_cli_success(&output);
    assert!(stderr_contains(
        &output,
        "embedded note 'missing-note' not found in index, skipping."
    ));
    assert!(stdout_contains(
        &output,
        "> <!-- [markbase] note 'missing-note' not found -->"
    ));
}

#[test]
fn test_web_output_rewrites_wikilinks_to_canonical_routes() {
    let vault = TestVault::new();
    vault.create_note_in_subdir("entities/person", "alice", "Target\n");
    vault.create_note("host", "[[alice]] and [[alice|Alias]]\n");

    let output = vault.web_get("/host.md");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[alice](/entities/person/alice.md)"));
    assert!(stdout.contains("[Alias](/entities/person/alice.md)"));
}

#[test]
fn test_web_render_mode_frontmatter_wikilink_field_matches_this_file_name() {
    let vault = TestVault::new();
    vault.create_note_in_subdir("entities/company", "acme", "![[company-person.base]]\n");
    vault.create_note_in_subdir(
        "entities/person",
        "alice",
        "---\ntype: person\ncompany: \"[[acme]]\"\ndescription: Account owner\n---\n",
    );
    vault.create_note_in_subdir(
        "entities/person",
        "bob",
        "---\ntype: person\ncompany: \"[[other]]\"\ndescription: Not this company\n---\n",
    );
    vault.create_file(
        "base-views/company-person.base",
        "views:\n  - type: table\n    name: Company People\n    filters:\n      and:\n        - type == \"person\"\n        - company == this.file.name\n    order:\n      - file.name\n      - description\n",
    );

    let output = vault.web_get("/entities/company/acme.md");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("| [alice](/entities/person/alice.md) |"));
    assert!(!stdout.contains("| [bob](/entities/person/bob.md) |"));
}

#[test]
fn test_web_output_rewrites_base_wikilinks_to_canonical_routes() {
    let vault = TestVault::new();
    vault.create_file(
        "views/All Opputunities Logs.base",
        "views:\n  - type: table\n    name: All Logs\n",
    );
    vault.create_note(
        "host",
        "[[All Opputunities Logs.base]]\n[[All Opputunities Logs.base|商机活动记录]]\n",
    );

    let output = vault.web_get("/host.md");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[All Opputunities Logs.base](/views/All%20Opputunities%20Logs.base)"));
    assert!(stdout.contains("[商机活动记录](/views/All%20Opputunities%20Logs.base)"));
}

#[test]
fn test_web_output_percent_encodes_emitted_urls() {
    let vault = TestVault::new();
    vault.create_note_in_subdir("entities/person", "张 三", "Target\n");
    vault.create_note("host", "[[张 三]]\n");

    let output = vault.web_get("/host.md");

    assert_cli_success(&output);
    assert!(stdout_contains(
        &output,
        "[张 三](/entities/person/%E5%BC%A0%20%E4%B8%89.md)"
    ));
}

#[test]
fn test_web_output_uses_design_contract_link_text_for_heading_and_block_links() {
    let vault = TestVault::new();
    vault.create_note_in_subdir("entities/person", "alice", "# Heading\n");
    vault.create_note(
        "host",
        "[[alice#Heading]]\n[[alice#Heading|Alias]]\n[[alice#^blockid]]\n[[alice#^blockid|Alias]]\n",
    );

    let output = vault.web_get("/host.md");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[alice > Heading](/entities/person/alice.md)"));
    assert!(stdout.contains("[Alias](/entities/person/alice.md)"));
    assert!(stdout.contains("[alice](/entities/person/alice.md)"));
}

#[test]
fn test_web_output_rewrites_resource_embeds() {
    let vault = TestVault::new();
    vault.create_file("assets/image.png", "png-bytes");
    vault.create_file("docs/file.pdf", "pdf-bytes");
    vault.create_file("files/blob.bin", "bin-bytes");
    vault.create_note("host", "![[image.png]]\n![[file.pdf]]\n![[blob.bin]]\n");

    let output = vault.web_get("/host.md");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("![](/assets/image.png)"));
    assert!(stdout.contains("[file.pdf](/docs/file.pdf)"));
    assert!(stdout.contains("[blob.bin](/files/blob.bin)"));
}

#[test]
fn test_web_output_removes_comments_and_preserves_deferred_syntax() {
    let vault = TestVault::new();
    vault.create_note(
        "host",
        "A %%hidden%% B\n![[note#Heading]]\n![[note#^blockid]]\n",
    );

    let output = vault.web_get("/host.md");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("A  B"));
    assert!(stdout.contains("![[note#Heading]]"));
    assert!(stdout.contains("![[note#^blockid]]"));
}

#[test]
fn test_web_output_preserves_code_fence_and_inline_code_literals() {
    let vault = TestVault::new();
    vault.create_note_in_subdir("entities/person", "alice", "Target\n");
    vault.create_file("assets/image.png", "png-bytes");
    vault.create_note(
        "host",
        "Inline `[[alice]]` and `%%comment%%`\n```\n[[alice]]\n![[image.png]]\n%%comment%%\n```\n[[alice]]\n",
    );

    let output = vault.web_get("/host.md");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("`[[alice]]`"));
    assert!(stdout.contains("`%%comment%%`"));
    assert!(stdout.contains("```\n[[alice]]\n![[image.png]]\n%%comment%%\n```"));
    assert!(stdout.contains("[alice](/entities/person/alice.md)"));
}

#[test]
fn test_web_output_preserves_quote_container_fenced_code_literals_after_embed_expansion() {
    let vault = TestVault::new();
    vault.create_note_in_subdir("entities/person", "alice", "Target\n");
    vault.create_file("assets/image.png", "png-bytes");
    vault.create_note(
        "snippet",
        "```\n[[alice]]\n![[image.png]]\n%%comment%%\n```\n",
    );
    vault.create_note("host", "> [!info]\n> ![[snippet]]\n[[alice]]\n");

    let output = vault.web_get("/host.md");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("> [!info]"));
    assert!(stdout.contains("> ```\n> [[alice]]\n> ![[image.png]]\n> %%comment%%\n> ```"));
    assert!(stdout.contains("[alice](/entities/person/alice.md)"));
}

#[test]
fn test_web_resource_route_streams_bytes_with_correct_content_type() {
    let vault = TestVault::new();
    vault.create_file("assets/image.png", "png-bytes");
    vault.create_file("assets/audio.mp3", "mp3-bytes");
    vault.create_file("data/report.json", "{\"ok\":true}");
    vault.create_file("files/blob.bin", "bin-bytes");
    vault.create_file("index.html", &docsify_shell_stub());
    let port = pick_free_port();
    let _server = vault.spawn_web_server("127.0.0.1", port);

    let image = http_get(port, "/assets/image.png");
    let audio = http_get(port, "/assets/audio.mp3");
    let json = http_get(port, "/data/report.json");
    let binary = http_get(port, "/files/blob.bin");

    assert_eq!(image.status_code, 200);
    assert_eq!(image.headers.get("content-type").unwrap(), "image/png");
    assert_eq!(
        image.headers.get("cache-control").unwrap(),
        "no-store, no-cache, must-revalidate"
    );
    assert_eq!(image.headers.get("pragma").unwrap(), "no-cache");
    assert_eq!(image.headers.get("expires").unwrap(), "0");
    assert_eq!(image.body, b"png-bytes");

    assert_eq!(audio.status_code, 200);
    assert_eq!(audio.headers.get("content-type").unwrap(), "audio/mpeg");
    assert_eq!(audio.body, b"mp3-bytes");

    assert_eq!(json.status_code, 200);
    assert_eq!(
        json.headers.get("content-type").unwrap(),
        "application/json; charset=utf-8"
    );
    assert_eq!(json.body, br#"{"ok":true}"#);

    assert_eq!(binary.status_code, 200);
    assert_eq!(
        binary.headers.get("content-type").unwrap(),
        "application/octet-stream"
    );
    assert_eq!(binary.body, b"bin-bytes");
}

#[test]
fn test_web_serve_cache_control_override_replaces_default_no_cache_headers() {
    let vault = TestVault::new();
    vault.create_file("assets/image.png", "png-bytes");
    vault.create_file("index.html", &docsify_shell_stub());
    let port = pick_free_port();
    let _server =
        vault.spawn_web_server_with_cache_control("127.0.0.1", port, Some("public, max-age=60"));

    let image = http_get(port, "/assets/image.png");

    assert_eq!(image.status_code, 200);
    assert_eq!(image.headers.get("content-type").unwrap(), "image/png");
    assert_eq!(
        image.headers.get("cache-control").unwrap(),
        "public, max-age=60"
    );
    assert_eq!(image.headers.get("pragma"), None);
    assert_eq!(image.headers.get("expires"), None);
}

#[test]
fn test_web_serve_cli_surface_matches_docs() {
    let vault = TestVault::new();
    let output = vault.run_cli(&["web", "serve", "--help"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--bind"));
    assert!(stdout.contains("--port"));
    assert!(stdout.contains("--homepage"));
    assert!(stdout.contains("--cache-control"));

    let readme = include_str!("../README.md");
    assert!(readme.contains("markbase web serve"));
    assert!(readme.contains("markbase web serve --homepage /HOME.md"));
    assert!(readme.contains("markbase web serve --cache-control"));
    assert!(readme.contains("127.0.0.1:3000"));
}

#[test]
fn test_web_init_docsify_help_positions_command_as_optional_export_tool() {
    let vault = TestVault::new();
    let output = vault.run_cli(&["web", "init-docsify", "--help"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("not required for normal browser use"));
    assert!(stdout.contains("note name, vault-relative file.path, or canonical URL"));
}

#[test]
fn test_web_get_matches_web_serve_for_note_targets() {
    let vault = TestVault::new();
    vault.create_note_in_subdir("entities/person", "alice", "[[alice]]\n");
    vault.create_note("task-open", "---\nstatus: open\n---\n");
    vault.create_file(
        "tasks.base",
        "views:\n  - type: table\n    name: Open Tasks\n    filters:\n      and:\n        - status == \"open\"\n    order:\n      - file.name\n",
    );
    vault.create_file("index.html", &docsify_shell_stub());
    let port = pick_free_port();
    let _server = vault.spawn_web_server("127.0.0.1", port);

    let cli_output = vault.web_get("/entities/person/alice.md");
    let http_response = http_get(port, "/entities/person/alice.md");
    let base_cli_output = vault.web_get("/tasks.base");
    let base_http_response = http_get(port, "/tasks.base");

    assert_cli_success(&cli_output);
    assert_eq!(http_response.status_code, 200);
    assert_eq!(
        String::from_utf8_lossy(&cli_output.stdout),
        String::from_utf8_lossy(&http_response.body)
    );

    assert_cli_success(&base_cli_output);
    assert_eq!(base_http_response.status_code, 200);
    assert_eq!(
        String::from_utf8_lossy(&base_cli_output.stdout),
        String::from_utf8_lossy(&base_http_response.body)
    );
    assert!(String::from_utf8_lossy(&base_http_response.body).contains("| name |"));
}

#[test]
fn test_web_get_refuses_binary_resource_targets() {
    let vault = TestVault::new();
    vault.create_file("assets/image.png", "png-bytes");

    let output = vault.web_get("/assets/image.png");

    assert_cli_error(&output);
    assert!(stderr_contains(&output, "does not stream resource bytes"));
}

#[test]
fn test_web_interface_behavior_matches_docs() {
    let readme = include_str!("../README.md");
    let architecture = include_str!("../ARCHITECTURE.md");

    assert!(readme.contains("markbase web get <canonical-url>"));
    assert!(readme.contains("request-scoped DuckDB handle"));
    assert!(readme.contains("route miss returns `404 Not Found`"));
    assert!(architecture.contains("src/web/"));
    assert!(
        architecture.contains("Canonical browser routes resolve by vault-relative `file.path`.")
    );
}

#[test]
fn test_web_init_docsify_requires_homepage() {
    let vault = TestVault::new();

    let output = vault.run_cli(&["web", "init-docsify"]);

    assert_cli_error(&output);
}

#[test]
fn test_web_init_docsify_writes_index_html_to_base_dir_root() {
    let vault = TestVault::new();
    create_home_note(&vault);

    let output = vault.web_init_docsify("/HOME.md");

    assert_cli_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "index.html");
    let index_path = vault.path.join("index.html");
    assert!(index_path.exists());
}

#[test]
fn test_web_init_docsify_writes_single_file_shell_output() {
    let vault = TestVault::new();
    create_home_note(&vault);

    let output = vault.web_init_docsify("/HOME.md");

    assert_cli_success(&output);
    let mut entries = fs::read_dir(&vault.path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    entries.sort();
    assert!(entries.contains(&"index.html".to_string()));
    assert!(!entries.iter().any(|name| name.ends_with(".css")));
    assert!(!entries.iter().any(|name| name.ends_with(".js")));
}

#[test]
fn test_web_init_docsify_refuses_overwrite_without_force() {
    let vault = TestVault::new();
    create_home_note(&vault);
    let index_path = vault.path.join("index.html");
    fs::write(&index_path, "old").unwrap();

    let output = vault.web_init_docsify("/HOME.md");

    assert_cli_error(&output);
    assert!(stderr_contains(&output, "already exists"));
    assert_eq!(fs::read_to_string(&index_path).unwrap(), "old");

    let forced = vault.web_init_docsify_force("/HOME.md");
    assert_cli_success(&forced);
    assert!(
        fs::read_to_string(&index_path)
            .unwrap()
            .contains("homepage: \"/HOME.md\"")
    );
}

#[test]
fn test_web_init_docsify_embeds_configured_homepage() {
    let vault = TestVault::new();
    create_base_homepage(&vault, "All Opputunities Logs.base");

    let output = vault.web_init_docsify("/All%20Opputunities%20Logs.base");

    assert_cli_success(&output);
    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("homepage: \"/All%20Opputunities%20Logs.base\""));
}

#[test]
fn test_web_init_docsify_embeds_markbase_build_metadata_in_shell() {
    let vault = TestVault::new();
    create_home_note(&vault);

    let output = vault.web_init_docsify("/HOME.md");

    assert_cli_success(&output);
    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("Generated by markbase"));
    assert!(html.contains("<meta name=\"markbase-version\" content=\""));
    assert!(html.contains("<meta name=\"markbase-git-commit\" content=\""));
    assert!(html.contains("<meta\n      name=\"markbase-git-commit-time\""));
    assert!(html.contains("<!-- markbase-shell-version: "));
    assert!(html.contains(&format!("v{}", env!("MARKBASE_BUILD_VERSION"))));
    assert!(html.contains("<span class=\"mb-shell-footer-value\">commit "));
    assert!(!html.contains("__MARKBASE_GIT_COMMIT__"));
    assert!(!html.contains("__MARKBASE_GIT_COMMIT_TIME__"));
}

#[test]
fn test_web_serve_requires_usable_exported_entry_html_when_homepage_is_not_provided() {
    let missing_vault = TestVault::new();
    let missing = missing_vault.run_cli(&["web", "serve"]);
    assert_cli_error(&missing);
    assert!(stderr_contains(
        &missing,
        "was started without `--homepage`, so it can only reuse the exported docsify entry HTML"
    ));
    assert!(stderr_contains(&missing, "does not exist"));

    let stale_vault = TestVault::new();
    create_home_note(&stale_vault);
    assert_cli_success(&stale_vault.web_init_docsify("/HOME.md"));
    let index_path = stale_vault.path.join("index.html");
    let html = fs::read_to_string(&index_path).unwrap();
    fs::write(
        &index_path,
        html.replace(
            &format!(
                "<!-- markbase-shell-version: {} -->",
                env!("MARKBASE_BUILD_VERSION")
            ),
            "<!-- markbase-shell-version: 0.0.0-test -->",
        ),
    )
    .unwrap();

    let stale = stale_vault.run_cli(&["web", "serve"]);
    assert_cli_error(&stale);
    assert!(stderr_contains(
        &stale,
        "can only reuse the exported docsify entry HTML"
    ));
    assert!(stderr_contains(&stale, "0.0.0-test"));
}

#[test]
fn test_web_serve_uses_exported_entry_html_when_version_matches() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("/HOME.md"));

    let port = pick_free_port();
    let _server = vault.spawn_web_server("127.0.0.1", port);

    let response = http_get(port, "/");

    assert_eq!(response.status_code, 200);
    let body = String::from_utf8_lossy(&response.body);
    assert!(body.contains("homepage: \"/HOME.md\""));
    assert_eq!(
        response.headers.get("content-type").unwrap(),
        "text/html; charset=utf-8"
    );
}

#[test]
fn test_web_serve_can_dynamically_serve_entry_html_without_exported_index() {
    let vault = TestVault::new();
    create_home_note(&vault);

    let port = pick_free_port();
    let server = vault.spawn_web_server_with_homepage("127.0.0.1", port, "HOME");

    let response = http_get(port, "/");

    assert_eq!(response.status_code, 200);
    let body = String::from_utf8_lossy(&response.body);
    assert!(body.contains("window.$docsify"));
    assert!(body.contains("homepage: \"/HOME.md\""));
    assert!(
        server
            .stderr_contents()
            .contains("INFO: serving dynamic docsify entry HTML for homepage '/HOME.md'.")
    );
}

#[test]
fn test_web_serve_with_homepage_ignores_existing_exported_entry_html() {
    let vault = TestVault::new();
    create_home_note(&vault);
    vault.create_file("index.html", &docsify_shell_stub_with_homepage("/OLD.md"));

    let port = pick_free_port();
    let server = vault.spawn_web_server_with_homepage("127.0.0.1", port, "HOME");

    let response = http_get(port, "/");
    let body = String::from_utf8_lossy(&response.body);
    assert!(body.contains("homepage: \"/HOME.md\""));
    assert!(!body.contains("<body>shell</body>"));
    assert!(
        server
            .stderr_contents()
            .contains("will not be used because `--homepage` requested dynamic mode")
    );
}

#[test]
fn test_web_root_serves_generated_index_html() {
    let vault = TestVault::new();
    create_home_note(&vault);
    let port = pick_free_port();
    let _server = vault.spawn_web_server_with_homepage("127.0.0.1", port, "/HOME.md");

    let response = http_get(port, "/");

    assert_eq!(response.status_code, 200);
    assert_eq!(
        response.headers.get("content-type").unwrap(),
        "text/html; charset=utf-8"
    );
    assert_eq!(
        response.headers.get("cache-control").unwrap(),
        "no-store, no-cache, must-revalidate"
    );
    assert_eq!(response.headers.get("pragma").unwrap(), "no-cache");
    assert_eq!(response.headers.get("expires").unwrap(), "0");
    let body = String::from_utf8_lossy(&response.body);
    assert!(body.contains("window.$docsify"));
    assert!(body.contains("homepage: \"/HOME.md\""));
}

#[test]
fn test_web_dynamic_entry_html_matches_init_docsify_output_byte_for_byte() {
    let vault = TestVault::new();
    create_home_note(&vault);
    let port = pick_free_port();
    let _server = vault.spawn_web_server_with_homepage("127.0.0.1", port, "/HOME.md");

    let dynamic_response = http_get(port, "/");
    assert_eq!(dynamic_response.status_code, 200);

    let output = vault.web_init_docsify("HOME");
    assert_cli_success(&output);

    let exported = fs::read(vault.path.join("index.html")).unwrap();
    assert_eq!(dynamic_response.body, exported);
}

#[test]
fn test_web_entry_html_embeds_homepage_metadata() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let exported = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(exported.contains("<!-- markbase-docsify-homepage: \"/HOME.md\" -->"));
    assert!(
        exported.contains(
            "<meta\n      name=\"markbase-docsify-homepage\"\n      content=\"/HOME.md\""
        )
    );

    fs::remove_file(vault.path.join("index.html")).unwrap();
    let port = pick_free_port();
    let _server = vault.spawn_web_server_with_homepage("127.0.0.1", port, "HOME");
    let dynamic = http_get(port, "/");
    let dynamic_html = String::from_utf8_lossy(&dynamic.body);
    assert!(dynamic_html.contains("<!-- markbase-docsify-homepage: \"/HOME.md\" -->"));
}

#[test]
fn test_web_homepage_input_resolves_to_existing_canonical_url() {
    let vault = TestVault::new();
    vault.create_note_in_subdir("areas/home", "HOME", "# Home\n");

    assert_cli_success(&vault.web_init_docsify("HOME"));
    let html_from_name = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html_from_name.contains("homepage: \"/areas/home/HOME.md\""));

    assert_cli_success(&vault.web_init_docsify_force("areas/home/HOME.md"));
    let html_from_path = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html_from_path.contains("homepage: \"/areas/home/HOME.md\""));

    let port = pick_free_port();
    let _server = vault.spawn_web_server_with_homepage("127.0.0.1", port, "/areas/home/HOME.md");
    let response = http_get(port, "/");
    let body = String::from_utf8_lossy(&response.body);
    assert!(body.contains("homepage: \"/areas/home/HOME.md\""));
}

#[test]
fn test_web_homepage_input_rejects_non_document_targets() {
    let vault = TestVault::new();
    vault.create_file("assets/image.png", "png");

    let init_output = vault.web_init_docsify("assets/image.png");
    assert_cli_error(&init_output);
    assert!(stderr_contains(
        &init_output,
        "Homepage only supports `.md` and `.base` targets."
    ));

    let serve_output = vault.run_cli(&["web", "serve", "--homepage", "assets/image.png"]);
    assert_cli_error(&serve_output);
    assert!(stderr_contains(
        &serve_output,
        "Homepage only supports `.md` and `.base` targets."
    ));
}

#[test]
fn test_web_serve_logs_clear_entry_html_mode_info() {
    let exported_vault = TestVault::new();
    create_home_note(&exported_vault);
    assert_cli_success(&exported_vault.web_init_docsify("HOME"));
    let exported_port = pick_free_port();
    let exported_server = exported_vault.spawn_web_server("127.0.0.1", exported_port);

    let dynamic_vault = TestVault::new();
    create_home_note(&dynamic_vault);
    dynamic_vault.create_file("index.html", &docsify_shell_stub());
    let dynamic_port = pick_free_port();
    let dynamic_server =
        dynamic_vault.spawn_web_server_with_homepage("127.0.0.1", dynamic_port, "HOME");

    assert!(
        exported_server
            .stderr_contents()
            .contains("INFO: using exported docsify entry HTML")
    );
    assert!(
        dynamic_server
            .stderr_contents()
            .contains("INFO: serving dynamic docsify entry HTML for homepage '/HOME.md'.")
    );
    assert!(
        dynamic_server
            .stderr_contents()
            .contains("WARN: exported docsify entry HTML")
    );
}

#[test]
fn test_web_serve_emits_access_logs_for_success_and_not_found() {
    let vault = TestVault::new();
    create_home_note(&vault);
    let port = pick_free_port();
    let server = vault.spawn_web_server_with_homepage("127.0.0.1", port, "/HOME.md");

    let root = http_get(port, "/");
    let missing = http_get(port, "/missing.md");

    assert_eq!(root.status_code, 200);
    assert_eq!(missing.status_code, 404);

    let stderr = server.stderr_contents();
    assert!(stderr.contains("ACCESS: GET / 200"));
    assert!(stderr.contains("ACCESS: GET /missing.md 404"));
}

#[test]
fn test_web_dynamic_entry_html_serves_root_and_index_routes_consistently() {
    let vault = TestVault::new();
    create_home_note(&vault);
    let port = pick_free_port();
    let _server = vault.spawn_web_server_with_homepage("127.0.0.1", port, "/HOME.md");

    let root = http_get(port, "/");
    let index = http_get(port, "/index.html");

    assert_eq!(root.status_code, 200);
    assert_eq!(index.status_code, 200);
    assert_eq!(root.body, index.body);
    assert_eq!(
        root.headers.get("content-type").unwrap(),
        "text/html; charset=utf-8"
    );
    assert_eq!(
        index.headers.get("content-type").unwrap(),
        "text/html; charset=utf-8"
    );
}

#[test]
fn test_web_dynamic_entry_html_preserves_docsify_frontend_contract() {
    let vault = TestVault::new();
    create_home_note(&vault);
    let port = pick_free_port();
    let _server = vault.spawn_web_server_with_homepage("127.0.0.1", port, "/HOME.md");

    let response = http_get(port, "/");
    let html = String::from_utf8_lossy(&response.body);
    assert!(html.contains("externalLinkTarget: \"_self\""));
    assert!(
        html.contains("noCompileLinks: [\"/.*\\\\.md(?:[?#].*)?\", \"/.*\\\\.base(?:[?#].*)?\"]")
    );
    assert!(html.contains("function normalizeDocsifyDom() {"));
    assert!(html.contains("function upgradeCalloutsDom() {"));
    assert!(html.contains("Generated by markbase"));
}

#[test]
fn test_web_init_docsify_plugin_rewrites_internal_document_links() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("externalLinkTarget: \"_self\""));
    assert!(
        html.contains("noCompileLinks: [\"/.*\\\\.md(?:[?#].*)?\", \"/.*\\\\.base(?:[?#].*)?\"]")
    );
    assert!(html.contains("function normalizeDocsifyDom() {"));
    assert!(html.contains("new MutationObserver(function () {"));
    assert!(html.contains(
        ".querySelectorAll(\".markdown-section a[href], .sidebar a[href], nav a[href]\")"
    ));
    assert!(html.contains(
        ".querySelectorAll(\".markdown-section img[data-origin], .sidebar img[data-origin]\")"
    ));
    assert!(html.contains("path.endsWith(\".md\") || path.endsWith(\".base\")"));
    assert!(html.contains("a.setAttribute(\"href\", \"#\" + href)"));
    assert!(html.contains("a.removeAttribute(\"target\")"));
    assert!(html.contains("img.setAttribute(\"src\", original)"));
}

#[test]
fn test_web_init_docsify_plugin_intercepts_same_page_section_links() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("function parseDocsifyRouteHref(href) {"));
    assert!(html.contains("function currentDocsifyRoutePath() {"));
    assert!(html.contains("function normalizeDocsifyDocumentPath(pathname) {"));
    assert!(html.contains("function scrollToDocsifySectionAnchor(anchorId) {"));
    assert!(html.contains("function handleDocsifySectionLinkClick(event) {"));
    assert!(html.contains(
        "const link = event.target.closest(\".sidebar a[href], a.section-link[href]\");"
    ));
    assert!(html.contains("const anchorId = route.searchParams.get(\"id\");"));
    assert!(html.contains("const targetPath = normalizeDocsifyDocumentPath(route.pathname);"));
    assert!(
        html.contains("if (!currentPath || !targetPath || targetPath !== currentPath) return;")
    );
    assert!(html.contains("target.scrollIntoView({ block: \"start\", behavior: \"auto\" });"));
    assert!(html.contains("event.stopImmediatePropagation();"));
    assert!(
        html.contains("document.addEventListener(\"click\", handleDocsifySectionLinkClick, true);")
    );
}

#[test]
fn test_web_init_docsify_sidebar_only_targets_markdown_note_routes() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("function eligibleMetadataRoutePath(route) {"));
    assert!(html.contains("if (!pathname || !pathname.endsWith(\".md\")) return null;"));
    assert!(html.contains("renderDocsifySidebar(\"hidden\", \"\");"));
    assert!(html.contains("const includeMetadataTabs = status !== \"hidden\";"));
}

#[test]
fn test_web_init_docsify_sidebar_metadata_request_uses_canonical_note_path_only() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("const sidebarMetadataFields = \"properties,links\";"));
    assert!(html.contains("function buildMetadataRequestPath(notePath) {"));
    assert!(html.contains("return notePath + \"?fields=\" + sidebarMetadataFields;"));
    assert!(html.contains("const requestPath = buildMetadataRequestPath(notePath);"));
    assert!(!html.contains("fetch(window.location.hash"));
    assert!(!html.contains("fields=properties%2Clinks"));
}

#[test]
fn test_web_init_docsify_sidebar_ignores_same_note_section_anchor_navigation() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains(
        "const link = event.target.closest(\".sidebar a[href], a.section-link[href]\");"
    ));
    assert!(html.contains("const anchorId = route.searchParams.get(\"id\");"));
    assert!(html.contains("const targetPath = normalizeDocsifyDocumentPath(route.pathname);"));
    assert!(
        html.contains("if (!currentPath || !targetPath || targetPath !== currentPath) return;")
    );
    assert!(html.contains("event.stopImmediatePropagation();"));
}

#[test]
fn test_web_init_docsify_sidebar_skips_unsupported_routes_without_metadata_errors() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("const notePath = eligibleMetadataRoutePath(route);"));
    assert!(html.contains("if (!notePath) {"));
    assert!(html.contains("clearDocsifySidebarRequest();"));
    assert!(html.contains("renderDocsifySidebar(\"hidden\", \"\");"));
    assert!(html.contains("showDocsifySidebarPanel(shell, \"outline\");"));
}

#[test]
fn test_web_init_docsify_sidebar_prevents_stale_response_overwrite() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("function shouldIgnoreSidebarResponse(notePath, requestId) {"));
    assert!(
        html.contains("return state.notePath !== notePath || state.activeRequestId !== requestId;")
    );
    assert!(html.contains("if (shouldIgnoreSidebarResponse(notePath, requestId)) return;"));
    assert!(html.contains("state.requestId += 1;"));
    assert!(html.contains("state.activeRequestId = state.requestId;"));
}

#[test]
fn test_web_init_docsify_sidebar_metadata_failure_does_not_block_note_body() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("renderDocsifySidebar(\"loading\", \"Loading metadata...\");"));
    assert!(html.contains("renderDocsifySidebar(\"error\", \"Metadata unavailable.\");"));
    assert!(html.contains("fetch(requestPath, requestOptions)"));
    assert!(html.contains("hook.doneEach(function () {"));
    assert!(html.contains("normalizeDocsifyDom();"));
    assert!(html.contains("syncDocsifyOutlinePanel(shell);"));
}

#[test]
fn test_web_init_docsify_sidebar_uses_outline_as_default_tab() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("activeTab: \"outline\","));
    assert!(html.contains("state.activeTab = \"outline\";"));
    assert!(html.contains("const activeTab = state.activeTab || \"outline\";"));
    assert!(html.contains("{ key: \"outline\", label: \"Outline\" }"));
}

#[test]
fn test_web_init_docsify_includes_callout_upgrade_plugin() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains(".mb-callout {"));
    assert!(html.contains("function upgradeCalloutsDom() {"));
    assert!(html.contains("function parseCalloutMetadata(firstParagraph) {"));
    assert!(html.contains("const calloutIconSvg = {"));
    assert!(html.contains("const foldMarkerSvg ="));
    assert!(html.contains("upgradeCalloutsDom();"));
}

#[test]
fn test_web_init_docsify_callout_plugin_recognizes_foldable_markers() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains(
        "const match = firstLine.match(/^\\[!([A-Za-z0-9_-]+)\\]([+-])?(?:\\s+(.*))?$/);"
    ));
    assert!(html.contains("foldable: foldMarker !== \"\""));
    assert!(html.contains("defaultOpen: foldMarker === \"+\""));
}

#[test]
fn test_web_init_docsify_callout_plugin_uses_stable_default_titles() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("const defaultCalloutTitles = {"));
    assert!(html.contains("info: \"Info\""));
    assert!(html.contains("faq: \"FAQ\""));
    assert!(html.contains("function defaultTitleForCallout(calloutType) {"));
    assert!(html.contains("explicitTitle || defaultTitleForCallout(calloutType)"));
}

#[test]
fn test_web_init_docsify_callout_plugin_uses_details_summary() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("document.createElement(metadata.foldable ? \"details\" : \"div\")"));
    assert!(html.contains("document.createElement(metadata.foldable ? \"summary\" : \"div\")"));
    assert!(html.contains("if (metadata.foldable && metadata.defaultOpen) {"));
    assert!(html.contains("wrapper.setAttribute(\"open\", \"\")"));
    assert!(html.contains("foldMarker.innerHTML = foldMarkerSvg"));
    assert!(html.contains("foldMarker.className = \"mb-callout-fold-marker\""));
    assert!(html.contains("title.appendChild(titleLabel);"));
    assert!(html.contains("title.appendChild(foldMarker);"));
}

#[test]
fn test_web_init_docsify_callout_plugin_renders_title_icons() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("const calloutIconSvg = {"));
    assert!(html.contains("const agentCalloutIconSvg ="));
    assert!(html.contains("titleIcon.className = \"mb-callout-icon\""));
    assert!(html.contains("titleIcon.innerHTML = iconSvgForCallout(metadata.calloutType)"));
    assert!(html.contains("calloutType === \"agent\" || calloutType.startsWith(\"agent-\")"));
}

#[test]
fn test_web_init_docsify_callout_plugin_preserves_nested_callouts() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("function calloutDepth(blockquote) {"));
    assert!(html.contains(".sort(function (left, right) {"));
    assert!(html.contains("return calloutDepth(right) - calloutDepth(left);"));
    assert!(html.contains("upgradeCalloutBlockquote(blockquote);"));
}

#[test]
fn test_web_init_docsify_callout_plugin_preserves_multiline_body_structure() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("function appendTextWithPreservedLineBreaks(target, text) {"));
    assert!(html.contains("const textParts = text.split(/(\\r?\\n)/);"));
    assert!(html.contains("target.appendChild(document.createElement(\"br\"));"));
    assert!(html.contains("function buildFirstParagraphRemainderParagraph(firstParagraph) {"));
    assert!(html.contains("trimBoundaryLineBreaks(paragraph);"));
}

#[test]
fn test_web_init_docsify_callout_plugin_preserves_line_breaks_around_inline_code() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("function appendNodeWithPreservedLineBreaks(target, node) {"));
    assert!(html.contains("if (node.nodeType === Node.TEXT_NODE) {"));
    assert!(html.contains("target.appendChild(node.cloneNode(true));"));
    assert!(html.contains("appendNodeWithPreservedLineBreaks(paragraph, node);"));
}

#[test]
fn test_web_init_docsify_callout_plugin_preserves_list_structure() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("const body = document.createElement(\"div\");"));
    assert!(html.contains("body.className = \"mb-callout-body\";"));
    assert!(html.contains("let sibling = firstParagraph.nextSibling;"));
    assert!(html.contains("body.appendChild(sibling);"));
    assert!(html.contains("const nextSibling = sibling.nextSibling;"));
}

#[test]
fn test_web_init_docsify_callout_plugin_preserves_backend_markdown_contract() {
    let vault = TestVault::new();
    vault.create_note("callout-demo", "> [!info]\n> Body\n");

    let output = vault.web_get("/callout-demo.md");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("> [!info]"));
    assert!(stdout.contains("> Body"));
    assert!(!stdout.contains("mb-callout"));
    assert!(!stdout.contains("<details"));
}

#[test]
fn test_web_init_docsify_callout_multiline_fix_preserves_backend_markdown_contract() {
    let vault = TestVault::new();
    create_multiline_callout_note(&vault, "callout-demo");

    let output = vault.web_get("/callout-demo.md");

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("> [!agent-update]- Overwrite"));
    assert!(stdout.contains("> `- 官网`"));
    assert!(stdout.contains("> `  - 官方首页：<官方首页 URL>`"));
    assert!(stdout.contains("> - 第一项"));
    assert!(!stdout.contains("mb-callout"));
    assert!(!stdout.contains("<details"));
}

#[test]
fn test_web_init_docsify_callout_changes_do_not_regress_navigation_plugin() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(
        html.contains("noCompileLinks: [\"/.*\\\\.md(?:[?#].*)?\", \"/.*\\\\.base(?:[?#].*)?\"]")
    );
    assert!(html.contains("a.setAttribute(\"href\", \"#\" + href)"));
    assert!(html.contains("img.setAttribute(\"src\", original)"));
}

#[test]
fn test_web_init_docsify_callout_multiline_fix_does_not_regress_existing_frontend_contract() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("document.createElement(metadata.foldable ? \"details\" : \"div\")"));
    assert!(html.contains("titleIcon.innerHTML = iconSvgForCallout(metadata.calloutType)"));
    assert!(html.contains("function calloutDepth(blockquote) {"));
    assert!(
        html.contains("noCompileLinks: [\"/.*\\\\.md(?:[?#].*)?\", \"/.*\\\\.base(?:[?#].*)?\"]")
    );
    assert!(html.contains("img.setAttribute(\"src\", original)"));
}

#[test]
fn test_web_init_docsify_plugin_leaves_binary_resource_urls_untouched() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("if (!(path.endsWith(\".md\") || path.endsWith(\".base\"))) return;"));
    assert!(html.contains("if (!original.startsWith(\"/\")) return;"));
}

#[test]
fn test_web_init_docsify_callout_docs_match_behavior() {
    let readme = fs::read_to_string("README.md").unwrap();
    let architecture = fs::read_to_string("ARCHITECTURE.md").unwrap();
    let design = fs::read_to_string(
        "docs/design-docs/implemented/design-012-docsify-frontend-integration.md",
    )
    .unwrap();

    assert!(readme.contains("browser entry HTML upgrades Obsidian-style callouts"));
    assert!(readme.contains("multiline body structure"));
    assert!(readme.contains("single `index.html`"));
    assert!(readme.contains("required for normal browser use"));
    assert!(architecture.contains("callout UI"));
    assert!(architecture.contains("multiline body preservation"));
    assert!(architecture.contains("frontend-only"));
    assert!(design.contains("multiline body preservation"));
    assert!(design.contains("flattening multiline content into a single paragraph"));
}
