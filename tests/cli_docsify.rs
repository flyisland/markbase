mod common;

use common::{TestVault, assert_cli_success};
use std::fs;

fn create_home_note(vault: &TestVault) {
    vault.create_note("HOME", "# Home\n");
}

#[test]
fn test_docsify_sidebar_uses_tabbed_panel_contract() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("function docsifySidebarTabs(includeMetadataTabs) {"));
    assert!(html.contains("{ key: \"outline\", label: \"Outline\" }"));
    assert!(html.contains("{ key: \"properties\", label: \"Properties\" }"));
    assert!(html.contains("{ key: \"links\", label: \"Links\" }"));
    assert!(html.contains("tabs.className = \"mb-note-sidebar-tabs\";"));
    assert!(html.contains("panelStack.className = \"mb-note-sidebar-panels\";"));
    assert!(html.contains("outlinePanel.id = \"mb-note-sidebar-outline-panel\";"));
    assert!(html.contains("metadataPanel.id = \"mb-note-sidebar-panel\";"));
    assert!(html.contains("function renderSidebarTab(tab, isActive, metadataPanelId) {"));
    assert!(html.contains("button.setAttribute(\"role\", \"tab\");"));
    assert!(html.contains("metadataPanel.appendChild(renderLinksSection(metadata.links));"));
    assert!(
        html.contains("metadataPanel.appendChild(renderPropertiesSection(metadata.properties));")
    );
}

#[test]
fn test_docsify_sidebar_defaults_to_outline_tab() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("activeTab: \"outline\","));
    assert!(html.contains("state.activeTab = \"outline\";"));
    assert!(html.contains("const activeTab = state.activeTab || \"outline\";"));
    assert!(html.contains("if (state.activeTab === tab.key) return;"));
    assert!(html.contains("state.activeTab = tab.key;"));
    assert!(html.contains("showDocsifySidebarPanel(shell, activeTab);"));
}

#[test]
fn test_docsify_sidebar_uses_unified_left_sidebar_layout() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("const docsifySidebar = document.querySelector(\".sidebar\");"));
    assert!(html.contains(".mb-note-sidebar-body {"));
    assert!(html.contains("border-top: 1px solid #eee;"));
    assert!(html.contains("grid-template-rows: auto minmax(0, 1fr);"));
    assert!(html.contains(".mb-note-sidebar-tabs {"));
    assert!(html.contains("position: sticky;"));
    assert!(html.contains("top: 0;"));
    assert!(html.contains(".mb-note-sidebar-panel {"));
    assert!(html.contains(".mb-note-sidebar-panel-outline .sidebar-nav,"));
}

#[test]
fn test_docsify_sidebar_reuses_docsify_outline_dom_in_outline_tab() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("function syncDocsifyOutlinePanel(shell) {"));
    assert!(html.contains("child.classList.contains(\"sidebar-nav\")"));
    assert!(html.contains("child.classList.contains(\"app-sub-sidebar\")"));
    assert!(html.contains("outlinePanel.appendChild(child);"));
    assert!(html.contains("syncDocsifyOutlinePanel(shell);"));
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
fn test_docsify_sidebar_adapts_note_and_base_links_to_docsify_routes() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("function docsifySidebarHref(href) {"));
    assert!(html.contains("if (path.endsWith(\".md\") || path.endsWith(\".base\")) {"));
    assert!(html.contains("return \"#\" + href;"));
    assert!(html.contains("link.href = docsifySidebarHref(entry.href);"));
}

#[test]
fn test_docsify_sidebar_preserves_resource_and_unresolved_link_behavior() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("return href;"));
    assert!(
        html.contains("unresolved.className = \"mb-sidebar-unresolved mb-sidebar-link-label\";")
    );
    assert!(html.contains("if (entry.exists && entry.href) {"));
}

#[test]
fn test_docsify_sidebar_adapts_rich_text_wikilinks_to_docsify_routes() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("function renderRichTextSegments(segments) {"));
    assert!(html.contains("if (segment.type === \"wikilink\") {"));
    assert!(html.contains("link.href = docsifySidebarHref(segment.href);"));
    assert!(html.contains("unresolved.className = \"mb-sidebar-unresolved\";"));
    assert!(html.contains(".mb-sidebar-unresolved::after {"));
}

#[test]
fn test_docsify_sidebar_includes_state_dom_contract() {
    let vault = TestVault::new();
    create_home_note(&vault);
    assert_cli_success(&vault.web_init_docsify("HOME"));

    let html = fs::read_to_string(vault.path.join("index.html")).unwrap();
    assert!(html.contains("body.className = \"mb-note-sidebar-body\";"));
    assert!(html.contains("tabs.className = \"mb-note-sidebar-tabs\";"));
    assert!(html.contains("panelStack.className = \"mb-note-sidebar-panels\";"));
    assert!(html.contains(
        "outlinePanel.className = \"mb-note-sidebar-panel mb-note-sidebar-panel-outline\";"
    ));
    assert!(html.contains(
        "metadataPanel.className =\n                  \"mb-note-sidebar-panel mb-note-sidebar-panel-metadata\";"
    ));
    assert!(html.contains("function renderSidebarStateMessage(status, message) {"));
    assert!(html.contains("state.className = \"mb-note-sidebar-state\";"));
    assert!(html.contains("tabs.replaceChildren();"));
    assert!(html.contains("metadataPanel.replaceChildren();"));
    assert!(html.contains("const includeMetadataTabs = status !== \"hidden\";"));
}
