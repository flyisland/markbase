---
id: design-003-patch-01-relative-web-links-for-docsify
title: "Web Note View Patch 01: Relative Web Links For Docsify"
status: draft
parent: design-003
module: web
---

# Web Note View Patch 01

## Relative Web Links For Docsify

**Status:** Draft  
**Parent:** [`docs/design-docs/implemented/design-003-web-note-view.md`](/Users/ichen/devProjects/island/markbase/docs/design-docs/implemented/design-003-web-note-view.md)  
**Target:** markbase web output contract  
**Type:** Design Patch

## Purpose

This patch records a design correction for markbase web output.

`design-003` currently defines server-emitted note and resource links as
canonical absolute browser URLs derived from `/<file.path>`. That shape is
stable as an HTTP route identity, but it is not the best href contract for a
docsify-based frontend that consumes Markdown as in-app navigation content.

The goal of this patch is to separate:

- request-path identity for the markbase web server
- emitted Markdown href shape inside response bodies

## Problem

The current web implementation rewrites live note links to absolute hrefs such
as:

```md
[alice](/entities/person/alice.md)
```

That output is valid as a raw browser URL, but it has an undesirable property
for docsify:

- clicking the link navigates the browser to the absolute path directly
- this leaves the current docsify HTML shell instead of staying inside docsify
  route handling

For a docsify consumer, the more natural Markdown contract is a path that is
resolved relative to the current rendered document location.

## Root Cause

The current design conflates two different things:

1. the canonical lookup key used by markbase to resolve incoming requests
2. the href shape that should be emitted inside Markdown returned to a frontend

These do not need to be identical.

Markbase can still resolve requests by decoded vault-relative `file.path` while
emitting relative hrefs in Markdown response bodies.

## Design Correction

### Request identity stays path-based

Incoming HTTP requests should continue to resolve by decoded vault-relative
`file.path`.

Examples:

```text
/entities/person/alice.md
/All%20Opputunities%20Logs.base
/assets/image.png
```

This remains the stable route-resolution contract for the server.

### Emitted Markdown links become response-relative

When markbase rewrites live note links or resource embeds inside a Markdown
response body, the emitted href should be relative to the top-level response
route directory, not emitted as an absolute `/<file.path>` URL.

Examples for a response served from `/logs/daily/today.md`:

- target note `entities/person/alice.md` emits `../../entities/person/alice.md`
- target image `assets/image.png` emits `../../assets/image.png`
- target sibling note `logs/daily/yesterday.md` emits `yesterday.md`

This matches ordinary Markdown file-link expectations and is friendlier to
docsify and similar Markdown-first frontends.

## Important Constraint

The relative-link base is **the top-level response path**, not the embedded
note path that may have introduced the link during recursive render.

This matters because browsers resolve links relative to the URL of the current
HTTP response, not relative to the origin file of an embedded note body.

Therefore:

- a response rendered from `/folder/host.md` must emit links relative to
  `/folder/`
- links originating from recursively embedded notes or `.base` sections inside
  that response must still be rewritten relative to `/folder/`
- markbase must not emit mixed bases inside one response body

## Contract Changes

If this patch is adopted, the `design-003` web contract should change as
follows.

### Note link rewrite

Current shape:

```md
[alice](/entities/person/alice.md)
```

New shape when emitted from `/logs/daily/today.md`:

```md
[alice](../../entities/person/alice.md)
```

### Resource embed rewrite

Current shape:

```md
![](/assets/image.png)
```

New shape when emitted from `/logs/daily/today.md`:

```md
![](../../assets/image.png)
```

### `.base` output links

Links emitted from `.base` query results in web mode must follow the same
response-relative rule.

This includes direct `.base` routes and `.base` sections rendered inside note
responses.

## Non-Goals

This patch does not propose:

- changing request matching away from `file.path`
- introducing docsify hash-route syntax such as `#/foo/bar.md` into markbase
  output
- serving a docsify HTML shell from markbase
- changing internal note identity away from name-based resolution

## Why Not Emit Bare `file.path`

It is not sufficient to simply emit `file.path` verbatim.

Example:

- current response: `/logs/daily/today.md`
- target path: `entities/person/alice.md`

If markbase emits:

```md
[alice](entities/person/alice.md)
```

that link is interpreted relative to `/logs/daily/`, producing the wrong
effective target.

The correct href must be computed as a relative path from the current response
directory to the target `file.path`.

## Migration Impact

Adopting this patch would require updates to:

- `src/web/mod.rs` link and resource rewrite logic
- web-mode `.base` link emission
- `tests/cli_web.rs` assertions that currently expect leading `/`
- `README.md`
- `ARCHITECTURE.md`
- `docs/design-docs/implemented/design-003-web-note-view.md`

## Open Questions

1. Should markbase expose an optional frontend-specific output mode in the
   future, or should relative links become the default web contract outright?
2. Should direct HTTP users still have a documented canonical absolute path
   form for copying and debugging, even if emitted Markdown no longer uses that
   shape in links?
3. Should the same response-relative rule also become the contract for any
   future HTML frontend other than docsify?
