---
id: project
status: active
---

# Project Constraints

This file records project-wide technical constraints for markbase.

- The vault filesystem under `MARKBASE_BASE_DIR` is the durable source of truth.
- DuckDB is a derived index and must remain rebuildable.
- Note-facing identities remain path-free and basename-oriented.
- Managed documentation follows `docs/design-docs/implemented/design-009-document-system.md`.
