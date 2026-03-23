            function ensureDocsifySidebarContainer() {
              const content = document.querySelector("section.content");
              if (!content) return null;

              let page = content.querySelector(".mb-note-page");
              if (!page) {
                page = document.createElement("div");
                page.className = "mb-note-page";

                const markdownSection = content.querySelector(".markdown-section");
                if (markdownSection) {
                  content.insertBefore(page, markdownSection);
                  page.appendChild(markdownSection);
                } else {
                  const footer = content.querySelector(".mb-shell-footer");
                  if (footer) {
                    content.insertBefore(page, footer);
                  } else {
                    content.appendChild(page);
                  }
                }
              }

              const markdownSection = content.querySelector(".markdown-section");
              if (markdownSection && markdownSection.parentElement !== page) {
                page.insertBefore(markdownSection, page.firstChild);
              }

              let sidebar = page.querySelector(".mb-note-sidebar");
              if (!sidebar) {
                sidebar = document.createElement("aside");
                sidebar.className = "mb-note-sidebar";
                sidebar.hidden = true;
                sidebar.setAttribute("aria-live", "polite");

                const body = document.createElement("div");
                body.className = "mb-note-sidebar-body";

                const tabs = document.createElement("div");
                tabs.className = "mb-note-sidebar-tabs";
                tabs.setAttribute("role", "tablist");
                tabs.setAttribute("aria-label", "Note metadata");

                const panel = document.createElement("div");
                panel.className = "mb-note-sidebar-panel";
                panel.id = "mb-note-sidebar-panel";

                body.appendChild(tabs);
                body.appendChild(panel);
                sidebar.appendChild(body);

                page.appendChild(sidebar);
              }

              return sidebar;
            }

            function docsifySidebarTabs() {
              return [
                { key: "properties", label: "Properties" },
                { key: "links", label: "Links" },
              ];
            }

            function ensureDocsifySidebarActiveTab() {
              const state = docsifySidebarState();
              const tabs = docsifySidebarTabs();
              const activeTab = state.activeTab || "properties";

              if (tabs.some(function (tab) { return tab.key === activeTab; })) {
                state.activeTab = activeTab;
                return activeTab;
              }

              state.activeTab = "properties";
              return state.activeTab;
            }

            function docsifySidebarHref(href) {
              if (!href) return "";
              if (href.startsWith("#")) return href;
              if (!href.startsWith("/")) return href;

              const path = href.split("#")[0].split("?")[0];
              if (path.endsWith(".md") || path.endsWith(".base")) {
                return "#" + href;
              }

              return href;
            }

            function sidebarPanelSection(title, bodyContent) {
              const section = document.createElement("section");
              section.className = "mb-note-sidebar-panel-section";

              const heading = document.createElement("h2");
              heading.className = "mb-note-sidebar-panel-title";
              heading.textContent = title;
              section.appendChild(heading);

              const body = document.createElement("div");
              body.className = "mb-note-sidebar-panel-content";
              if (bodyContent) {
                body.appendChild(bodyContent);
              }
              section.appendChild(body);

              return section;
            }

            function sidebarEmptyState(message) {
              const empty = document.createElement("p");
              empty.className = "mb-note-sidebar-empty";
              empty.textContent = message;
              return empty;
            }

            function schemaHint(label, kind) {
              const hint = document.createElement("span");
              hint.className = "mb-sidebar-schema-hint";
              hint.dataset.hintKind = kind;
              hint.textContent = label;
              return hint;
            }

            function renderSchemaHints(schema) {
              if (!schema) return null;

              const hints = document.createElement("div");
              hints.className = "mb-sidebar-schema-hints";

              if (schema.required) {
                hints.appendChild(schemaHint("required", "required"));
              }
              if (schema.type) {
                hints.appendChild(schemaHint(schema.type, "type"));
              }
              if (schema.format) {
                hints.appendChild(schemaHint(schema.format, "format"));
              }

              return hints.childNodes.length > 0 ? hints : null;
            }

            function renderRichTextSegments(segments) {
              const container = document.createElement("span");
              container.className = "mb-sidebar-rich-text";

              (segments || []).forEach(function (segment) {
                if (!segment || !segment.type) return;

                if (segment.type === "text") {
                  container.appendChild(document.createTextNode(segment.text || ""));
                  return;
                }

                if (segment.type === "wikilink") {
                  if (segment.exists && segment.href) {
                    const link = document.createElement("a");
                    link.className = "mb-sidebar-link";
                    link.href = docsifySidebarHref(segment.href);
                    link.textContent = segment.text || segment.target || "";
                    container.appendChild(link);
                    return;
                  }

                  const unresolved = document.createElement("span");
                  unresolved.className = "mb-sidebar-unresolved";
                  unresolved.textContent = segment.text || segment.target || "";
                  container.appendChild(unresolved);
                }
              });

              return container;
            }

            function renderSidebarValueNode(node) {
              if (!node || !node.kind) {
                return sidebarEmptyState("No value");
              }

              if (node.kind === "null") {
                const empty = document.createElement("span");
                empty.className = "mb-sidebar-placeholder";
                empty.textContent = "null";
                return empty;
              }

              if (node.kind === "scalar") {
                const scalar = document.createElement("span");
                scalar.textContent =
                  node.value === null || node.value === undefined
                    ? ""
                    : String(node.value);
                return scalar;
              }

              if (node.kind === "rich_text") {
                return renderRichTextSegments(node.segments);
              }

              if (node.kind === "list") {
                const list = document.createElement("ul");
                list.className = "mb-sidebar-list-value";

                (node.items || []).forEach(function (item) {
                  const entry = document.createElement("li");
                  entry.appendChild(renderSidebarValueNode(item));
                  list.appendChild(entry);
                });

                return list;
              }

              if (node.kind === "object") {
                const object = document.createElement("div");
                object.className = "mb-sidebar-object-fields";

                (node.fields || []).forEach(function (field) {
                  const row = document.createElement("div");
                  row.className = "mb-sidebar-property";

                  const header = document.createElement("div");
                  header.className = "mb-sidebar-property-header";

                  const key = document.createElement("span");
                  key.className = "mb-sidebar-property-key";
                  key.textContent = field.key || "";
                  header.appendChild(key);

                  const value = document.createElement("div");
                  value.className = "mb-sidebar-object-value";
                  value.appendChild(renderSidebarValueNode(field.value));

                  row.appendChild(header);
                  row.appendChild(value);
                  object.appendChild(row);
                });

                return object;
              }

              return sidebarEmptyState("Unsupported value");
            }

            function renderPropertiesSection(properties) {
              const fields = properties && Array.isArray(properties.fields) ? properties.fields : [];
              if (fields.length === 0) {
                return sidebarPanelSection("Properties", sidebarEmptyState("No properties"));
              }

              const container = document.createElement("div");
              container.className = "mb-sidebar-properties";

              fields.forEach(function (field) {
                const row = document.createElement("div");
                row.className = "mb-sidebar-property";

                const header = document.createElement("div");
                header.className = "mb-sidebar-property-header";

                const key = document.createElement("span");
                key.className = "mb-sidebar-property-key";
                key.textContent = field.key || "";
                header.appendChild(key);

                const hints = renderSchemaHints(field.schema);
                if (hints) {
                  header.appendChild(hints);
                }

                const value = document.createElement("div");
                value.className = "mb-sidebar-property-value";
                value.appendChild(renderSidebarValueNode(field.value));

                row.appendChild(header);
                row.appendChild(value);
                container.appendChild(row);
              });

              return sidebarPanelSection("Properties", container);
            }

            function renderLinkRow(entry) {
              const row = document.createElement("li");
              row.className = "mb-sidebar-links-row";

              const kind = document.createElement("span");
              kind.className = "mb-sidebar-link-kind";
              kind.textContent = entry.kind || "link";
              row.appendChild(kind);

              if (entry.exists && entry.href) {
                const link = document.createElement("a");
                link.className = "mb-sidebar-link-label";
                link.href = docsifySidebarHref(entry.href);
                link.textContent = entry.target || "";
                row.appendChild(link);
              } else {
                const unresolved = document.createElement("span");
                unresolved.className = "mb-sidebar-unresolved mb-sidebar-link-label";
                unresolved.textContent = entry.target || "";
                row.appendChild(unresolved);
              }

              return row;
            }

            function renderLinksSection(links) {
              const entries = Array.isArray(links) ? links : [];
              if (entries.length === 0) {
                return sidebarPanelSection("Links", sidebarEmptyState("No links"));
              }

              const list = document.createElement("ul");
              list.className = "mb-sidebar-links-list";
              entries.forEach(function (entry) {
                list.appendChild(renderLinkRow(entry));
              });

              return sidebarPanelSection("Links", list);
            }

            function renderSidebarStateMessage(status, message) {
              const state = document.createElement("div");
              state.className = "mb-note-sidebar-state";
              state.textContent = message;
              state.dataset.stateKind = status;
              return state;
            }

            function renderSidebarTab(tab, isActive) {
              const button = document.createElement("button");
              button.type = "button";
              button.className = "mb-note-sidebar-tab";
              button.dataset.sidebarTab = tab.key;
              button.setAttribute("role", "tab");
              button.setAttribute("aria-controls", "mb-note-sidebar-panel");
              button.setAttribute("aria-selected", isActive ? "true" : "false");
              button.setAttribute("tabindex", isActive ? "0" : "-1");
              button.textContent = tab.label;

              if (isActive) {
                button.dataset.active = "true";
              }

              button.addEventListener("click", function () {
                const state = docsifySidebarState();
                if (state.activeTab === tab.key) return;

                state.activeTab = tab.key;

                const sidebar = ensureDocsifySidebarContainer();
                if (!sidebar) return;
                if (sidebar.dataset.sidebarState === "ready") {
                  renderDocsifySidebar("ready", "");
                }
                const panel = sidebar.querySelector(".mb-note-sidebar-panel");
                if (panel) {
                  panel.scrollTop = 0;
                }
              });

              return button;
            }

            function renderDocsifySidebar(status, message) {
              const sidebar = ensureDocsifySidebarContainer();
              if (!sidebar) return;

              const body = sidebar.querySelector(".mb-note-sidebar-body");
              const tabs = sidebar.querySelector(".mb-note-sidebar-tabs");
              const panel = sidebar.querySelector(".mb-note-sidebar-panel");
              if (!body || !tabs || !panel) return;

              sidebar.dataset.sidebarState = status;
              sidebar.setAttribute("aria-busy", status === "loading" ? "true" : "false");

              if (status === "hidden") {
                sidebar.hidden = true;
                tabs.replaceChildren();
                panel.replaceChildren();
                return;
              }

              sidebar.hidden = false;
              const activeTab = ensureDocsifySidebarActiveTab();

              tabs.replaceChildren();
              docsifySidebarTabs().forEach(function (tab) {
                tabs.appendChild(renderSidebarTab(tab, tab.key === activeTab));
              });

              panel.replaceChildren();

              if (status === "loading" || status === "error") {
                panel.appendChild(renderSidebarStateMessage(status, message || ""));
                return;
              }

              const metadata = docsifySidebarState().metadata || {};
              if (activeTab === "links") {
                panel.appendChild(renderLinksSection(metadata.links));
                return;
              }

              panel.appendChild(renderPropertiesSection(metadata.properties));
            }
