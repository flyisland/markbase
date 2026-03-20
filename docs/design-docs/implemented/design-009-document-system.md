---
id: design-009
title: "Document System"
status: implemented
module: documentation-system
---

# Document System Design

This document defines the managed document types, naming rules, lifecycle
states, and directory layout for markbase's Specmate-aligned documentation
system.

It is the source of truth for how managed project documents are named, where
they live, and when they move.

## 1. Managed document types

Markbase manages these document types:

- PRD
- Design Doc
- Design Patch
- Exec Plan
- Guideline
- Task Spec
- `specs/project.md`
- `specs/org.md`

Repository-owned onboarding and structural documents may still exist outside
the managed model:

- `AGENTS.md`
- `ARCHITECTURE.md`
- `README.md`
- `docs/DOCUMENTATION.md`
- `docs/DESIGN.md`
- `docs/PLANS.md`

Supporting material lives under:

- `docs/references/`
- `docs/generated/`

## 2. Naming rules

Managed files use these patterns:

```text
prd-001-<slug>.md
design-001-<slug>.md
design-001-patch-01-<slug>.md
exec-001-<slug>.md
task-0001-<slug>.md
docs/guidelines/<slug>.md
```

Rules:

- PRDs, Design Docs, Design Patches, and Exec Plans use three-digit IDs.
- Task Specs use four-digit IDs.
- IDs are permanent and never reused.
- Task Spec IDs are globally unique across the repo.
- Design Patch IDs are bound to one parent Design Doc.

## 3. Lifecycle states

### PRD

`draft -> approved -> obsolete`

### Design Doc

`draft -> candidate -> implemented -> obsolete`

Design Patch terminal state: `obsolete:merged`

### Exec Plan

`draft -> active -> completed`

Alternative terminal state: `abandoned`

### Task Spec

`draft -> active -> completed`

Alternative terminal state: `cancelled`

### Guideline

Always active. No lifecycle directories or IDs.

### project.md / org.md

Always `active`.

## 4. Directory layout

Frontmatter state is authoritative. Directory layout must reflect it.

```text
docs/
  guidelines/
  references/
  generated/
  prd/
    draft/
    approved/
    obsolete/
  design-docs/
    draft/
    candidate/
    implemented/
    obsolete/
  exec-plans/
    draft/
    active/
    archived/
specs/
  active/
  archived/
  project.md
  org.md
```

Directory mapping:

- PRD `draft` -> `docs/prd/draft/`
- PRD `approved` -> `docs/prd/approved/`
- PRD `obsolete` -> `docs/prd/obsolete/`
- Design Doc `draft` -> `docs/design-docs/draft/`
- Design Doc `candidate` -> `docs/design-docs/candidate/`
- Design Doc `implemented` -> `docs/design-docs/implemented/`
- Design Doc `obsolete` / Design Patch `obsolete:merged` -> `docs/design-docs/obsolete/`
- Exec Plan `draft` -> `docs/exec-plans/draft/`
- Exec Plan `active` -> `docs/exec-plans/active/`
- Exec Plan `completed` / `abandoned` -> `docs/exec-plans/archived/`
- Task Spec `draft` / `active` -> `specs/active/`
- Task Spec `completed` / `cancelled` -> `specs/archived/`

## 5. Current markbase mapping

Current active implementation contracts live under
`docs/design-docs/implemented/`.

Current planned-but-unimplemented design contracts live under
`docs/design-docs/candidate/`.

Historical unmanaged material from the pre-migration system lives under
`docs/references/legacy-designs/` and `docs/references/legacy-exec-plans/`.

## 6. Change rules

- Do not create new active design docs at `docs/design-docs/` root.
- Do not create new completed task specs under `specs/archived/`.
- Do not reuse an existing task ID for a second task.
- When a status changes, move the file and update frontmatter in the same
  change.
- When a patch is merged, fold its content back into the parent Design Doc and
  move the patch to `docs/design-docs/obsolete/` with `status: obsolete:merged`.
