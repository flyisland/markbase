---
id: design-012-patch-01
title: "Docsify Callout UI And Shell Authoring"
status: obsolete:merged
parent: design-012
merged-into: design-012
module: web-frontend
---

# Docsify Callout UI And Shell Authoring

**Status:** Obsolete: Merged  
**Target:** `design-012` frontend integration follow-up

This file is retained as an archived patch record.

Its active content has been folded back into
`docs/design-docs/implemented/design-012-docsify-frontend-integration.md`.

## Purpose

This patch refines the docsify frontend integration defined by `design-012` in
two areas that were intentionally left open in the initial implementation:

- ownership and implementation shape for Obsidian callouts, including foldable
  callouts
- how the generated docsify shell should be authored inside the repository
  without changing the single-file output contract for users

This patch does not change the core backend route, render, or resource
contracts defined by `design-003` and adopted by `design-012`.

## Problem

The initial docsify integration established shell installation, homepage
configuration, and internal-link adaptation. It did not settle two follow-up
questions:

1. whether callouts should be interpreted by the backend or by docsify UI
2. whether a growing docsify shell should continue to live as a large inline
   string in Rust source

Those questions become concrete once the frontend grows beyond link adaptation.

## Decision Summary

This patch makes four decisions:

1. Obsidian callout rendering is a docsify UI responsibility, not a backend
   HTML-generation responsibility.
2. The backend responsibility for callouts is limited to preserving Markdown
   container structure during render-time note and `.base` expansion.
3. Foldable callouts must follow Obsidian-compatible marker semantics:
   `[!type]`, `[!type]+`, and `[!type]-`.
4. `web init-docsify` should continue to generate a single `index.html` for
   users, while repository implementation should move toward template-backed
   authoring rather than expanding one Rust string literal indefinitely.

## Callout Ownership

Callouts are presentation-layer Markdown syntax. They should therefore be owned
by the docsify frontend layer.

The division of responsibility is:

- backend:
  preserve blockquote and callout container structure
- backend:
  keep the original callout marker line in Markdown output
- backend:
  do not generate dedicated callout HTML
- backend:
  do not add a docsify-specific callout rewrite in OFM normalization
- frontend:
  recognize callout marker syntax after Markdown has been rendered
- frontend:
  apply callout UI, fold/unfold interaction, and styling

This keeps the backend focused on vault-aware semantics and keeps the browser
responsible for UI interpretation.

## Foldable Callout Semantics

When callout support is added to the docsify shell, it must support these
marker forms:

- `[!type]`
  non-foldable callout
- `[!type]+`
  foldable callout expanded by default
- `[!type]-`
  foldable callout collapsed by default

Trailing text on the same marker line becomes the visible title.

Nested callouts remain supported because the backend continues to preserve the
underlying nested blockquote structure in Markdown output.

## Frontend Upgrade Strategy

The preferred implementation strategy is a docsify-side DOM upgrade after
Markdown has already been rendered to HTML.

The plugin should:

- scan rendered `blockquote` elements inside the docsify app container
- detect an initial marker line matching Obsidian callout syntax
- derive the callout type, foldable state, and title
- replace plain blockquote presentation with callout UI while preserving the
  rendered HTML body
- process nested matches from the inside out

The preferred DOM representation is:

- non-foldable callouts:
  a styled container such as `<div class="mb-callout" data-callout="info">`
- foldable callouts:
  native disclosure UI such as
  `<details class="mb-callout" data-callout="faq">`
- foldable title row:
  `<summary>`

Using `<details>` / `<summary>` keeps fold/unfold behavior accessible and avoids
introducing unnecessary custom state machinery.

This DOM-upgrade approach is preferred over browser-side Markdown pre-processing
because it reuses docsify's own parser output and avoids introducing a second
partial Markdown parser in frontend code.

## Generated Shell Authoring

This patch does not change the user-facing output shape of `web init-docsify`.

Users should still receive a single generated artifact:

```text
<base-dir>/index.html
```

The improvement is internal to repository authoring.

The preferred authoring model is:

- final generated output remains one `index.html`
- repository source may use template or asset files to maintain shell HTML, JS,
  and CSS more clearly
- generation injects dynamic values such as `homepage` into the final output
- richer frontend behaviors such as callout UI should be maintained in those
  source templates rather than growing a monolithic inline Rust string

This preserves the current installation contract while giving the repo a better
place to maintain non-trivial frontend logic.

## Relationship To Existing Designs

This patch refines, but does not replace, existing ownership boundaries.

- `design-003` remains the backend source of truth for route resolution, render
  semantics, and web-targeted Markdown output
- `design-012` remains the source of truth for docsify shell installation and
  navigation adaptation
- this patch adds a more precise frontend contract for callout UI and shell
  authoring

If this patch is implemented, its content should be folded back into
`design-012` and the patch should then move to `docs/design-docs/obsolete/`
with `status: obsolete:merged`.

## Non-Goals

- adding backend-generated callout HTML
- changing canonical backend href contracts
- requiring users to manage a multi-file docsify install by default
- adding Mermaid, search, or sidebar behavior in the same patch
