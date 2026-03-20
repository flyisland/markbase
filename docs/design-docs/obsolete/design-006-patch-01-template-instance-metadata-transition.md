---
id: design-006-patch-01-template-instance-metadata-transition
title: "Template Instance Metadata Transition"
status: obsolete:merged
parent: design-006
merged-into: design-006
---

# Template Instance Metadata Patch

**Status:** Obsolete: Merged  
**Target:** markbase CLI  
**Merged into:** `docs/design-docs/implemented/design-006-template-system.md`, `docs/design-docs/implemented/design-004-note-verify.md`, `README.md`

## Why This File Still Exists

This file is retained as an archived patch record.

It is no longer an active source of truth for markbase template behavior.

Its implementation-era content has been merged into the formal documents listed above, so readers should use those documents for current behavior.

## Current Status

`design-007` was created as a temporary patch contract during `exec-003` to guide the transition from legacy outer-frontmatter instance semantics to the create-surface model that is now represented as `_schema.create`.

That transition is now complete:

- template creation semantics live in `docs/design-docs/implemented/design-006-template-system.md`
- verifier semantics live in `docs/design-docs/implemented/design-004-note-verify.md`
- user-facing template examples live in `README.md`

This file therefore remains only for historical traceability.

## Why It Is Archived Instead Of Active

Keeping `design-007` active after the merge would create two problems:

1. it would leave a second active contract alongside `design-006` and `design-004`
2. it would make it unclear which document owns the final post-transition behavior

The correct active ownership after the merge is:

- `design-006`: template normalization, `_schema.create`, and `note new --template`
- `design-004`: `note verify` semantics, including stable identity and mutable seed behavior
- `README.md`: user-facing examples and command guidance

## Historical Scope

`design-007` originally introduced or clarified these transition topics:

- moving note-creation defaults under the schema-owned create surface
- making `templates` system-derived during note creation
- separating creation-time seed values from verification-time constraints
- requiring stable identity fields such as `type` to be modeled through both creation and schema surfaces
- removing legacy outer-frontmatter literal-match verification semantics

Those topics are now part of the merged formal contract, not this archived patch note.
