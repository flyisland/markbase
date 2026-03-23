mod common;

use common::{TestVault, assert_cli_success};
use std::fs;

fn create_home_note(vault: &TestVault) {
    vault.create_note("HOME", "# Home\n");
}

#[test]
fn test_docsify_sidebar_includes_desktop_two_column_layout_contract() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains(".mb-note-page {"));
    assert!(html.contains("display: flex;"));
    assert!(html.contains("align-items: flex-start;"));
    assert!(html.contains(".mb-note-sidebar {"));
    assert!(html.contains("flex: 0 0 320px;"));
}

#[test]
fn test_docsify_sidebar_includes_mobile_stack_layout_contract() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("@media (max-width: 960px) {"));
    assert!(html.contains(".mb-note-page {"));
    assert!(html.contains("flex-direction: column;"));
    assert!(html.contains(".mb-note-sidebar-body {"));
    assert!(html.contains("position: static;"));
}

#[test]
fn test_docsify_sidebar_renders_property_semantic_value_kinds() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("function renderSidebarValueNode(node) {"));
    assert!(html.contains("if (node.kind === \"null\") {"));
    assert!(html.contains("if (node.kind === \"scalar\") {"));
    assert!(html.contains("if (node.kind === \"rich_text\") {"));
    assert!(html.contains("if (node.kind === \"list\") {"));
    assert!(html.contains("if (node.kind === \"object\") {"));
}

#[test]
fn test_docsify_sidebar_renders_rich_text_wikilink_segments() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("function renderRichTextSegments(segments) {"));
    assert!(html.contains("if (segment.type === \"wikilink\") {"));
    assert!(html.contains("if (segment.exists && segment.href) {"));
    assert!(html.contains("link.href = segment.href;"));
    assert!(html.contains("unresolved.className = \"mb-sidebar-unresolved\";"));
    assert!(html.contains(".mb-sidebar-unresolved::after {"));
}

#[test]
fn test_docsify_sidebar_links_section_uses_current_metadata_contract_only() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("function renderLinksSection(links) {"));
    assert!(html.contains("kind.textContent = entry.kind || \"link\";"));
    assert!(html.contains("link.href = entry.href;"));
    assert!(html.contains("link.textContent = entry.target || \"\";"));
    assert!(!html.contains("entry.alias"));
    assert!(!html.contains("entry.source"));
}

#[test]
fn test_docsify_sidebar_includes_state_dom_contract() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("body.className = \"mb-note-sidebar-body\";"));
    assert!(html.contains("const body = sidebar.querySelector(\".mb-note-sidebar-body\");"));
    assert!(html.contains("function renderSidebarStateMessage(status, message) {"));
    assert!(html.contains("state.className = \"mb-note-sidebar-state\";"));
    assert!(html.contains("sidebar.dataset.sidebarState = status;"));
    assert!(html.contains("body.replaceChildren();"));
    assert!(html.contains("if (status === \"loading\" || status === \"error\") {"));
    assert!(html.contains("sidebar.hidden = true;"));
}
