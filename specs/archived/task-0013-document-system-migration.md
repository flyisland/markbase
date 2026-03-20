---
id: task-0013
title: "Migrate markbase to the Specmate document system"
status: completed
exec-plan: exec-004
boundaries:
  allowed:
    - "AGENTS.md"
    - "ARCHITECTURE.md"
    - "docs/**"
    - "specs/**"
  forbidden_patterns:
    - "src/**"
    - "tests/**"
completion_criteria:
  - id: "cc-001"
    scenario: "managed docs live under lifecycle directories"
    test: "doc review"
  - id: "cc-002"
    scenario: "task specs use four-digit IDs and `.md` filenames"
    test: "doc review"
  - id: "cc-003"
    scenario: "entry-point docs and indexes match the migrated layout"
    test: "doc review"
---

## Intent

Bring markbase's repository documentation into direct alignment with the
Specmate document model without changing product behavior.

## Decisions

- use `docs/design-docs/implemented/` as the source of truth for shipped design
  contracts
- preserve historical material under `docs/references/` rather than leaving it
  in managed-document directories
- treat `docs/guidelines/core-beliefs.md` as the repository's first managed
  Guideline
- reserve `task-0012` for the existing active `note resolve` task and record
  this migration as `task-0013`

## Completion Criteria

Scenario: managed docs live under lifecycle directories
Test: doc review
Given the repository has completed the migration
When the docs tree is inspected
Then Design Docs, Exec Plans, and Task Specs live under their Specmate
lifecycle paths

Scenario: task specs use four-digit IDs and `.md` filenames
Test: doc review
Given archived and active task specs
When their filenames and frontmatter IDs are reviewed
Then they use `task-0001` style IDs and `.md` filenames

Scenario: entry-point docs and indexes match the migrated layout
Test: doc review
Given a developer starts from `AGENTS.md`
When they follow the documented entry points
Then all referenced managed documents exist at their migrated paths
