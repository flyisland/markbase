---
id: design-015
title: "Dynamic Docsify Shell Serving"
status: draft
module: web-frontend
---

# Dynamic Docsify Shell Serving

**Status:** Draft  
**Target:** markbase web frontend integration

## Purpose

This document records a future-facing idea: instead of requiring
`markbase web init-docsify` to generate `<base-dir>/index.html`, `markbase web
serve` could generate and return the supported docsify shell dynamically at
request time.

This is only a design placeholder. It does not change the current implemented
contract in `design-012`.

## Current State

`design-012` defines the current model:

- users explicitly run `markbase web init-docsify --homepage <canonical-url>`
- markbase writes a single `index.html` into the base-dir root
- `markbase web serve` refuses to start unless that shell exists
- the generated shell is version-checked against the serving binary

That model keeps shell installation explicit and gives users a concrete browser
entry artifact that can be inspected, regenerated, and version-locked.

## Proposed Alternative

Under this alternative, `markbase web serve` would own the docsify shell
directly:

- `markbase web init-docsify` remains available as an explicit shell export
  command
- `markbase web serve` first checks whether `<base-dir>/index.html` already
  exists
- if `index.html` exists, `web serve` uses that installed shell after verifying
  that its embedded `markbase` version matches the serving binary
- if `index.html` does not exist, `web serve` dynamically generates the same
  docsify shell from the current binary's built-in templates
- requesting `/` returns the selected shell
- requesting `/index.html` returns the same shell content

This means browser usage has two supported paths:

1. explicit exported shell via `init-docsify`
2. dynamic fallback shell when no exported shell is present

In both cases, `web serve` should print a clear `INFO` message at startup so
users can immediately tell which mode is active:

- using installed `index.html` after version validation
- or dynamically serving the built-in shell because no `index.html` exists

The backend Markdown and resource contracts from `design-003` would remain
unchanged. Only the shell delivery model would change.

## Expected Benefits

- zero shell installation step for users
- no stale `index.html` after CLI upgrades
- no explicit shell-version mismatch state to manage
- simpler first-run experience for browser usage

## Expected Costs

- `web serve` becomes both a content server and a shell generator
- the browser entry artifact is no longer directly inspectable on disk
- debugging becomes less concrete because the shell is produced at runtime
- the current explicit installation model in `design-012` would need to be
  revised or partially replaced
- route behavior for `/` and `/index.html` would become product-critical rather
  than install-time details

## Design Constraints

If this direction is ever implemented, these constraints should hold:

- the backend contract remains Markdown plus resource bytes, not server-side
  note HTML generation
- the shell remains a frontend concern owned by `src/web/`
- `/` and `/index.html` must serve the same shell content
- shell metadata such as `markbase` version and git information should still be
  exposed in the returned HTML
- the dynamic shell must preserve the current docsify behaviors already defined
  in `design-012`, including internal link adaptation, resource normalization,
  and callout UI

## Migration Options

There are two plausible migration shapes:

1. Replace the explicit shell-installation model entirely.
2. Keep `web init-docsify` as an optional static export command, while `web
   serve` supports both installed-shell mode and dynamic-shell fallback mode.

The second path is the currently preferred direction in this draft because it
preserves a static export mode for debugging and offline inspection while also
removing the hard requirement that users pre-generate `index.html` before first
browser use.

## Open Questions

- Should `web init-docsify` be removed, retained, or repurposed as an export
  command?
- Should the homepage remain explicit, or should dynamic serving introduce a
  new default homepage contract?
- Should non-root shell URLs such as `/index.html` remain stable forever if the
  primary entrypoint becomes `/`?
- How much shell metadata should remain visible in the UI versus only embedded
  in HTML?

## Decision Status

No change is approved.

Current implementation remains the explicit single-file shell model in
`design-012`. This draft exists only to preserve the alternative for future
evaluation.
