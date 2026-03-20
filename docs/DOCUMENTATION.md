# Documentation Guide

This file is the repository-level guide to markbase's Specmate-aligned
documentation system.

The formal managed-document contract lives in
`docs/design-docs/implemented/design-009-document-system.md`.

## 1. Managed vs Repository vs Supporting

Use these categories:

- Managed documents: PRDs, Design Docs, Design Patches, Exec Plans, Guidelines,
  Task Specs, `specs/project.md`, and `specs/org.md`
- Repository documents outside management: `AGENTS.md`, `ARCHITECTURE.md`,
  `README.md`, `docs/DOCUMENTATION.md`, `docs/DESIGN.md`, and `docs/PLANS.md`
- Supporting material: `docs/references/` and `docs/generated/`

## 2. Placement Rules

When creating or moving managed documents, use these paths:

- `docs/prd/draft/`, `docs/prd/approved/`, `docs/prd/obsolete/`
- `docs/design-docs/draft/`, `docs/design-docs/candidate/`,
  `docs/design-docs/implemented/`, `docs/design-docs/obsolete/`
- `docs/exec-plans/draft/`, `docs/exec-plans/active/`,
  `docs/exec-plans/archived/`
- `docs/guidelines/`
- `specs/active/`
- `specs/archived/`
- `specs/project.md`
- `specs/org.md`

Do not place active design contracts at `docs/design-docs/` root.

Do not use `specs/archived/` or `docs/exec-plans/archived/`; those were
pre-migration locations.

## 3. Naming Rules

- PRDs, Design Docs, Design Patches, and Exec Plans use three-digit IDs.
- Task Specs use four-digit IDs.
- Task Spec IDs are globally unique and never reused.
- Task Specs use `.md`, not `.spec`.
- Design Patches use `design-<parent-id>-patch-<nn>-<slug>.md`.

## 4. Authority Order

When documents overlap, use this order:

1. `specs/active/task-*.md` for the specific task they cover
2. `docs/exec-plans/active/` for sequencing and execution progress
3. `docs/design-docs/implemented/` for current system contracts
4. `docs/design-docs/candidate/` for approved-but-not-yet-implemented targets
5. `docs/guidelines/` plus `ARCHITECTURE.md` for cross-cutting rules
6. `README.md` for user-visible CLI behavior
7. `docs/references/` and `docs/generated/` for background only

Do not execute against `docs/design-docs/draft/` documents.

## 5. Lifecycle Rules

- New implementation-ready design docs start in `docs/design-docs/candidate/`.
- Once code fully matches the design, move the doc to
  `docs/design-docs/implemented/`.
- Superseded design docs and merged design patches move to
  `docs/design-docs/obsolete/`.
- Completed or cancelled Task Specs move to `specs/archived/`.
- Completed or abandoned Exec Plans move to `docs/exec-plans/archived/`.
- Historical material that is not part of the managed system belongs in
  `docs/references/`.

## 6. Index Maintenance

When you change the documentation layout or the managed-doc inventory, update
these files in the same change:

- `docs/DESIGN.md`
- `docs/PLANS.md`
- `AGENTS.md`
- `ARCHITECTURE.md` if documentation roles or boundaries changed
- `README.md` if user-visible behavior changed

## 7. Current Migration Notes

Markbase now uses the full managed-document layout, including:

- lifecycle directories for PRDs, Design Docs, and Exec Plans
- `docs/guidelines/` for cross-cutting guidance
- `specs/project.md` and `specs/org.md`
- four-digit `task-0001` style IDs

Pre-migration legacy design and planning material is preserved under
`docs/references/legacy-designs/` and `docs/references/legacy-exec-plans/`.
