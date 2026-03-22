---
id: design-013
title: "Web Note Metadata Mode"
status: implemented
module: web
---

# Web Note Metadata Mode

## Purpose

This document defines the backend contract for exposing structured note
metadata on canonical web note routes.

The goal is to let browser-side consumers inspect note metadata without
changing the existing Markdown body contract and without introducing a second
`/api` route family.

The first implemented metadata fields are:

- frontmatter `properties`
- outgoing `links`

## Relationship To Other Designs

- `design-003` defines the broader web note delivery contract
- `design-006` defines template semantics reused for schema enrichment
- `design-012` defines docsify shell integration at the frontend boundary
- `design-014` defines docsify sidebar layout, fetch lifecycle, and rendering
  behavior for consumers of this metadata contract

This design is intentionally backend-only. It defines route semantics, response
shape, and semantic value nodes. It does not define docsify layout or browser
state handling.

## Scope

This design covers:

- metadata mode on canonical Markdown note routes
- the `fields` query contract
- the JSON response shape for `file`, `properties`, and `links`
- frontmatter wiki-link semantic annotation
- template-aware property schema enrichment
- deterministic precedence for multi-template schema conflicts

This design does not cover:

- docsify sidebar layout or styling
- metadata loading, empty, or error states in the browser
- route-change refresh behavior in the frontend shell
- backlinks in the first version
- changing the Markdown response for note routes without query parameters
- browser-side property editing

## Design Goals

- Preserve the canonical note route as the only web identity.
- Keep Markdown content and metadata as separate response modes on the same
  route.
- Reuse existing markbase semantics for note resolution, canonical href
  generation, link parsing, and template interpretation.
- Return semantic data for frontend rendering instead of pre-rendered sidebar
  HTML.

## Core Decision

The canonical Markdown note route remains:

```text
/<file.path>
```

Without supported query parameters, the route keeps its existing behavior and
returns translated Markdown.

When a request includes a supported `fields` query parameter, the same
canonical Markdown note route returns metadata as JSON.

Examples:

```text
/entities/person/alice.md
/entities/person/alice.md?fields=properties
/entities/person/alice.md?fields=properties,links
```

This preserves path-based web identity while avoiding a separate route tree for
metadata delivery.

## Route Contract

### Supported Targets

Metadata mode is defined only for canonical Markdown note routes.

In the implemented version:

- `/<file.path>.md` without `fields` returns translated Markdown
- `/<file.path>.md?fields=...` returns JSON metadata
- `/<file.path>.base?fields=...` returns `400 Bad Request`
- binary resource routes with `fields` return `400 Bad Request`

### Query Parameters

The metadata switch is the presence of the `fields` query parameter.

Supported shape:

```text
?fields=<field>[,<field>...]
```

Supported fields:

- `properties`
- `links`

Rules:

- field names are case-sensitive
- duplicate field names are ignored after the first occurrence
- unknown field names return `400 Bad Request`
- unsupported query parameters return `400 Bad Request`
- malformed `fields` syntax returns `400 Bad Request`

### Response Mode

Response mode is determined by the query string:

- no supported query parameters: `text/markdown; charset=utf-8`
- `fields=...`: `application/json; charset=utf-8`

## Response Envelope

Metadata mode returns semantic JSON rather than a rendered sidebar fragment.

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

## `properties`

### Purpose

`properties` exposes the current note frontmatter as an ordered, template-aware
property list.

The backend owns:

- raw frontmatter extraction
- recursive value classification
- wiki-link parsing inside frontmatter string values
- note resolution and canonical href generation
- template/schema association

The frontend owns presentation decisions such as layout, badges, grouping, and
error states. Those browser concerns are defined by `design-014`.

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

The implemented ordering rule is:

1. emit note fields that actually exist on the current note
2. preserve parsed top-level frontmatter order when available
3. append remaining fields in stable sorted order

### Value Model

Each property field contains a semantic `value` node.

Supported node kinds:

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

```json
{ "kind": "scalar", "value": 3 }
```

#### `rich_text`

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

Unresolved frontmatter wiki-links remain semantic segments with `exists: false`
and no fabricated fallback href.

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

### Frontmatter Link Parsing

Frontmatter string handling reuses the shared link parser from
`src/link_syntax.rs` in `FrontmatterString` mode.

This keeps frontmatter wiki-link interpretation aligned with extraction,
rename, verify, and canonical web route generation.

### Template-Aware Enrichment

When a note is template-backed, `properties` enriches fields with schema
metadata derived from the referenced templates.

The property-level `schema` object may include:

- `template`
- `required`
- `type`
- `format`
- `target`
- `enum`
- `description`

This schema object is advisory metadata for inspection and presentation. It
does not turn the web surface into an edit or verify endpoint.

### Multi-Template Precedence

If multiple templates contribute definitions for the same property, the
implemented precedence rule is deterministic and simple:

1. parse note template names from the note `templates` frontmatter in order
2. iterate templates in that order
3. the first template that contributes a definition for a given field wins
4. later conflicting definitions for that field are ignored

No implicit merge or conflict annotation is added in the implemented version.

## `links`

### Purpose

`links` exposes the current note's outgoing references as resolved web targets.

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

The implemented version returns a deduplicated target list rather than
source-location-aware link instances.

## Error Handling

Metadata mode returns:

- `400 Bad Request` for unknown fields or unsupported query parameters
- `400 Bad Request` when `fields` is used on unsupported route kinds
- `404 Not Found` when the canonical note route does not resolve
- `500 Internal Server Error` for server-side failures

## Compatibility Notes

- This design preserves the `design-003` Markdown route contract when no
  `fields` parameter is present.
- This design reuses `design-006` template semantics rather than inventing a
  browser-only schema model.
- This design intentionally stops at a semantic metadata surface. Docsify
  layout, fetch timing, stale-response protection, and sidebar state handling
  belong to `design-014`.
