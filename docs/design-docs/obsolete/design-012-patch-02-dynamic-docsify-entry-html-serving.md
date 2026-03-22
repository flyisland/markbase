---
id: design-012-patch-02
title: "Dynamic Docsify Entry HTML Serving"
status: obsolete:merged
parent: design-012
merged-into: design-012
module: web-frontend
---

# Dynamic Docsify Entry HTML Serving

**Status:** Obsolete: Merged  
**Target:** `design-012` frontend integration follow-up

This file is retained as an archived patch record.

Its active content has been folded back into
`docs/design-docs/implemented/design-012-docsify-frontend-integration.md`.

## Purpose

This patch records a follow-up to `design-012`: instead of requiring users to
pre-generate `<base-dir>/index.html`, `markbase web serve` can dynamically
return the supported docsify entry HTML when the user explicitly requests
dynamic mode via `--homepage`, while still preserving the explicit export path.

This patch defines the follow-up contract that makes dynamic docsify entry HTML
the default browser path while preserving explicit export.

## Current State

`design-012` defines the current model:

- users explicitly run `markbase web init-docsify --homepage <homepage-ref>`
- markbase writes a single docsify entry HTML file as `index.html` into the
  base-dir root
- `markbase web serve` used to refuse to start unless that entry HTML existed
- the generated entry HTML is version-checked against the serving binary

That model keeps docsify entry HTML installation explicit and gives users a
concrete browser entry artifact that can be inspected, regenerated, and
version-locked.

## Proposed Alternative

Under this alternative, `markbase web serve` owns the docsify entry HTML
selection directly:

- `markbase web init-docsify` remains available as an explicit entry HTML
  export command
- `markbase web serve` has two explicit modes:
  1. without `--homepage`, it only reuses existing exported `index.html`
  2. with `--homepage`, it always dynamically generates docsify entry HTML
- when `web serve` runs without `--homepage`, `<base-dir>/index.html` must
  exist and its embedded `markbase` version must match the serving binary
- when `web serve` runs with `--homepage`, any existing `<base-dir>/index.html`
  is not used; the server logs a warning that the exported file was found but
  ignored because dynamic mode was requested
- requesting `/` returns the selected entry HTML
- requesting `/index.html` returns the same entry HTML content

This alternative must not create two different entry HTML implementations.
`markbase web serve` dynamic entry HTML generation and `markbase web
init-docsify` export must both call the same shared entry-HTML rendering path,
so that the HTML returned dynamically is identical to the HTML that
`init-docsify` would write for the same homepage and binary version.

Homepage input should not be limited to canonical URLs. Both
`web serve --homepage` and `web init-docsify --homepage` should accept:

- note names
- vault-relative `file.path`
- canonical URLs

The implementation must resolve those forms to one existing `.md` or `.base`
target before entry HTML generation, then canonicalize the result back to the
stable `/<file.path>` route used by the browser entry HTML.

This means browser usage has two supported paths:

1. explicit exported entry HTML via `init-docsify`
2. explicit dynamic entry HTML via `web serve --homepage <homepage-ref>`

Under this patch's preferred direction, these two paths are not equal in
product positioning:

- `web serve` dynamic entry HTML is the default browser entry experience
- `web init-docsify` is retained primarily as an export and debugging tool
- advanced users may also use `init-docsify` when they intentionally want to
  inspect or manually modify the exported `index.html`

In both cases, `web serve` should print clear startup logs so
users can immediately tell which mode is active:

- using installed `index.html` after version validation
- dynamically serving the built-in docsify entry HTML
- warning when an installed `index.html` was found but ignored because
  `--homepage` explicitly requested dynamic mode

The backend Markdown and resource contracts from `design-003` and `design-012`
would remain unchanged. Only the docsify entry HTML delivery model would
change.

## Expected Benefits

- no required docsify entry HTML installation step for normal browser use
- explicit and debuggable separation between static reuse mode and dynamic mode
- simpler first-run experience for browser usage when users provide homepage
  explicitly

## Expected Costs

- `web serve` becomes both a content server and a docsify entry HTML generator
- the browser entry artifact is no longer directly inspectable on disk
- debugging becomes less concrete because the entry HTML is produced at runtime
- the current explicit installation model in `design-012` would need to be
  revised or partially replaced
- route behavior for `/` and `/index.html` would become product-critical rather
  than install-time details

## Design Constraints

If this direction is ever implemented, these constraints should hold:

- the backend contract remains Markdown plus resource bytes, not server-side
  note HTML generation
- the docsify entry HTML remains a frontend concern owned by `src/web/`
- `/` and `/index.html` must serve the same entry HTML content
- the dynamically served entry HTML must be identical to the entry HTML
  produced by
  `markbase web init-docsify` for the same inputs
- when `--homepage` is not provided, an installed but version-mismatched
  `index.html` must still block startup because static reuse mode was
  explicitly selected
- when `--homepage` is provided, any installed `index.html` must be ignored in
  favor of dynamic entry HTML
- the implementation must use one shared entry-HTML renderer rather than
  separate "serve-time entry HTML" and "export-time entry HTML" codepaths
- entry HTML metadata such as `markbase` version and git information should
  still be
  exposed in the returned HTML
- homepage inputs must resolve only to existing `.md` or `.base` targets, not
  binary resources
- the dynamic entry HTML must preserve the current docsify behaviors already
  defined in `design-012`, including internal link adaptation, resource
  normalization, and callout UI

## Migration Options

There are two plausible migration shapes:

1. Replace the explicit docsify entry HTML export model entirely.
2. Keep `web init-docsify` as an optional static export command, while `web
   serve` supports both exported-entry-HTML mode and dynamic-entry-HTML
   fallback mode.

The second path is the chosen direction because it
preserves a static export mode for debugging and offline inspection while also
making `web serve` dynamic entry HTML the default browser path instead of
requiring users to pre-generate `index.html` before first browser use.

That preference only holds if the implementation keeps entry HTML generation
single sourced. If dynamic serving and exported files can drift, this
alternative is not acceptable.

## Open Questions

- Should `web init-docsify` be removed, retained, or repurposed as an export
  command?
- Should the homepage remain explicit, or should dynamic serving introduce a
  new default homepage contract?
- Should non-root docsify entry HTML URLs such as `/index.html` remain stable
  forever if the primary entrypoint becomes `/`?
- How much docsify entry HTML metadata should remain visible in the UI versus
  only embedded in HTML?

## Decision Status

Implemented via `task-0021` and merged into `design-012`.

The active steady-state contract now lives in
`docs/design-docs/implemented/design-012-docsify-frontend-integration.md`.
