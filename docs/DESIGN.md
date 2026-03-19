# Design Docs

This file is the entry index for design documents.

## Stable Global Docs

- `../ARCHITECTURE.md`: system map and cross-module boundaries
- `core-beliefs.md`: project-level design beliefs

## Design Documents

- `design-docs/design-001-links-and-embeds.md`: Obsidian link, embed, and backlink design for markbase
- `design-docs/design-002-render.md`: active render subsystem contract for `note render`, `.base` expansion, and renderer module boundaries
- `design-docs/design-003-web-note-view.md`: web delivery design for serving rendered Obsidian-compatible notes through docsify
- `design-docs/design-004-note-verify.md`: active verification contract for `note verify`, template/schema checks, issue levels, and exit behavior
- `design-docs/design-005-indexing.md`: active indexing contract for traversal scope, ignore behavior, resource treatment, and incremental updates
- `design-docs/design-006-template-system.md`: active template subsystem contract for normalization, instance creation, and ownership boundaries with `note verify`
- `design-docs/design-008-note-resolve.md`: active resolution contract for `note resolve`, including input rules, indexed name/alias matching, and JSON result semantics

## Legacy Design Docs

- `design-docs/legacy/design-007-template-instance-metadata-patch.md`: archived transition patch for `_schema.instance`; retained for history after its content was merged into `design-docs/design-006-template-system.md`, `design-docs/design-004-note-verify.md`, and `README.md`
- `design-docs/legacy/index.md`: archived feature-level design docs that predate the current Exec Plan / Task Spec structure
- `design-docs/legacy/query_design.md`: query language and translation behavior
- `design-docs/legacy/properties_design.md`: file vs note property model
- `design-docs/legacy/note_render_design.md`: historical render design retained for background context; superseded by `design-docs/design-002-render.md` for active behavior
- `design-docs/legacy/template_schema.md`: deprecated redirect to `design-docs/design-006-template-system.md`
