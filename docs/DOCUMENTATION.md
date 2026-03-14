# Documentation Guide

This document tells humans and agents where to place documentation in markbase and which document is authoritative when multiple layers exist.

## 1. Current Document Layers

Markbase currently uses this subset of the larger document-driven workflow:

- `ARCHITECTURE.md`: stable system map, module boundaries, invariants, and shared-logic rules
- `docs/core-beliefs.md`: project-level decision rules for ambiguous implementation choices
- `README.md`: user-facing CLI behavior and command contract
- `docs/design-docs/`: current feature or subsystem design contracts
- `docs/exec-plans/active/`: active multi-task execution plans and sequencing
- `docs/exec-plans/completed/`: completed execution plans kept for rationale and history
- `docs/exec-plans/legacy/`: older implementation plans from before the current plan/spec structure
- `specs/active/`: active task-level specs with intent, boundaries, and completion criteria
- `docs/design-docs/legacy/`: older or superseded design references kept for history, not the default active contract

This repo does not currently rely on a required `specs/project.spec` or PRD layer. Do not invent those as part of routine work unless the repo explicitly adopts them.

## 2. Classification Rules

When deciding where a new document belongs, use the first matching rule:

1. If it defines stable cross-module structure or invariants for the whole repo, put it in `ARCHITECTURE.md` or link it from there.
2. If it records project-wide engineering beliefs that guide choices but are not mechanically verifiable, put it in `docs/core-beliefs.md`.
3. If it defines current feature behavior, data shape, interface semantics, or subsystem boundaries that should remain true after implementation, put it in `docs/design-docs/`.
4. If it coordinates multiple tasks for one delivery goal, including order, dependencies, and progress, put it in `docs/exec-plans/active/`.
5. If it is a single task contract with intent, decisions, boundaries, and completion criteria, put it in `specs/active/`.
6. If it explains current user-visible CLI behavior, examples, or flags, update `README.md`.
7. If it is only historical background or has been superseded by a newer design or plan, move it to the matching `legacy/` directory instead of treating it as active guidance.

## 3. Authority And Precedence

When documents overlap, use this order:

1. `specs/active/*.spec` for the specific task they cover
2. `docs/exec-plans/active/` for task ordering, dependencies, and progress
3. `docs/design-docs/` for active feature-level design contracts
4. `ARCHITECTURE.md` and `docs/core-beliefs.md` for global boundaries and decision rules
5. `README.md` for user-facing command behavior
6. `docs/design-docs/legacy/` and `docs/exec-plans/legacy/` for background only

If an active task spec or active exec plan conflicts with a legacy document, follow the active spec/plan and update or archive the outdated legacy doc in the same change.

## 4. Lifecycle Rules

- New active design docs go in `docs/design-docs/` using `design-{id}-{slug}.md`.
- New active exec plans go in `docs/exec-plans/active/` using `exec-{id}-{slug}.md`.
- Completed exec plans move to `docs/exec-plans/completed/` without renaming.
- Historical pre-migration implementation plans stay in `docs/exec-plans/legacy/`.
- Task contracts go in `specs/active/` using `task-{id}-{slug}.spec`.
- Superseded or background-only design docs move to `docs/design-docs/legacy/`.

Prefer moving a document between lifecycle directories over inventing a second file for the same artifact.

## 5. Index Maintenance

When you add or move a document, update the matching entry points in the same change:

- `docs/DESIGN.md` for active or legacy design docs
- `docs/PLANS.md` for exec plan indexes
- `AGENTS.md` when the set of core entry documents changes
- `README.md` when user-visible behavior changes
- `ARCHITECTURE.md` when structural boundaries or documentation roles change

## 6. Agent Shortcut

If asked to "classify" a document, decide in this order:

1. Is it a stable design contract, an execution tracker, or a task acceptance contract?
2. Is it active guidance or historical background?
3. Does it change user-facing behavior, project-wide rules, or only one feature/task?

Then place it in the narrowest authoritative location above and update the relevant index file.
