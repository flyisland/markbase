---
id: design-012-patch-02
title: "Dynamic Docsify Shell Serving"
status: draft
parent: design-012
module: web-frontend
---

# Dynamic Docsify Shell Serving

**Status:** Draft  
**Target:** `design-012` frontend integration follow-up

## Purpose

This patch records a future-facing follow-up to `design-012`: instead of
requiring `markbase web init-docsify` to generate `<base-dir>/index.html`,
`markbase web serve` could dynamically return the supported docsify entry HTML
at request time while preserving the current explicit export path.

This patch is exploratory only. It does not change the current implemented
contract in `design-012`.

## Current State

`design-012` defines the current model:

- users explicitly run `markbase web init-docsify --homepage <canonical-url>`
- markbase writes a single `index.html` into the base-dir root
- `markbase web serve` refuses to start unless that entry HTML exists
- the generated entry HTML is version-checked against the serving binary

That model keeps docsify entry HTML installation explicit and gives users a
concrete browser entry artifact that can be inspected, regenerated, and
version-locked.

## Proposed Alternative

Under this alternative, `markbase web serve` would own the docsify entry HTML
directly:

- `markbase web init-docsify` remains available as an explicit entry HTML
  export command
- `markbase web serve` first checks whether `<base-dir>/index.html` already
  exists
- if `index.html` exists, `web serve` uses that exported entry HTML after
  verifying that its embedded `markbase` version matches the serving binary
- if `index.html` exists but its embedded `markbase` version does not match
  the serving binary, `web serve` must not fail startup; it should log a clear
  message and fall back to dynamically generated docsify entry HTML
- if `index.html` does not exist, `web serve` dynamically generates the same
  docsify entry HTML from the current binary's built-in templates
- requesting `/` returns the selected entry HTML
- requesting `/index.html` returns the same entry HTML content

This alternative must not create two different entry HTML implementations.
`markbase web serve` dynamic entry HTML generation and `markbase web
init-docsify` export must both call the same shared entry-HTML rendering path,
so that the HTML returned dynamically is identical to the HTML that
`init-docsify` would write for the same homepage and binary version.

This means browser usage has two supported paths:

1. explicit exported entry HTML via `init-docsify`
2. dynamic fallback entry HTML when no exported entry HTML is present

Under this patch's preferred direction, these two paths are not equal in
product positioning:

- `web serve` dynamic entry HTML is the default browser entry experience
- `web init-docsify` is retained primarily as an export and debugging tool
- advanced users may also use `init-docsify` when they intentionally want to
  inspect or manually modify the exported `index.html`

In both cases, `web serve` should print a clear `INFO` message at startup so
users can immediately tell which mode is active:

- using installed `index.html` after version validation
- ignoring an installed but stale `index.html` and dynamically serving the
  built-in docsify entry HTML instead
- or dynamically serving the built-in docsify entry HTML because no
  `index.html` exists

The backend Markdown and resource contracts from `design-003` and `design-012`
would remain unchanged. Only the docsify entry HTML delivery model would
change.

## Expected Benefits

- zero docsify entry HTML installation step for users
- no stale `index.html` after CLI upgrades
- no explicit entry-HTML version mismatch state to manage
- simpler first-run experience for browser usage

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
- an installed but version-mismatched `index.html` must not block browser
  startup; it should be ignored in favor of dynamic entry HTML
- the implementation must use one shared entry-HTML renderer rather than
  separate "serve-time entry HTML" and "export-time entry HTML" codepaths
- entry HTML metadata such as `markbase` version and git information should
  still be
  exposed in the returned HTML
- the dynamic entry HTML must preserve the current docsify behaviors already
  defined in `design-012`, including internal link adaptation, resource
  normalization, and callout UI

## Migration Options

There are two plausible migration shapes:

1. Replace the explicit docsify entry HTML export model entirely.
2. Keep `web init-docsify` as an optional static export command, while `web
   serve` supports both exported-entry-HTML mode and dynamic-entry-HTML
   fallback mode.

The second path is the currently preferred direction in this draft because it
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

No change is approved.

Current implementation remains the explicit single-file docsify entry HTML
model in `design-012`. This patch exists only to preserve the alternative for
future evaluation as a possible follow-up to that design.
