---
id: design-014
title: "Docsify Note Sidebar UI"
status: draft
module: web-frontend
---

# Docsify Note Sidebar UI

## Purpose

This document defines the docsify-side presentation contract for the note
metadata sidebar powered by the backend metadata mode defined in `design-013`.

It answers a frontend question, not a backend one:

- how note metadata should be laid out in the docsify shell
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

- desktop and mobile sidebar layout
- sidebar section structure for `Properties` and `Links`
- rendering rules for `properties.fields[].value`
- loading, empty, and error states
- docsify route-change refresh behavior

This design does not cover:

- backend route changes
- metadata JSON schema changes
- editing note properties in the browser
- backlinks
- full-site navigation redesign beyond the note metadata sidebar

## Design Goals

- Keep note body reading as the primary activity.
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

## Core Decision

The docsify shell should render a two-region note page on desktop:

- main content column for note Markdown
- right sidebar for metadata

On narrow screens, the sidebar should move below the note content rather than
compete for horizontal space.

This preserves reading flow while keeping metadata available as persistent page
chrome on larger screens.

## Layout

### Desktop

Desktop note pages should use a two-column layout:

- main column: rendered note content
- sidebar column: metadata sections

Recommended behavior:

- main column remains visually dominant
- sidebar width stays fixed or capped within a narrow readable range
- sidebar remains independently scrollable only if needed; otherwise the page
  should prefer normal document scroll

The first version should avoid a floating overlay or collapsible drawer on
desktop. The metadata is important enough to deserve stable placement.

### Mobile

On mobile or narrow viewports, the sidebar should stack below the note body.

Reasons:

- preserving note readability matters more than persistent side placement
- docsify already has limited horizontal space on small screens
- stacking avoids building an early drawer system before the information
  architecture stabilizes

The first version does not require a separate mobile toggle button.

## Section Structure

The sidebar contains two top-level sections in v1:

1. `Properties`
2. `Links`

Rules:

- section titles are always visible
- sections render in the order above
- a missing requested section renders an explicit empty state, not a silent gap

## `Properties` Section

### Overall Structure

`Properties` should render as a vertical list of key/value entries.

Each entry contains:

- property key
- property value
- optional schema hint presentation

The visual default should be compact and scan-friendly rather than
document-like.

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

## `Links` Section

### Overall Structure

`Links` should render as a simple list of outgoing references.

Each row may include:

- target display text
- optional kind hint such as `note`, `base`, or `resource`
- resolved/unresolved state

The first version should optimize for quick scanning, not graph exploration.

### Link Behavior

Rules:

- resolved note and base links navigate inside the docsify shell
- resolved resource links follow their direct resource URL
- unresolved links render as non-clickable text

If later versions add source attribution such as `body` or `frontmatter`, that
should be treated as secondary row metadata rather than a separate primary
grouping in the first version.

## States

### Loading

While metadata is loading for the current note:

- keep the main note body visible
- render lightweight loading placeholders in the sidebar

The first version should avoid blocking the note page on sidebar data.

### Empty

Empty states must be explicit.

Examples:

- no frontmatter properties: show `No properties`
- no outgoing links: show `No links`

### Error

If the sidebar metadata request fails:

- the main note body remains visible
- the sidebar shows a compact error state
- a later route change retries automatically

The first version may omit manual retry UI if automatic retry on navigation is
already present.

## Route Change Behavior

On docsify route changes to a canonical note page:

1. keep the previous sidebar visible briefly or replace it with a loading state
2. request `?fields=properties,links` for the new note route
3. replace sidebar contents only with the response for the latest active route

The implementation should guard against stale-response overwrite when users
navigate quickly.

## Visual Direction

The sidebar should feel like stable reading chrome, not like a debug panel.

Recommended visual principles:

- restrained density
- strong separation between section headers and rows
- low-noise typography
- enough contrast to make clickable values obvious
- consistent spacing between rows

The first version should prefer a clean neutral presentation over decorative
cards or heavy borders.

## Accessibility

The docsify sidebar UI should:

- preserve keyboard access to links
- keep section headings semantic
- not rely on color alone to distinguish unresolved links
- remain readable when properties contain long values or many list items

## Compatibility Notes

- This design depends on the semantic property/value model from `design-013`.
- This design extends the docsify integration surface defined in `design-012`
  without changing the backend Markdown contract from `design-003`.
