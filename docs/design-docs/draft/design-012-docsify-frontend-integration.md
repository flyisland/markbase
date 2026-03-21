---
id: design-012
title: "Docsify Frontend Integration"
status: draft
module: web-frontend
---

# Docsify Frontend Integration

**Status:** Draft  
**Target:** markbase web frontend integration

## Purpose

This document defines how markbase should support docsify as an explicit,
optional frontend for browsing markbase web output in a browser.

The goal is not to turn markbase into a full HTML application server. The goal
is to give users a supported way to install a lightweight docsify shell that
works correctly with markbase's web output and can evolve to cover
presentation-layer concerns such as callout styling.

## Problem Statement

`design-003` established markbase's first web delivery contract:

- `markbase web serve` returns Markdown for `.md` and `.base` routes
- binary resources are returned as raw bytes
- markbase does not own a docsify shell or HTML entrypoint in v1

That contract is sufficient for raw content serving, but users still need a
frontend shell to browse the content comfortably.

Ad hoc manual placement of an `index.html` in the vault root has three
problems:

1. it pollutes the user's base-dir with frontend-specific files
2. it makes setup implicit rather than explicit
3. it leaves docsify-specific behavior such as internal-link adaptation outside
   any supported markbase contract

This design therefore treats docsify integration as a first-class frontend
installation problem rather than as an accidental side effect of serving random
HTML from the vault.

## Scope

This design covers:

- an explicit `markbase web init-docsify` command
- where docsify shell files should be generated
- how the generated shell should consume markbase web output
- how internal link handling should work for docsify
- which presentation concerns belong to docsify rather than to the markbase
  backend

This design does not cover:

- replacing `design-003` backend routing or render semantics
- making docsify mandatory for web output
- implementing every future frontend plugin immediately

## Design Goals

- Keep docsify installation explicit and user-triggered.
- Preserve the current markbase backend boundary: Markdown and resource bytes
  remain the primary HTTP contract.
- Make the generated docsify shell work with current markbase web output,
  including absolute internal links emitted by the backend.
- Establish a supported home for future frontend-only concerns such as callout
  presentation, Mermaid rendering, sidebar behavior, and search.

## Non-Goals

- Markbase does not become a full docsify hosting framework.
- Markbase does not silently create or overwrite frontend files during ordinary
  indexing, querying, note creation, or serving.
- This design does not change the backend Markdown link contract defined by
  `design-003`.
- This design does not require callout rendering, Mermaid, sidebar generation,
  or search to ship in the first implementation.

## Core Decision

Markbase should provide an explicit initialization command:

```bash
markbase web init-docsify
```

This command generates a minimal docsify shell into the current base-dir so the
shell is directly reachable as `/index.html` from the existing `web serve`
root.

The shell is optional. Users who do not want docsify should not pay any cost or
see any generated files outside explicit initialization.

## Chosen Direction

The chosen direction for docsify integration is:

- keep the current `design-003` backend route and href contract unchanged
- treat docsify compatibility as a frontend integration responsibility
- solve docsify navigation with a generated frontend plugin rather than a
  backend link-shape rewrite
- generate the shell at the base-dir root as `index.html` rather than under a
  markbase-owned subdirectory

This means the current absolute backend href shape, such as:

```md
[alice](/entities/person/alice.md)
```

remains valid markbase output for now.

`design-012` is therefore not a backend URL redesign document. It is a frontend
integration document built on top of the existing backend contract.

## Generated Location

The default output location should be the base-dir root:

```text
<base-dir>/index.html
```

The first implementation should generate at least:

```text
index.html
```

Reasons:

- users already expect a browsable entrypoint at the web root
- `http://127.0.0.1:3000/index.html` is simpler than a tool-owned nested path
- docsify shell installation should optimize for direct use, not hidden storage
- overwrite risk is still controlled by explicit command invocation plus
  `--force`

## Command Contract

The initial command surface should be small:

```bash
markbase web init-docsify --homepage <canonical-url>
markbase web init-docsify --force
```

### Behavior

- create the target directory if needed
- write the docsify shell file at `<base-dir>/index.html`
- refuse to overwrite an existing `index.html` unless `--force` is provided
- require users to specify an initial homepage route via `--homepage`

### Homepage Input

`--homepage` should accept the same canonical route shape that `markbase web
get` and `markbase web serve` already understand, for example:

```text
/All%20Opputunities%20Logs.base
/entities/person/alice.md
```

In the first implementation, `--homepage` is required.

The command must not guess a homepage from arbitrary vault contents and must not
invent implicit defaults such as `/README.md` or a generated placeholder home.
Requiring an explicit homepage keeps the installed docsify shell aligned with
the user's actual navigation intent and avoids turning a fallback default into a
hidden product contract.

## Serving Model

After initialization, users should be able to keep using:

```bash
markbase web serve
```

and then open:

```text
http://127.0.0.1:3000/index.html
```

For docsify-first browsing, the frontend integration should also define a root
entry behavior:

- when the browser requests `/`, markbase should serve the generated
  `/index.html` shell instead of treating `/` as a canonical vault document
  route
- this root-to-index behavior exists to make the docsify shell directly
  enterable from the site root and does not change canonical note or resource
  routing rules under `design-003`

This works because markbase already serves non-Markdown files as raw resources.
The docsify shell remains just another served asset at the vault root, while
markbase continues to act as the content server for Markdown and attachments.

Because the supported browser entrypoint is now the generated docsify shell,
`markbase web serve` should treat the presence of `<base-dir>/index.html` as a
startup prerequisite for user-facing browser mode. If the shell has not been
initialized yet, `web serve` should fail fast with an explanatory message that
directs the user to run:

```bash
markbase web init-docsify --homepage <canonical-url>
```

This keeps browser-serving behavior honest: the command should not report a
ready web experience when the supported browser entrypoint does not exist.

`markbase web get` remains a backend inspection tool and does not depend on the
presence of `index.html`.

This preserves the backend boundary from `design-003`:

- markbase still serves Markdown and resource bytes
- docsify remains a frontend consumer
- the new command only installs the shell; it does not change the web server
  into an HTML app framework

## URL Handling

### Current backend reality

The current backend emits internal note links in Markdown as absolute paths
such as:

```md
[alice](/entities/person/alice.md)
```

This is acceptable as a backend route contract, but it causes docsify to leave
its HTML shell when those links are clicked.

### Initial docsify solution

The generated docsify shell should include a small frontend plugin that adapts
markbase-emitted internal document links for docsify navigation.

In the first version, that plugin should:

- inspect rendered links inside the docsify app container
- identify markbase internal document routes such as `.md` and `.base`
- rewrite those hrefs so that clicking them stays inside the docsify shell
- leave binary resource URLs untouched so images and attachments still resolve
  directly

### Resource URL normalization

The docsify integration must distinguish document navigation from resource
delivery:

- Markdown note routes and `.base` routes should be adapted into docsify-local
  navigation
- binary resources such as images, PDFs, and other attachments must remain
  direct resource requests and must not be rewritten into docsify hash routes

In practice, docsify may further normalize rendered HTML after Markdown is
converted, especially for image elements. When markbase emits an absolute
resource URL such as:

```md
![](/logs/opportunities/_assets/example.png)
```

the frontend integration must preserve that absolute resource identity. If the
rendered DOM contains a docsify-adjusted image `src` that no longer matches the
server-emitted absolute resource path, but the original absolute path is still
available from docsify metadata such as `data-origin`, the plugin should
restore the image request URL back to the original absolute resource path.

This rule applies to image-style resources that docsify renders as `<img>`
elements. Non-image attachments such as PDFs and generic binary downloads
should continue to use their direct absolute resource links without docsify hash
adaptation.

This means the first docsify integration solves the navigation problem in the
frontend layer without changing the backend contract from `design-003`.

## Why This Is A Separate Design

Docsify integration is not just a one-line fix for links.

It opens a separate frontend problem space that includes:

- shell installation and ownership
- homepage configuration
- internal route adaptation
- future callout styling
- future Mermaid integration
- future sidebar, search, and theme decisions

These concerns are broader than a small patch to `design-003`, so they should
be tracked as their own design document.

## Relationship To `design-003`

`design-003` remains the backend source of truth for:

- request routing
- note and `.base` rendering
- OFM normalization
- resource delivery

This new design adds a frontend integration layer on top:

- how docsify is installed
- how docsify navigates markbase content
- how frontend-only presentation concerns are owned

For the purposes of this design, the backend href contract is assumed to stay
as-is. Any future decision to change backend-emitted link shapes would require
a separate backend design decision and should not be folded into docsify
installation work.

## Future Frontend Responsibilities

The generated docsify shell is also the right place to host future
presentation-only behaviors, for example:

- callout styling for `> [!info]` and related blocks
- Mermaid rendering for preserved fenced code blocks
- optional sidebar loading
- optional search integration
- theme customization

These are frontend concerns because they affect visual interpretation of already
normalized Markdown, not vault-aware backend semantics.

## Definition Of Done For The First Implementation

This design should be considered implemented only when all of the following are
true:

1. `markbase web init-docsify` exists and is documented
2. the command writes a minimal docsify shell at `<base-dir>/index.html`
3. the command has explicit overwrite behavior and does not silently replace an
   existing `index.html` without `--force`
4. `markbase web serve` fails with an explanatory setup error when
   `<base-dir>/index.html` is missing
5. opening `/` or `/index.html` renders the generated shell and configured
   homepage route
6. clicking internal `.md` and `.base` links stays inside the docsify shell
7. binary resource links continue to resolve as direct resources
8. README and ARCHITECTURE document the docsify integration boundary

## Open Questions

1. Should `init-docsify` generate only `index.html` at first, or should it also
   generate a small local CSS file for callout styling scaffolding?
2. Should the generated shell keep using docsify CDN assets by default, or
   should markbase eventually support vendored frontend assets?
3. Should markbase later add a companion command such as
   `markbase web open-docsify` for convenience, or is shell generation alone
   sufficient?
