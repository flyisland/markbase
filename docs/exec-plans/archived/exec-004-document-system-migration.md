---
id: exec-004
title: "Document System Migration"
status: completed
design-doc: design-009
---

## Goal

Migrate markbase's repository documentation to the Specmate-managed document
system defined in `design-009`.

## Phases

### Phase 1: Structure

- [x] task-0013: create lifecycle directories, move managed documents, and
  establish guideline, project, and org entry points

## Dependencies

task-0013

## Progress Notes

- 2026-03-20: moved active design contracts into `candidate/` and
  `implemented/`
- 2026-03-20: moved obsolete patch material into `docs/design-docs/obsolete/`
- 2026-03-20: migrated completed task specs to four-digit IDs under
  `specs/archived/`
- 2026-03-20: preserved pre-migration historical material under
  `docs/references/`

## Definition of Done

`exec-004` is complete only when:

1. managed docs use the Specmate lifecycle directories
2. task specs use `.md` and four-digit IDs
3. `docs/guidelines/`, `specs/project.md`, and `specs/org.md` exist
4. repository indexes and onboarding docs point to the migrated locations
5. legacy material is preserved outside the managed-document directories
