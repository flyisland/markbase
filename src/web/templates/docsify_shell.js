            const defaultCalloutTitles = {
              abstract: "Abstract",
              attention: "Attention",
              bug: "Bug",
              caution: "Caution",
              check: "Check",
              cite: "Cite",
              danger: "Danger",
              done: "Done",
              error: "Error",
              example: "Example",
              fail: "Fail",
              failure: "Failure",
              faq: "FAQ",
              help: "Help",
              hint: "Hint",
              important: "Important",
              info: "Info",
              missing: "Missing",
              note: "Note",
              question: "Question",
              quote: "Quote",
              success: "Success",
              summary: "Summary",
              tip: "Tip",
              tldr: "TLDR",
              todo: "Todo",
              warning: "Warning",
            };

            const calloutIconSvg = {
              abstract:
                '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"></circle><path d="M9.5 9.5h5"></path><path d="M9.5 12h5"></path><path d="M9.5 14.5h3.5"></path></svg>',
              bug:
                '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 7.5h6"></path><path d="M10 4.5h4"></path><rect x="7.5" y="7.5" width="9" height="10" rx="4"></rect><path d="M4.5 9.5h3"></path><path d="M16.5 9.5h3"></path><path d="M4.5 14.5h3"></path><path d="M16.5 14.5h3"></path></svg>',
              danger:
                '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3.5l9 16H3z"></path><path d="M12 9v4.5"></path><path d="M12 17h.01"></path></svg>',
              error:
                '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"></circle><path d="M9 9l6 6"></path><path d="M15 9l-6 6"></path></svg>',
              faq:
                '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"></circle><path d="M9.75 9.25a2.75 2.75 0 1 1 4.36 2.22c-.88.62-1.61 1.19-1.61 2.28"></path><path d="M12 17h.01"></path></svg>',
              help:
                '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"></circle><path d="M9.75 9.25a2.75 2.75 0 1 1 4.36 2.22c-.88.62-1.61 1.19-1.61 2.28"></path><path d="M12 17h.01"></path></svg>',
              important:
                '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3.5l9 16H3z"></path><path d="M12 8.5v5"></path><path d="M12 17h.01"></path></svg>',
              info:
                '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"></circle><path d="M12 10v6"></path><path d="M12 7h.01"></path></svg>',
              note:
                '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 4.5h7l4 4v11H7z"></path><path d="M14 4.5v4h4"></path></svg>',
              question:
                '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"></circle><path d="M9.75 9.25a2.75 2.75 0 1 1 4.36 2.22c-.88.62-1.61 1.19-1.61 2.28"></path><path d="M12 17h.01"></path></svg>',
              quote:
                '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9.5 9.5h-3v5h4v-2.5h-2"></path><path d="M17.5 9.5h-3v5h4v-2.5h-2"></path></svg>',
              success:
                '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"></circle><path d="M8.5 12.5l2.5 2.5 4.5-5"></path></svg>',
              tip:
                '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 18.5h6"></path><path d="M10 21h4"></path><path d="M8.5 14.5c-1.3-1-2-2.54-2-4.25a5.5 5.5 0 1 1 11 0c0 1.71-.7 3.25-2 4.25-.75.58-1.25 1.3-1.5 2.25h-4c-.25-.95-.75-1.67-1.5-2.25z"></path></svg>',
              warning:
                '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3.5l9 16H3z"></path><path d="M12 8.5v5"></path><path d="M12 17h.01"></path></svg>',
            };

            const agentCalloutIconSvg =
              '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="2.5"></circle><path d="M12 5.5v1.5"></path><path d="M12 17v1.5"></path><path d="M5.5 12h1.5"></path><path d="M17 12h1.5"></path><path d="M7.4 7.4l1.05 1.05"></path><path d="M15.55 15.55l1.05 1.05"></path><path d="M16.6 7.4l-1.05 1.05"></path><path d="M8.45 15.55L7.4 16.6"></path></svg>';

            const foldMarkerSvg =
              '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6.75L15 12l-6 5.25"></path></svg>';

            function defaultTitleForCallout(calloutType) {
              const knownTitle = defaultCalloutTitles[calloutType];
              if (knownTitle) return knownTitle;

              return calloutType
                .split(/[-_]/)
                .filter(function (segment) {
                  return segment.length > 0;
                })
                .map(function (segment) {
                  return segment.charAt(0).toUpperCase() + segment.slice(1);
                })
                .join(" ");
            }

            function iconSvgForCallout(calloutType) {
              if (calloutType === "agent" || calloutType.startsWith("agent-")) {
                return agentCalloutIconSvg;
              }

              return (
                calloutIconSvg[calloutType] ||
                '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 4.5h7l4 4v11H7z"></path><path d="M14 4.5v4h4"></path></svg>'
              );
            }

            function parseCalloutMetadata(firstParagraph) {
              const fullText = firstParagraph.textContent || "";
              const firstLine = fullText.split(/\r?\n/, 1)[0].trim();
              const match = firstLine.match(/^\[!([A-Za-z0-9_-]+)\]([+-])?(?:\s+(.*))?$/);
              if (!match) return null;

              const calloutType = match[1].toLowerCase();
              const foldMarker = match[2] || "";
              const explicitTitle = (match[3] || "").trim();

              return {
                calloutType: calloutType,
                foldable: foldMarker !== "",
                defaultOpen: foldMarker === "+",
                title: explicitTitle || defaultTitleForCallout(calloutType),
              };
            }

            function appendTextWithPreservedLineBreaks(target, text) {
              const textParts = text.split(/(\r?\n)/);
              let justAppendedBreak = false;

              textParts.forEach(function (part) {
                if (!part) return;

                if (/^\r?\n$/.test(part)) {
                  target.appendChild(document.createElement("br"));
                  justAppendedBreak = true;
                  return;
                }

                target.appendChild(document.createTextNode(part));
                justAppendedBreak = false;
              });

              return justAppendedBreak;
            }

            function appendNodeWithPreservedLineBreaks(target, node) {
              if (node.nodeType === Node.TEXT_NODE) {
                return appendTextWithPreservedLineBreaks(target, node.textContent || "");
              }

              if (node.nodeType === Node.ELEMENT_NODE && node.tagName === "BR") {
                target.appendChild(document.createElement("br"));
                return true;
              }

              target.appendChild(node.cloneNode(true));
              return false;
            }

            function trimBoundaryLineBreaks(container) {
              while (container.firstChild && container.firstChild.nodeName === "BR") {
                container.removeChild(container.firstChild);
              }

              while (container.lastChild && container.lastChild.nodeName === "BR") {
                container.removeChild(container.lastChild);
              }
            }

            function buildFirstParagraphRemainderParagraph(firstParagraph) {
              const paragraph = document.createElement("p");
              let sawFirstLineBreak = false;

              Array.from(firstParagraph.childNodes).forEach(function (node) {
                if (sawFirstLineBreak) {
                  appendNodeWithPreservedLineBreaks(paragraph, node);
                  return;
                }

                if (node.nodeType === Node.TEXT_NODE) {
                  const text = node.textContent || "";
                  const match = text.match(/\r?\n/);
                  if (!match || typeof match.index !== "number") return;

                  sawFirstLineBreak = true;
                  appendTextWithPreservedLineBreaks(
                    paragraph,
                    text.slice(match.index + match[0].length)
                  );
                  return;
                }

                if (node.nodeType === Node.ELEMENT_NODE && node.tagName === "BR") {
                  sawFirstLineBreak = true;
                }
              });

              if (!sawFirstLineBreak) return null;

              trimBoundaryLineBreaks(paragraph);
              if (!(paragraph.textContent || "").trim()) return null;
              return paragraph;
            }

            function calloutDepth(blockquote) {
              let depth = 0;
              let current = blockquote.parentElement;

              while (current) {
                if (current.tagName === "BLOCKQUOTE") {
                  depth += 1;
                }
                current = current.parentElement;
              }

              return depth;
            }

            function upgradeCalloutBlockquote(blockquote) {
              if (blockquote.dataset.mbCalloutUpgraded === "true") return;

              const firstParagraph = Array.from(blockquote.children).find(function (child) {
                return child.tagName === "P";
              });
              if (!firstParagraph) return;

              const metadata = parseCalloutMetadata(firstParagraph);
              if (!metadata) return;

              const wrapper = document.createElement(metadata.foldable ? "details" : "div");
              wrapper.className = "mb-callout";
              wrapper.dataset.callout = metadata.calloutType;
              wrapper.dataset.calloutFoldable = metadata.foldable ? "true" : "false";
              wrapper.dataset.mbCalloutUpgraded = "true";
              if (metadata.foldable && metadata.defaultOpen) {
                wrapper.setAttribute("open", "");
              }

              const title = document.createElement(metadata.foldable ? "summary" : "div");
              title.className = "mb-callout-title";

              const titleIcon = document.createElement("span");
              titleIcon.className = "mb-callout-icon";
              titleIcon.setAttribute("aria-hidden", "true");
              titleIcon.innerHTML = iconSvgForCallout(metadata.calloutType);
              title.appendChild(titleIcon);

              const titleLabel = document.createElement("span");
              titleLabel.className = "mb-callout-label";
              titleLabel.textContent = metadata.title;
              title.appendChild(titleLabel);

              if (metadata.foldable) {
                const foldMarker = document.createElement("span");
                foldMarker.className = "mb-callout-fold-marker";
                foldMarker.innerHTML = foldMarkerSvg;
                foldMarker.setAttribute("aria-hidden", "true");
                title.appendChild(foldMarker);
              }

              const body = document.createElement("div");
              body.className = "mb-callout-body";

              const remainderParagraph = buildFirstParagraphRemainderParagraph(firstParagraph);
              if (remainderParagraph) {
                body.appendChild(remainderParagraph);
              }

              let sibling = firstParagraph.nextSibling;
              while (sibling) {
                const nextSibling = sibling.nextSibling;
                body.appendChild(sibling);
                sibling = nextSibling;
              }

              wrapper.appendChild(title);
              wrapper.appendChild(body);
              blockquote.replaceWith(wrapper);
            }

            function upgradeCalloutsDom() {
              const blockquotes = Array.from(
                document.querySelectorAll(".markdown-section blockquote")
              );

              blockquotes
                .sort(function (left, right) {
                  return calloutDepth(right) - calloutDepth(left);
                })
                .forEach(function (blockquote) {
                  upgradeCalloutBlockquote(blockquote);
                });
            }

            function attachDocsifyFooter() {
              const footer = document.querySelector(".mb-shell-footer");
              if (!footer) return;

              const content = document.querySelector("section.content");
              if (!content) return;

              if (content.lastElementChild !== footer) {
                content.appendChild(footer);
              }
            }

            function parseDocsifyRouteHref(href) {
              if (!href || !href.startsWith("#/")) return null;

              try {
                return new URL(href.slice(1), "https://markbase.invalid");
              } catch (_error) {
                return null;
              }
            }

            function currentDocsifyRoutePath() {
              const route = parseDocsifyRouteHref(window.location.hash || "");
              return route ? route.pathname : null;
            }

            function normalizeDocsifyDocumentPath(pathname) {
              if (!pathname) return null;
              if (pathname.endsWith(".md")) return pathname.slice(0, -3);
              if (pathname.endsWith(".base")) return pathname.slice(0, -5);
              return pathname;
            }

            function scrollToDocsifySectionAnchor(anchorId) {
              if (!anchorId) return false;

              const target = document.getElementById(anchorId);
              if (!target) return false;

              target.scrollIntoView({ block: "start", behavior: "auto" });
              return true;
            }

            function handleDocsifySectionLinkClick(event) {
              if (event.defaultPrevented) return;
              if (event.button !== 0) return;
              if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;

              const link = event.target.closest("a.section-link[href]");
              if (!link) return;

              const route = parseDocsifyRouteHref(link.getAttribute("href"));
              if (!route) return;

              const anchorId = route.searchParams.get("id");
              if (!anchorId) return;

              const currentPath = normalizeDocsifyDocumentPath(currentDocsifyRoutePath());
              const targetPath = normalizeDocsifyDocumentPath(route.pathname);
              if (!currentPath || !targetPath || targetPath !== currentPath) return;

              if (!scrollToDocsifySectionAnchor(anchorId)) return;

              event.preventDefault();
              event.stopPropagation();
              event.stopImmediatePropagation();
            }
            function normalizeDocsifyDom() {
              document
                .querySelectorAll(".markdown-section a[href], .sidebar a[href], nav a[href]")
                .forEach(function (a) {
                  const href = a.getAttribute("href");
                  if (!href) return;
                  if (!href.startsWith("/")) return;
                  if (href.startsWith("//")) return;
                  if (href.startsWith("/#")) return;

                  const path = href.split("#")[0].split("?")[0];
                  if (!(path.endsWith(".md") || path.endsWith(".base"))) return;

                  a.setAttribute("href", "#" + href);
                  a.removeAttribute("target");
                  a.removeAttribute("rel");
                });

              document
                .querySelectorAll(".markdown-section img[data-origin], .sidebar img[data-origin]")
                .forEach(function (img) {
                  const original = img.getAttribute("data-origin");
                  if (!original) return;
                  if (!original.startsWith("/")) return;

                  img.setAttribute("src", original);
                });

              upgradeCalloutsDom();
              attachDocsifyFooter();
            }

            if (!window.__markbaseDocsifyObserverInstalled) {
              window.__markbaseDocsifyObserverInstalled = true;
              const observer = new MutationObserver(function () {
                normalizeDocsifyDom();
              });

              observer.observe(document.body, {
                childList: true,
                subtree: true,
                attributes: true,
                attributeFilter: ["href", "src", "data-origin", "open"],
              });
            }

            if (!window.__markbaseDocsifySectionLinkHandlerInstalled) {
              window.__markbaseDocsifySectionLinkHandlerInstalled = true;
              document.addEventListener("click", handleDocsifySectionLinkClick, true);
            }

            hook.doneEach(function () {
              normalizeDocsifyDom();
            });
