mod common;

use common::{
    TestVault, assert_cli_error, assert_cli_success, http_get, pick_free_port, stderr_contains,
    stdout_contains,
};

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
    let port = pick_free_port();
    let _server = vault.spawn_web_server("127.0.0.1", port);

    let image = http_get(port, "/assets/image.png");
    let audio = http_get(port, "/assets/audio.mp3");
    let json = http_get(port, "/data/report.json");
    let binary = http_get(port, "/files/blob.bin");

    assert_eq!(image.status_code, 200);
    assert_eq!(image.headers.get("content-type").unwrap(), "image/png");
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
fn test_web_serve_cli_surface_matches_docs() {
    let vault = TestVault::new();
    let output = vault.run_cli(&["web", "serve", "--help"]);

    assert_cli_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--bind"));
    assert!(stdout.contains("--port"));

    let readme = include_str!("../README.md");
    assert!(readme.contains("markbase web serve"));
    assert!(readme.contains("127.0.0.1:3000"));
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
