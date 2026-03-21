---
id: design-013
title: "Web Note Metadata Sidebar"
status: draft
module: web-frontend
---

# Web Note Metadata Sidebar

## Purpose

This document defines how markbase should expose note metadata for browser-side
sidebar presentation, starting with:

- frontmatter properties
- outgoing links

The initial frontend consumer is the generated docsify shell, but the backend
contract is intentionally defined as a route-level JSON surface rather than as
a docsify-only UI primitive.

## Problem Statement

`design-003` and `design-012` establish the current browser delivery model:

- markbase serves translated Markdown and resource bytes
- docsify is an optional frontend shell
- note rendering remains server-side

That is enough to read note bodies, but it does not expose important structured
metadata that users rely on while browsing a vault, especially:

- current note frontmatter
- current note outgoing links
- clickable wiki-links that appear inside frontmatter string values

Rendering this metadata by stuffing extra Markdown into the note body would
blur the boundary between note content and presentation-only chrome. The web
surface therefore needs a separate metadata contract.

## Scope

This design covers:

- a metadata mode on canonical Markdown note routes
- the initial `fields` query contract
- the JSON shape for `properties` and `links`
- template-aware property enrichment
- docsify sidebar fetching and refresh behavior

This design does not cover:

- backlinks in the first release
- changing the existing Markdown response for note routes without query
  parameters
- making the backend decide docsify-specific UI layout
- embedding property blocks directly into rendered note Markdown
- adding schema-driven write/edit behavior in the browser

## Design Goals

- Preserve the current canonical note route as the only route identity.
- Keep Markdown note content and sidebar metadata as separate response modes.
- Make frontmatter wiki-links clickable without requiring frontend-side
  reimplementation of Obsidian parsing.
- Reuse existing markbase semantics for note resolution, canonical href
  generation, and template interpretation.
- Return metadata as semantic data, not pre-rendered docsify UI.

## Non-Goals

- The backend does not choose whether a property renders as a table row, chip,
  card, or tree node in docsify.
- The first version does not add `backlinks`.
- The first version does not promise metadata mode for `.base` routes.
- The first version does not expose every possible derived note statistic.

## Core Decision

The canonical Markdown note route remains the primary web identity:

```text
/<file.path>
```

Without supported query parameters, the route keeps its current behavior and
returns translated Markdown.

When the route includes a supported `fields` query parameter, the same
canonical note route returns note metadata as JSON.

Examples:

```text
/entities/person/alice.md
/entities/person/alice.md?fields=properties
/entities/person/alice.md?fields=properties,links
```

This keeps web identity path-based and stable while avoiding a second `/api`
route system for note metadata.

## Route Contract

### Supported Targets

Metadata mode is defined only for canonical Markdown note routes in the first
version.

In v1:

- `/<file.path>.md` without `fields` returns translated Markdown
- `/<file.path>.md?fields=...` returns JSON metadata
- `/<file.path>.base?fields=...` is not supported
- binary resource routes do not support `fields`

If a request uses `fields` on an unsupported route kind, the server returns
`400 Bad Request`.

### Query Parameters

The metadata switch is the presence of the `fields` query parameter.

Supported shape:

```text
?fields=<field>[,<field>...]
```

Initial supported fields:

- `properties`
- `links`

Rules:

- field names are case-sensitive
- duplicate field names are ignored after the first occurrence
- unknown field names return `400 Bad Request`
- unsupported query parameters return `400 Bad Request`

Examples:

```text
/entities/person/alice.md?fields=properties
/entities/person/alice.md?fields=properties,links
```

### Response Mode

Response mode is determined by the presence of supported query parameters:

- no supported query parameters: `text/markdown; charset=utf-8`
- `fields=...`: `application/json; charset=utf-8`

## Metadata Response Shape

The response is semantic data for frontend rendering. It is not a pre-rendered
sidebar fragment.

The response envelope is:

```json
{
  "file": {
    "path": "entities/person/alice.md",
    "name": "alice",
    "folder": "entities/person",
    "templates": ["person"]
  },
  "properties": {
    "fields": []
  },
  "links": []
}
```

Rules:

- `file` is always present in metadata mode
- only requested top-level fields are returned beyond `file`
- omitted fields are absent rather than present as `null`

## `properties` Field

### Purpose

`properties` exposes the current note frontmatter as an ordered, template-aware
property list suitable for sidebar presentation.

The backend owns:

- raw frontmatter extraction
- recursive value classification
- wiki-link parsing inside frontmatter string values
- note resolution and canonical href generation
- template/schema association

The frontend owns:

- visual layout
- grouping, folding, badges, and typography
- empty-state and error-state presentation

### Top-Level Shape

`properties` returns:

```json
{
  "fields": [
    {
      "key": "manager",
      "raw": "[[Bob|负责人]]",
      "value": {
        "kind": "rich_text",
        "segments": [
          {
            "type": "wikilink",
            "target": "Bob",
            "text": "负责人",
            "href": "/entities/person/bob.md",
            "exists": true
          }
        ]
      },
      "schema": {
        "template": "person",
        "required": false,
        "type": "text",
        "format": "link",
        "target": "person",
        "description": "Direct manager"
      }
    }
  ]
}
```

### Ordered Field List

`properties.fields` is an ordered array rather than an object map.

Reasons:

- sidebar presentation needs stable order
- the response should not rely on JSON object key ordering
- future implementations may choose ordering rules that combine raw note
  properties and template-defined schema order

The v1 ordering rule is:

1. emit note fields that actually exist on the current note
2. preserve the parsed frontmatter field order when available
3. if exact source order is unavailable in implementation, fall back to a
   stable deterministic order and document that limitation

### Value Model

Each property field contains a `value` node. The node is semantic data, not a
UI instruction.

Supported node kinds in v1:

- `null`
- `scalar`
- `rich_text`
- `list`
- `object`

#### `null`

```json
{ "kind": "null" }
```

#### `scalar`

For non-string scalar JSON values such as numbers, booleans, and date-like
strings that the backend leaves as plain scalar content:

```json
{ "kind": "scalar", "value": 3 }
```

#### `rich_text`

Used for string values so the backend can annotate frontmatter wiki-links while
leaving ordinary text intact.

```json
{
  "kind": "rich_text",
  "segments": [
    { "type": "text", "text": "Owner: " },
    {
      "type": "wikilink",
      "target": "Alice",
      "text": "Alice",
      "href": "/entities/person/alice.md",
      "exists": true
    }
  ]
}
```

Segment types:

- `text`
- `wikilink`

For `wikilink` segments:

- `target` is the normalized logical target name
- `text` is alias text when present, otherwise the default display text
- `href` is the canonical path-based browser route when resolution succeeds
- `exists` reports whether the target resolved

Unresolved frontmatter wiki-links remain semantic segments with `exists: false`.
They do not invent a fallback href from raw note name text alone.

#### `list`

```json
{
  "kind": "list",
  "items": [
    { "kind": "scalar", "value": "ai" },
    {
      "kind": "rich_text",
      "segments": [
        {
          "type": "wikilink",
          "target": "Project X",
          "text": "Project X",
          "href": "/projects/project-x.md",
          "exists": true
        }
      ]
    }
  ]
}
```

#### `object`

```json
{
  "kind": "object",
  "fields": [
    {
      "key": "reviewer",
      "value": {
        "kind": "rich_text",
        "segments": [
          {
            "type": "wikilink",
            "target": "Bob",
            "text": "Bob",
            "href": "/entities/person/bob.md",
            "exists": true
          }
        ]
      }
    }
  ]
}
```

### Link Parsing In Frontmatter

Frontmatter string handling must reuse the shared link parser from
`src/link_syntax.rs` in `FrontmatterString` mode.

This rule exists so:

- frontmatter wiki-link interpretation stays aligned with extraction semantics
- the docsify frontend does not need its own Obsidian parser
- link normalization remains consistent with rename, verify, and web route
  generation

This design does not require frontmatter strings to become Markdown. Only the
embedded wiki-link spans become annotated `wikilink` segments.

### Template-Aware Enrichment

When a note is template-backed, `properties` should enrich fields with schema
metadata derived from the referenced templates.

Relevant template behavior continues to follow `design-006`.

The property-level `schema` object may include:

- `template`
- `required`
- `type`
- `format`
- `target`
- `enum`
- `description`

The schema object is advisory metadata for presentation and inspection. It does
not turn the web surface into a verification or editing endpoint.

If multiple templates contribute conflicting definitions for the same field,
the server should return a deterministic merged result or an explicit conflict
annotation in a future revision. The first version may choose a simpler
deterministic precedence rule, but that rule must be documented in the
implementation.

## `links` Field

### Purpose

`links` exposes the current note's outgoing references as resolved web targets
for sidebar presentation.

This field is intentionally narrower than a full graph API.

### Shape

```json
[
  {
    "target": "Bob",
    "href": "/entities/person/bob.md",
    "kind": "note",
    "exists": true
  },
  {
    "target": "diagram.png",
    "href": "/assets/diagram.png",
    "kind": "resource",
    "exists": true
  }
]
```

Rules:

- `target` is the normalized link target stored by markbase
- `href` is the canonical browser path when resolution succeeds
- `kind` is one of `note`, `base`, or `resource`
- `exists` reports whether the target resolved in the current index

The first version may return a deduplicated target list rather than
source-location-aware link instances. If future UI needs distinguish body links
from frontmatter links or embeds, that can extend this field without changing
the route contract.

## Docsify Integration

### Sidebar Data Flow

The generated docsify shell should keep the current note body rendering flow and
add a sidebar fetch alongside it:

1. docsify navigates to a canonical note route inside the shell
2. the main content continues to fetch Markdown from `/<file.path>.md`
3. the sidebar requests the same canonical route with
   `?fields=properties,links`
4. the sidebar renders returned metadata in docsify-managed DOM

Example:

```text
/entities/person/alice.md
/entities/person/alice.md?fields=properties,links
```

### Presentation Boundary

The docsify plugin may choose any appropriate sidebar layout, but it must treat
the metadata response as semantic input rather than raw Markdown to be reparsed.

In particular:

- `rich_text` segments should be rendered directly
- resolved `wikilink` segments should be clickable
- unresolved `wikilink` segments should remain visibly unresolved
- schema metadata may be used for labels, badges, grouping, or tooltips

### Failure Behavior

Sidebar metadata failure must not block the main note body from rendering.

Recommended behavior:

- keep the note body visible
- render an empty or warning state in the sidebar
- allow later route changes to retry the metadata request

## Error Handling

Metadata mode returns:

- `400 Bad Request` for unknown fields or unsupported query parameters
- `400 Bad Request` when `fields` is used on unsupported route kinds
- `404 Not Found` when the canonical note route does not resolve
- `500 Internal Server Error` for server-side failures

## Compatibility Notes

- This design preserves the current `design-003` Markdown route contract when
  no `fields` parameter is present.
- This design extends `design-012` by giving the docsify shell a supported
  metadata fetch surface for sidebar rendering.
- This design reuses `design-006` template semantics instead of inventing a new
  browser-only schema model.
