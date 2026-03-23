---
id: design-014
title: "Docsify Note Sidebar UI"
status: implemented
module: web-frontend
---

# Docsify Note Sidebar UI

## Purpose

This document defines the docsify-side presentation contract for the note
metadata sidebar powered by the backend metadata mode defined in `design-013`.

It answers a frontend question, not a backend one:

- when metadata tabs should appear at all
- how docsify outline and note metadata should share one sidebar
- how the sidebar should react to route changes and request failures
- how semantic metadata nodes from the backend should be rendered as browser UI

## Relationship To Other Designs

- `design-003` defines the backend web note delivery contract
- `design-012` defines docsify shell installation and frontend integration
- `design-013` defines the implemented metadata route and JSON response
  contract

This document intentionally does not redefine the metadata route shape. It
consumes the semantic response defined in `design-013` and owns the browser UI
and fetch lifecycle above that contract.

## Scope

This design covers:

- the unified left-sidebar layout inside docsify's existing `.sidebar`
- the sidebar tab-strip structure and active-panel behavior
- how docsify outline content should be presented as an `Outline` tab
- which docsify routes are eligible for metadata tabs
- metadata request construction for eligible note routes
- sidebar section structure for `Properties` and `Links`
- rendering rules for `properties.fields[].value`
- docsify route adaptation for sidebar-internal note/base links
- loading, empty, and error states
- docsify route-change refresh behavior

This design does not cover:

- backend route changes
- metadata JSON schema changes
- generic docsify internal-link rewriting outside the metadata sidebar
- editing note properties in the browser
- backlinks
- replacing docsify's own outline generation with a markbase-owned TOC parser

## Design Goals

- Keep note body reading as the primary activity.
- Preserve docsify's familiar left-sidebar navigation model.
- Make important note metadata visible without polluting the note Markdown.
- Preserve clickable wiki-links inside frontmatter-derived values.
- Keep the sidebar readable on dense knowledge-management notes with many
  fields.
- Degrade cleanly on small screens and on metadata fetch failure.

## Non-Goals

- The first version does not aim to mimic Obsidian's exact property panel UI.
- The first version does not render every schema hint as a visible control.
- The first version does not support inline property editing.
- The first version does not require animations beyond simple state changes.
- The first version does not replace docsify's mobile drawer behavior with a
  separate markbase-specific drawer.

## Core Decision

The docsify shell should use the existing docsify left sidebar as the only
sidebar in the page.

That sidebar should be organized as one tab strip plus one active panel:

- `Outline`
- `Properties`
- `Links`

`Outline` is the default tab and presents docsify's own navigation and
heading-outline content. `Properties` and `Links` are markbase-owned metadata
panels that only become available for canonical Markdown note routes supported
by `design-013`.

This is a unified-shell decision:

- the page should no longer render a separate right metadata sidebar
- note metadata should not compete with docsify outline as two unrelated
  sidebars
- metadata should feel like part of the docsify reading rail rather than a
  second page chrome system with its own palette and placement rules

## Eligible Routes

The `Outline` tab is part of the unified sidebar shell and may appear on any
docsify document route where docsify itself renders sidebar content.

Metadata tabs are defined only for canonical Markdown note routes.

Rules:

- a docsify route whose pathname ends in `.md` is eligible for `Properties`
  and `Links`
- a docsify route whose pathname ends in `.base` is not eligible for metadata
  requests and should expose only `Outline`
- non-document shell routes such as `/` are not eligible for metadata requests
  and should expose only `Outline`
- docsify query parameters used only for in-page navigation, such as `?id=...`,
  do not change route eligibility or document identity

Unsupported routes are not an error state. They simply do not surface metadata
tabs.

## Layout

### Desktop

Desktop should keep docsify's existing left-rail structure:

- docsify left sidebar remains the only sidebar
- note body remains in the main content area
- markbase tabs live inside `.sidebar`

Recommended behavior:

- the docsify app name/header stays outside the tab panels
- the tab strip sits directly below the docsify app name area
- the active panel uses the sidebar's existing visual language
- the sidebar keeps one coherent background, border, typography, and hover
  system rather than mixing a docsify rail with a card-styled markbase widget
- `Outline`, `Properties`, and `Links` share one sidebar slot rather than
  appearing as separate stacked regions

The first version should not render metadata as a floating overlay or a second
column.

### Mobile

On mobile or narrow viewports, the unified sidebar should continue to follow
docsify's existing mobile/sidebar behavior.

Implications:

- markbase should not move metadata below the note body on narrow screens
- tabs live inside the same docsify sidebar or drawer that already serves
  navigation
- the first version does not require a second mobile toggle button

## Tab Structure

The sidebar contains a stable tab strip plus one active panel.

In v1 the tabs are:

1. `Outline`
2. `Properties`
3. `Links`

Rules:

- only one panel is visible at a time
- `Outline` is the default tab
- tab order is stable: `Outline`, then `Properties`, then `Links`, then any
  future tabs such as `Backlinks`
- on non-eligible routes, only `Outline` is shown
- a tab whose requested data is empty still renders an explicit empty state
  when selected, rather than disappearing silently

This is a structural decision, not just styling. Docsify outline and markbase
metadata should share one sidebar framework without being flattened into the
same `ul/li` tree.

## `Outline` Tab

### Ownership

`Outline` is docsify-owned content presented inside the unified tab framework.

Markbase should not replace docsify's TOC generation logic with a second
outline implementation. Instead, docsify should continue generating its own
sidebar navigation and in-page outline, and markbase should treat that content
as the body of the `Outline` tab.

### Integration Rules

- docsify must be allowed to generate its normal sidebar DOM first
- markbase may wrap or relocate that generated DOM into the `Outline` panel
  after docsify render hooks run
- markbase must preserve docsify-generated link targets, nesting, and DOM
  semantics inside the outline content
- markbase must continue applying same-note `?id=...` interception so outline
  clicks remain in-page navigation instead of backend requests

This means `Outline` is a presentation container around docsify output, not a
markbase-authored parallel TOC.

## `Properties` Tab

### Overall Structure

`Properties` should render as a vertical list of key/value entries.

Each entry contains:

- property key
- property value
- optional schema hint presentation

The visual default should be compact and scan-friendly rather than
document-like.

`Properties` lives inside the unified sidebar panel and should adopt the
docsify sidebar palette instead of a detached card treatment.

### Property Keys

Property keys should:

- be visually distinct from values
- favor readability over raw YAML aesthetics
- preserve the original key text from the note

The first version should not rename keys based on schema labels. If schema
labels are later introduced visually, the raw key should remain discoverable.

### Property Values

Property values are rendered from the semantic node model defined by
`design-013`.

Rendering rules:

- `null`: render a subdued placeholder such as `null` or an empty-value marker
- `scalar`: render as plain inline text
- `rich_text`: render inline segments in order
- `list`: render as a vertical mini-list or wrapped token list, depending on
  content density
- `object`: render as a nested key/value block within the property row

The frontend must not reparse frontmatter strings as Markdown.

### `rich_text` Segments

For `rich_text`:

- `text` segments render as literal text
- resolved `wikilink` segments render as clickable internal links
- unresolved `wikilink` segments render as visibly unresolved text without a
  fabricated destination

The visual treatment of unresolved links should be clearly different from
resolved links, for example through color, underline style, or an unresolved
badge.

### Schema Hints

Schema metadata is secondary information. It should not dominate the row.

The first version may optionally render lightweight hints such as:

- `required`
- field `type`
- `format: link`

Recommended treatment:

- small badges or subdued annotations near the key
- field description only on hover, expand, or secondary detail view

The first version should avoid turning schema descriptions into long always-on
paragraphs in the sidebar.

## `Links` Tab

### Overall Structure

`Links` should render as a simple list of outgoing references.

Each row may include:

- a label derived from the backend `target` field
- optional kind hint such as `note`, `base`, or `resource`
- resolved/unresolved state

The first version should optimize for quick scanning, not graph exploration.

The first version should not assume alias text, source-location metadata, or
frontmatter-vs-body attribution because `design-013` does not expose those
details in the `links` field.

### Link Behavior

Rules:

- resolved note and base links navigate inside the docsify shell
- resolved resource links follow their direct resource URL
- unresolved links render as non-clickable text

For note/base targets, the frontend must not use the backend canonical href as
a direct browser destination. The backend may expose canonical paths such as
`/entities/company/acme.md`, but inside the docsify sidebar these must be
adapted into docsify shell navigation targets so clicking opens the note in the
docsify app rather than showing raw Markdown.

Equivalent examples:

- backend canonical href: `/entities/company/acme.md`
- docsify sidebar destination: `#/entities/company/acme.md`

If later versions add source attribution such as `body` or `frontmatter`, that
should be treated as secondary row metadata rather than a separate primary
grouping in the first version.

## States

### Loading

While metadata is loading for the current note:

- keep the main note body visible
- keep `Outline` available
- render lightweight loading placeholders when users switch into a metadata tab

The first version should avoid blocking the note page or the docsify outline on
sidebar data.

### Empty

Empty states must be explicit when a metadata tab is active for an eligible
note route.

Examples:

- no frontmatter properties: show `No properties`
- no outgoing links: show `No links`

### Error

If the sidebar metadata request fails:

- the main note body remains visible
- `Outline` remains usable
- the metadata tab shows a compact error state
- a later route change retries automatically

The first version may omit manual retry UI if automatic retry on navigation is
already present.

## Route Change Behavior

On docsify route changes:

1. derive the logical document identity from the docsify route pathname, not
   from docsify's section-anchor query parameters
2. keep the unified sidebar shell mounted inside docsify's existing `.sidebar`
3. if the pathname is not an eligible `.md` note route, clear metadata state,
   suppress metadata requests, and expose only `Outline`
4. if only the docsify `?id=...` anchor changes while the normalized note
   pathname stays the same, treat it as in-page navigation and keep current
   sidebar state
5. for a newly active eligible note route, build the metadata request from the
   canonical note pathname plus `?fields=properties,links`, without forwarding
   docsify-only query parameters such as `id`
6. adapt resolved note/base links in sidebar content to docsify shell routes
   before attaching them to clickable UI
7. re-sync the docsify-generated outline DOM into the `Outline` tab after docsify
   updates the sidebar for the new route
8. replace metadata contents only with the response for the latest active route

The implementation should guard against stale-response overwrite when users
navigate quickly.

## Visual Direction

The sidebar should feel like one coherent docsify rail, not like a docsify rail
plus a second embedded app.

Recommended visual principles:

- follow docsify sidebar colors for background, text, separators, and hover
- use restrained density
- keep tab chrome simple and rail-like rather than card-like
- maintain enough contrast to make clickable values obvious
- keep consistent spacing between rows

The first version should prefer docsify-native visual alignment over decorative
cards, heavy shadows, or a competing color system.

## Accessibility

The docsify sidebar UI should:

- preserve keyboard access to tabs and links
- keep section headings semantic
- not rely on color alone to distinguish unresolved links
- remain readable when properties contain long values or many list items
- preserve outline navigation as a first-class keyboard-accessible surface

## Compatibility Notes

- This design depends on the semantic property/value model from `design-013`.
- This design intentionally avoids metadata requests for `.base` and other
  unsupported routes because `design-013` defines metadata mode only on
  canonical Markdown note routes.
- This design extends the docsify integration surface defined in `design-012`
  without changing the backend Markdown contract from `design-003`.
