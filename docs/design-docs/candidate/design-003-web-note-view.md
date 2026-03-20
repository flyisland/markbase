---
id: design-003
title: "Web Note View"
status: candidate
module: web
---

# Web Note View

## Purpose

This document defines a web delivery design for viewing markbase-managed, Obsidian-compatible notes in a browser.

The goal is to reuse markbase's existing read-only render pipeline, keep the vault filesystem as the source of truth, and add only enough server-side and frontend translation to make notes render correctly in a browser.

## Summary

The proposed system has three layers:

1. A new markbase web server serves translated Markdown notes and raw resource files.
2. markbase performs server-side note rendering and OFM normalization before Markdown reaches any browser-side renderer.
3. docsify is one possible frontend consumer that renders the translated Markdown in the browser.

This design intentionally does not treat docsify or marked as the primary OFM execution engine. markbase remains responsible for semantics that depend on vault structure, DuckDB note metadata, or recursive note expansion.

The core boundary is:

- browser-side renderers should receive Markdown that `docsify + marked` can render correctly, not raw Obsidian syntax for features that require vault-aware semantics
- server-side output should preserve Markdown container semantics where practical instead of relying on frontend renderers to recover them later

## Layered Architecture

The v1 architecture is intentionally split into three independent layers:

```text
Browser
  -> docsify frontend
       - fetches Markdown and resources over HTTP
       - renders Markdown to HTML
       - applies presentation plugins such as Mermaid or callout styling
  -> markbase web serve
       - resolves canonical vault-shaped URLs
       - refreshes the index per request
       - renders notes
       - translates OFM into docsify/marked-renderable Markdown
       - streams raw resource bytes
  -> vault filesystem + derived DuckDB index
       - filesystem remains source of truth
       - DuckDB remains derived state
```

### Layering Principles

- `markbase` is a content translation and resource delivery service, not an HTML application server.
- `markbase web serve` returns translated Markdown for note routes and raw bytes for resource routes.
- `markbase` does not generate or serve a docsify shell, static site shell, or any other HTML entrypoint in v1.
- `docsify` is a replaceable frontend consumer. Its job is Markdown-to-HTML rendering, not vault-aware note semantics.
- Vault-aware behavior such as wikilink resolution, recursive note expansion, `.base` execution, and canonical URL generation belongs on the markbase side.
- Presentation-oriented behavior such as HTML layout, theme, and renderer plugins belongs on the frontend side.
- The boundary between the two layers is HTTP: translated Markdown and resource bytes flow out of markbase; rendered HTML is produced elsewhere.

### Output Terminology

This document uses `docsify/marked-renderable Markdown` to mean:

- Markdown text that the reference frontend stack, `docsify + marked`, can render correctly
- without requiring vault-aware link resolution, note expansion, or `.base` execution in the browser

This term is intentionally more precise than `browser-targeted Markdown`, which is too vague for an implementation contract.

## Current Leverage In Markbase

The existing codebase already provides the foundations needed for a browser delivery path:

- `src/link_syntax.rs` already scans Obsidian wikilinks and embeds, including alias and anchor parsing, while skipping fenced code blocks and inline code spans.
- `src/renderer/mod.rs` already performs recursive whole-note embed expansion for `![[note]]` and `.base` expansion for `![[file.base]]` and `![[file.base#View]]`.
- The `notes` table already stores both `file.name` and `file.path`, which makes it straightforward to map path-based web URLs onto markbase's existing name-based identity model.
- Current render contracts already define how embedded note bodies, nested `.base` renders, warnings, and placeholder comments behave.

This means the main new work is not note identity or `.base` execution. The main work is building a web-facing output pipeline and defining which OFM syntax is translated on the server versus in the frontend renderer.

## Design Goals

- Preserve markbase's name-based note identity internally.
- Support path-based web URLs derived from `file.path`.
- Reuse existing `note render` semantics wherever they already match browser needs.
- Keep vault-aware transformations on the server.
- Keep the frontend renderer focused on Markdown rendering and light presentation plugins.
- Prioritize OFM features that materially affect current template-generated notes.

## Non-Goals

- Reproducing Obsidian's reader UI chrome or exact visual behavior.
- Turning DuckDB into a durable content source.
- Adding path-based note identity to markbase's note-facing core APIs.
- Supporting every OFM extension in the first web release.
- Making selector-based note embeds part of the first web release.

## Route And Identity Model

### Canonical Web Identity

The canonical browser URL for a note is path-based and derived from `file.path`, for example:

```text
/entities/person/张三.md
```

Internally, markbase still resolves and renders notes by note name.

### Resolution Rules

- Incoming note requests are resolved by `file.path`.
- Once the note row is found, the server uses the corresponding `file.name` when it invokes note rendering logic.
- Incoming resource requests are resolved by `file.path` and streamed directly from disk.
- Path-based URLs are a web delivery contract only. They do not change markbase's core note-facing identity rules.

This preserves the existing architectural invariant that Markdown notes are name-addressed inside markbase while allowing browser URLs to reflect vault structure.

### Canonical Link Target

For browser-facing note links, the canonical target is the note's path-derived route, not its note name.

The canonical server-emitted note URL shape is:

```text
/<file.path>
```

The server-side Markdown contract must not depend on frontend-specific routing format.

If the frontend later chooses to use hash routing, aliases, or another renderer-specific route scheme, that adaptation belongs to the frontend layer rather than to server-emitted Markdown.

The stable rule is: link generation must use one canonical path-based browser URL and must not emit bare relative href values such as `note name`.

### URL Encoding And Request Matching

The logical route identity is still the vault-relative `file.path` string, but server-emitted URLs must be valid browser URLs.

The v1 contract is:

- `file.path` is the canonical logical lookup key for note and resource routes
- server-emitted URLs percent-encode path characters as required for browser-safe URLs
- incoming HTTP request paths are URL-decoded once before matching against indexed `file.path`
- route matching is performed against the decoded vault-relative path, not against a percent-encoded byte sequence
- undecodable request paths are rejected with `400 Bad Request`

Examples:

- a space in `file.path` is emitted as `%20`
- non-ASCII text such as `张三.md` is emitted using standard URL percent-encoding and decoded back before lookup
- reserved URL characters that are part of a filename, such as `#` or `?`, must be percent-encoded in emitted URLs and matched by their decoded filename form

This keeps browser URLs reversible without changing markbase's canonical note/resource identity model.

### Wiki-Link Conversion Rules

Server-side note rendering must convert live Obsidian wikilinks into final browser-facing links before the frontend renderer consumes the response.

The conversion algorithm is:

1. Parse the token using the shared `src/link_syntax.rs` contract.
2. Resolve `parsed.normalized_target` against indexed notes by note name.
3. Read the canonical `file.path` of the resolved note.
4. Construct the browser-facing route from `file.path`.
5. Choose display text from alias if present; otherwise use the logical note name.

The server must not emit Markdown links whose target is a bare note name such as:

```md
[张三](张三)
```

That form leaves route interpretation to the frontend and is not a stable markbase web contract.

#### Base Cases

`[[note]]` becomes:

```md
[note](/<file.path>)
```

`[[note|Alias]]` becomes:

```md
[Alias](/<file.path>)
```

#### Heading Links

For `[[note#Heading]]` and `[[note#Heading|Alias]]`, v1 should still generate a path-based note link, but heading-fragment behavior is only valid if the web renderer has a stable heading-anchor contract.

Until that anchor contract is explicitly specified, the safe v1 behavior is:

- if an alias exists, use the alias as display text
- otherwise use `note > Heading` as display text
- link to the canonical note route without claiming a stable in-page anchor

Example safe v1 output:

```md
[note > Heading](/<file.path>)
```

If a later design adds stable heading anchors, the conversion rule may be tightened to emit a URL fragment derived from that shared slugging rule.

#### Block Links

For `[[note#^blockid]]` and `[[note#^blockid|Alias]]`, the same rule applies:

- in v1, generate a canonical note-page link
- do not claim a stable block fragment unless block-anchor generation is explicitly implemented
- if an alias exists, use the alias as display text
- otherwise use `note` as display text

Example safe v1 output:

```md
[note](/<file.path>)
```

#### Missing Targets

If a wikilink target cannot be resolved to an indexed note, the server should preserve readable text and avoid producing a broken frontend-relative link.

The v1 contract is:

- unresolved wikilinks remain literal source text in the output
- the server does not emit a fallback clickable link
- the server does not emit a bare relative href based only on note name

Examples:

```md
[[missing-note]]
[[missing-note|Alias]]
[[missing-note#Heading]]
```

This rule keeps the downgrade path simple, preserves the author's original intent, and avoids inventing a second unresolved-link presentation contract in v1.

### Resource Embed Conversion Rules

For non-Markdown, non-`.base` embed targets, the server converts Obsidian embed syntax into browser-facing Markdown or HTML based on resource type.

The v1 mapping is:

- image resources become standard Markdown images: `![](/<file.path>)`
- PDF resources become standard links: `[target-name](/<file.path>)`
- other attachment resources become standard links: `[target-name](/<file.path>)`

For resource embeds, v1 does not implement Obsidian-specific size semantics from `alias_or_size`.

That means:

- `![[image.png]]` becomes `![](/<file.path>)`
- `![[image.png|200]]` follows the same mapping and ignores the size suffix in v1
- `![[file.pdf]]` becomes `[file.pdf](/<file.path>)`

If a resource embed target cannot be resolved to an indexed resource in v1:

- the original embed token remains literal output
- the server does not synthesize a broken Markdown image
- the server does not synthesize a fallback clickable resource link

## Delivery Model

### Index Freshness And Database Lifecycle

Web serving follows the same core principle as other DB-backed markbase commands: the index must be refreshed as part of serving the request.

The v1 contract is:

- each HTTP request performs an index refresh before route resolution or rendering begins
- the refresh reuses markbase's existing incremental indexing mechanism rather than inventing a second indexing path
- after the response is produced, the request-scoped database handle is closed
- the server does not keep a long-lived open DuckDB connection across requests

This design intentionally favors correctness and compatibility with other markbase usage over maximum throughput. The goal is to serve the freshest vault state on every request while minimizing interference with other local markbase commands.

### Server Responsibilities

The web server is the source of truth for vault-aware behavior:

- refresh the derived index before serving each request
- resolve incoming routes against indexed notes
- render Markdown notes through markbase's render pipeline
- stream non-Markdown assets such as images and PDFs
- rewrite Obsidian syntax that the frontend renderer cannot consume directly
- generate canonical browser-facing links for internal note references
- ensure the frontend renderer does not need vault-aware knowledge to interpret the response

### Frontend Responsibilities

The frontend renderer is responsible for browser presentation:

- fetch translated Markdown from the markbase server
- render CommonMark or Markdown-compatible output via marked
- apply lightweight display plugins where browser-side rendering is the better fit

docsify is the reference frontend for v1, but these concerns are intentionally outside markbase's HTTP contract.

### Request Flow

For Markdown note requests:

1. Browser requests the canonical note URL `/<file.path>`.
2. Server refreshes the index using the existing incremental indexing mechanism.
3. Server resolves the request to a `notes` row by `file.path`.
4. Server renders the note body using web note render semantics.
5. Server performs OFM compatibility translation on the rendered output.
6. Server returns docsify/marked-renderable Markdown text.
7. Server closes the request-scoped database handle.

For attachments:

1. Browser requests `/<relative-path>`.
2. Server refreshes the index using the existing incremental indexing mechanism.
3. Server resolves the request to a `notes` row or filesystem path.
4. Server streams the file bytes with the correct content type.
5. Server closes the request-scoped database handle.

### Route Miss Contract

For the vault-path-based HTTP route:

- if the decoded request path does not resolve to an indexed note or indexed resource, the server returns `404 Not Found`
- the v1 miss body is plain text
- note misses and resource misses use the same HTTP status contract

This keeps route resolution predictable and leaves frontend HTML-shell behavior outside markbase's scope.

## OFM Compatibility Strategy

### Processing Layers

Each syntax feature must be assigned to one target handling layer:

- `server`: markbase rewrites or expands it before the frontend sees it
- `marked-native`: the reference frontend stack can render it without special work
- `frontend-plugin`: the browser handles it with a frontend plugin
- `hybrid`: the server normalizes structure and the frontend adds presentation

### Decision Rule

Use `server` when the syntax:

- depends on vault or DuckDB metadata
- requires recursive note expansion
- requires canonical URL generation
- must remain stable regardless of frontend plugin behavior

Use `frontend-plugin` when the syntax:

- does not require vault context
- is presentation-oriented
- is already well-supported by a small plugin

### Docsify/Marked-Renderable Output Contract

Before Markdown reaches the frontend renderer, the server must eliminate or normalize live syntax that requires vault-aware interpretation.

Unless a feature explicitly defines different rules, this normalization should follow the same body/code-context boundary already used by markbase render: ordinary Markdown body content is transformed, while fenced code blocks and inline code spans are preserved as literal examples.

For v1 note responses, live source syntax in these categories must not remain in the final server response:

- live wikilinks such as `[[note]]`
- live resource embeds such as `![[image.png]]`
- live whole-note embeds such as `![[note]]`
- live `.base` embeds such as `![[tasks.base]]`
- live note or `.base` embed syntax inside supported blockquote/callout containers

For deferred features, literal passthrough is still acceptable in v1:

- selector-based note embeds such as `![[note#Heading]]`
- block-target note embeds such as `![[note#^blockid]]`
- block reference definitions such as `^blockid`

## Compatibility Matrix

| OFM syntax | reference frontend native support | Target handling layer | Existing markbase leverage | Priority | Effort | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| Standard Markdown and GFM basics: headings, lists, tables, task lists, code fences, blockquotes | Yes | `marked-native` | None needed beyond normal response delivery | P0 | None | Not a markbase-specific problem |
| `[[note]]`, `[[note#Heading]]`, `[[note\|Alias]]` | No | `server` | Shared parser already extracts normalized target, anchor, and alias; DB already stores `name`, `path`, and `ext` | P0 | Low | Rewrite to canonical web URLs before frontend rendering |
| `![[image.png]]`, `![[file.pdf]]`, other non-Markdown resource embeds | No | `server` | Shared embed parser already identifies targets; DB and filesystem already provide direct file lookup | P0 | Low | Rewrite to standard Markdown or HTML resource embeds and stream bytes from server |
| `![[note]]`, `![[note\|Alias]]` | No | `server` | Already implemented by current recursive note render pipeline | P0 | Very low | Reuse current whole-note embed expansion |
| `![[file.base]]`, `![[file.base#View]]` | No | `server` | Already implemented by current `.base` render pipeline | P0 | Very low | Reuse current `.base` execution and view selection |
| Obsidian callouts `> [!type]` including template-generated callouts | No stable native support | `server` | Current render pipeline already scans callout bodies as MarkdownBody content; the remaining work is preserving quote-container structure during live embed expansion | P0 | Medium | Service-side contract owns callout semantics; the frontend must not infer raw Obsidian callout meaning |
| `%%comment%%` | No | `server` | Simple text-level preprocessing; no DB or render contract changes needed | P1 | Low | Remove from output entirely |
| `==highlight==` | No | `server` | Inline preprocessing only; independent from indexing and render recursion | P1 | Low | Rewrite to `<mark>` |
| Mermaid code blocks | No native chart rendering | `frontend-plugin` | No markbase changes required beyond preserving fenced code blocks | P1 | Low | Use a docsify Mermaid plugin or equivalent lightweight custom plugin |
| Footnotes | Not reliably native | `deferred` | Little reusable markbase logic | P2 | Medium | Explicitly out of v1 scope |
| Math / LaTeX | Not native to marked rendering alone | `deferred` | Little reusable markbase logic | P2 | Medium | Explicitly out of v1 scope |
| `![[note#Heading]]` | No | `server` | Current parser preserves anchor text, but current render contract intentionally leaves selector-based note embeds literal | P2 | High | Requires heading extraction and recursive fragment rendering |
| `![[note#^blockid]]` and block references `^blockid` | No | `server` | Current parser preserves block selector text, but no block index or fragment extraction exists | P2 | High | Requires block boundary detection, extraction, and recursive fragment rendering |

## V1 Scope

The first web release includes:

- all P0 items in the compatibility matrix
- `%%comment%%` removal

The first web release does not include:

- `==highlight==`
- Mermaid rendering
- footnotes
- math / LaTeX
- selector-based note embeds such as `![[note#Heading]]`
- block-target note embeds such as `![[note#^blockid]]`
- block reference rendering

## Why Callouts Are P0

Callouts are P0 because current templates rely on them heavily and browser readability would regress immediately if they remain dependent on frontend-specific interpretation or if live embed expansion breaks the quote container.

Callouts also interact with existing markbase render behavior:

- current render logic already scans callout bodies as normal Markdown body content
- callouts are textually a specialized blockquote form, so preserving quote-container structure is the relevant server-side responsibility
- once the quote structure is preserved, the frontend renderer can render the resulting Markdown without needing to infer raw Obsidian callout semantics as the primary source of truth

For this reason, callouts should not be deferred to a later polish pass, and their meaning should not depend on a browser-only parser extension.

### Blockquote And Callout Container Rule

For live note embeds and live `.base` embeds, blockquotes and callouts are the same container class.

The contract is:

- if a live embed appears in a blockquote body or callout body, the expanded multi-line output remains inside that quote container
- preserving the container means each emitted output line receives the relevant quote prefix
- this is a server-side render responsibility, not a frontend plugin responsibility

The preservation rule is line-oriented:

- each emitted output line inherits the quote prefix depth of the embed-bearing source line
- blank lines remain inside the quote container
- nested quote depth is preserved
- for callouts, the existing marker line such as `> [!info]` remains unchanged; expanded content inherits the quote container prefix rather than generating a fresh marker line
- inline embeds inside quote containers are first expanded as block content, then each emitted line receives the preserved quote prefix
- soft-failure placeholder output produced at the embed position follows the same rule and remains inside the quote container

### List Item Rule

List items are intentionally outside the supported live-embed container contract.

The contract is:

- live note embeds inside list items remain literal output
- live `.base` embeds inside list items remain literal output
- the frontend is not expected to recover or emulate list-item embed semantics
- list-item exclusion takes precedence over nested blockquote or callout syntax on the same logical line

## Why Mermaid Is P1

Mermaid matters, but it is presentation-oriented and does not require vault-aware translation.

It is therefore a better fit for a frontend plugin than for the core markbase render pipeline. The server only needs to preserve Mermaid fenced code blocks unchanged.

## Web-Facing Render Pipeline

For Markdown note responses, the server pipeline should be:

1. Resolve the request path to a Markdown note row by `file.path`.
2. Render the note using existing markbase note render semantics plus the web output-shape rules defined below.
3. Run a server-side OFM normalization pass over the rendered Markdown.
4. Return docsify/marked-renderable Markdown in the HTTP response body.

### Web Output Shape

Web note rendering reuses the current render subsystem's semantic rules for:

- recursive whole-note expansion
- `.base` view selection
- soft-failure placeholders
- quote-container preservation

But web note rendering does not reuse the CLI output shape verbatim.

The web-mode output contract is:

- note responses are plain docsify/marked-renderable Markdown text
- `.base` embeds default to Markdown table output in web mode
- web mode does not emit the CLI's default fenced JSON output for `.base` sections
- web mode does not introduce a second debug wrapper format for v1

This means web delivery reuses render semantics, but it is a distinct output mode from `markbase note render` default stdout formatting.

### Required Server-Side Passes In V1

- rewrite `[[...]]` wikilinks to canonical browser URLs
- rewrite non-Markdown `![[...]]` embeds to standard Markdown or HTML embeds
- preserve existing whole-note embed expansion
- preserve existing `.base` expansion
- preserve blockquote/callout container structure for expanded live embeds
- remove `%%comment%%`
- ensure the response contains no remaining vault-aware syntax except features explicitly deferred as literal passthrough

### Explicitly Deferred From V1

- `==highlight==`
- Mermaid rendering
- selector-based note embeds such as `![[note#Heading]]`
- block-target note embeds such as `![[note#^blockid]]`
- block reference rendering beyond plain literal passthrough
- full footnote support
- math / LaTeX rendering

## Role Of `obsidian-export`

`obsidian-export` is not part of the primary hot-path design.

The reasons are:

- markbase already has the semantics for `.base` expansion and whole-note embed expansion that matter most for this vault model
- the current web problem is dominated by path routing, OFM normalization, and docsify/marked-renderable output shaping
- `obsidian-export` is structurally closer to a vault exporter than to a minimal request-time string transformer

It may still be useful for prototyping or for validating edge-case OFM behavior, but this design does not require it as a runtime dependency for note-serving requests.

## Warning And HTTP Output Contract

The existing CLI render contract uses stderr warnings plus stdout placeholders for soft failures.

For the web server:

- soft warnings are server-log concerns, not part of the HTTP response contract
- HTTP response bodies carry only the rendered Markdown plus any placeholder comments already defined by render contracts
- the server does not add a second warning channel to Markdown bodies for v1
- database connection lifetime is request-scoped; warnings do not require keeping DuckDB open after the response is produced
- route misses are represented by HTTP status codes rather than in-band Markdown warning bodies

## Proposed Public Interface

The design assumes a new web-serving command surface, likely:

```bash
markbase web serve
```

It also adds a direct inspection helper:

```bash
markbase web get <canonical-url>
```

The `web get` contract is:

- input is a canonical vault-shaped browser URL such as `/entities/person/张三.md`
- if the target resolves to a Markdown note, the command prints the fully processed web Markdown body that `markbase web serve` would return for that same request
- if the target resolves to an indexed non-Markdown resource such as an image or PDF, the command does not stream bytes and instead exits without rendering plus a short explanatory message
- if the target does not resolve, the command returns a not-found style failure consistent with the route-resolution contract

`web get` exists for human and agent verification of the web Markdown pipeline without needing to open a browser.

At minimum, the server-facing HTTP surface must expose:

- canonical vault-path note routes derived directly from `file.path` and returning translated Markdown
- canonical vault-path resource routes derived directly from `file.path` and returning raw file bytes

Exact CLI flags, bind defaults, and cache policy are intentionally deferred until a later task spec.

## Testing And Acceptance

The implementation should add coverage for:

- canonical note route resolution maps the requested `file.path` to the correct internal note name
- canonical note requests return translated Markdown rather than HTML shell content
- each request refreshes the index before route resolution
- request-scoped database handles are closed after the response is produced
- wikilinks are rewritten to canonical browser URLs
- server-emitted URLs are percent-encoded and request matching is performed against decoded `file.path`
- image and PDF embeds are rewritten and remain fetchable
- unresolved resource embeds remain literal source text
- whole-note embeds still expand recursively in web output
- `.base` embeds still render in web output
- web-mode `.base` output defaults to Markdown table shape rather than CLI default JSON fences
- callouts remain browser-visible quote containers after live embed expansion
- blockquote live embeds preserve quote-container structure
- quote-container preservation keeps blank lines and nested quote depth inside the container
- soft-failure placeholder comments also preserve quote-container structure
- comments are removed from output
- heading-selector and block-selector note embeds remain literal output in v1
- raw wikilink syntax does not remain in the final response for P0-supported notes
- wikilink conversion never emits bare relative href targets derived only from note names
- list-item live embeds remain literal output
- unresolved wikilinks remain literal source text
- route misses return `404 Not Found`
- image resources map to Markdown images and PDF resources map to standard links
- `markbase web get <canonical-url>` returns the same Markdown body as `markbase web serve` would return for note targets
- `markbase web get <canonical-url>` refuses to stream binary resource targets and exits with an explanatory message

## Assumptions

- docsify remains the reference frontend renderer for the first release.
- Path-based URLs are a presentation contract, not a change to internal note identity.
- Template-generated callouts are common enough that they must be handled in the first implementation phase.
- Whole-note note embeds and `.base` embeds are already production-ready enough to reuse as the semantic core of web note rendering.
- v1 may still return literal selector-based note embeds because those semantics are explicitly outside the current active render contract.

## Relationship To Other Docs

- `ARCHITECTURE.md` defines the global invariant that notes remain name-addressed internally and that DuckDB is derived state.
- `docs/design-docs/implemented/design-001-links-and-embeds.md` defines the shared parse contract and current note-embed boundaries used by this design.
- `docs/design-docs/implemented/design-002-render.md` defines the active note render semantics that this web design reuses.
- `README.md` should be updated when a user-facing web command or route behavior is implemented.
