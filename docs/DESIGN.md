# Design Docs

This file is the entry index for managed and supporting design material.

## Stable Global Docs

- `../ARCHITECTURE.md`: system map and cross-module boundaries
- `guidelines/core-beliefs.md`: project-wide engineering beliefs

## Implemented Design Docs

- `design-docs/implemented/design-001-links-and-embeds.md`: shared contract for wiki-links, embeds, backlinks, and rename-safe target normalization
- `design-docs/implemented/design-002-render.md`: active render subsystem contract for `note render`, `.base` expansion, and note-embed behavior
- `design-docs/implemented/design-004-note-verify.md`: verification contract for `note verify`, template checks, issue levels, and exit behavior
- `design-docs/implemented/design-005-indexing.md`: indexing contract for traversal scope, ignore behavior, resource treatment, and incremental updates
- `design-docs/implemented/design-006-template-system.md`: template subsystem contract for normalization, instance creation, and ownership boundaries with `note verify`
- `design-docs/implemented/design-008-note-resolve.md`: resolution contract for `note resolve`, including input rules and JSON result semantics
- `design-docs/implemented/design-009-document-system.md`: managed-document system contract for naming, lifecycle, and directory layout

## Candidate Design Docs

- `design-docs/candidate/design-003-web-note-view.md`: planned web delivery design for rendered Obsidian-compatible notes

## Obsolete Design Docs

- `design-docs/obsolete/design-006-patch-01-template-instance-metadata-transition.md`: merged patch for the `_schema.create` transition; retained for history with `status: obsolete:merged`

## Supporting References

- `references/legacy-designs/index.md`: index for design material created before the managed-document migration
- `references/legacy-designs/query_design.md`: historical query-language design reference
- `references/legacy-designs/properties_design.md`: historical `file.*` vs `note.*` field model reference
- `references/legacy-designs/note_render_design.md`: historical render design retained for background context
- `references/legacy-designs/template_schema.md`: deprecated template-system redirect retained for history
