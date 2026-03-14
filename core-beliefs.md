# Core Beliefs

These are markbase's global design beliefs. They are not lint rules. They exist to guide decisions when the codebase does not already dictate a single obvious move.

## 1. Human intent must become repository context

If a decision only lives in chat, memory, or oral tradition, it does not exist for an agent and is fragile even for humans.

Write down stable constraints in docs, encode repeatable checks in tests, and keep user-visible behavior in `README.md`. Prefer improving the harness over repeating the same correction in review.

## 2. The job is to maintain a reliable index, not a hidden application state

Markbase is a Markdown-first system. Files remain the product. DuckDB is an acceleration layer.

When forced to choose, preserve filesystem truth and rebuildability over clever database-only state. A corrupted or stale index should be recoverable by reindexing, not by manual surgery.

## 3. Obsidian compatibility beats internal elegance

The tool exists inside an existing note-taking ecosystem. Users think in note names, wiki-links, frontmatter, and vault structure.

Prefer behavior that matches Obsidian mental models, even when a path-aware or schema-heavy internal model would look cleaner from a pure engineering perspective.

## 4. Name uniqueness is a feature, not a limitation

Global note-name uniqueness simplifies linking, renaming, resolving, and agent usage.

Do not patch around duplicate names with hidden heuristics. Ambiguity should surface as an explicit warning or failure because silent disambiguation makes both human and agent behavior less predictable.

## 5. Keep pure transformations separate from orchestration

Parsing, translation, validation, and formatting logic should stay as stateless as practical. Command orchestration should be explicit about when it reads, writes, indexes, or prints.

This separation keeps behavior testable and makes it easier for agents to modify one layer without breaking another.

## 6. Friendly syntax is allowed; hidden semantics are not

Markbase can offer ergonomic user syntax such as bare frontmatter fields, but the internal translation must stay explicit and unsurprising.

A user-friendly command is good only if its behavior is mechanically explainable. If a shortcut makes translation ambiguous or inconsistent across modules, the shortcut is wrong.

## 7. Default to safe, narrow authority

Read-only features should stay read-only. Write paths should be obvious, validated, and easy to audit.

If a module's name suggests inspection, query, render, verify, or describe, it should not secretly mutate the vault or database. Side effects belong in narrow orchestrators and command modules.

## 8. Performance comes from incrementalism and restraint

For a local CLI, predictable performance matters more than theoretical sophistication.

Prefer incremental scans, bounded parsing, and low dependency overhead before adding concurrency, caches, or abstraction layers. Simpler systems are easier to keep fast and easier for agents to extend safely.

## 9. Machine-friendly output is the default; human-friendly views are opt-in

Markbase is infrastructure for scripts and agents as much as for humans.

Default outputs should stay structured and stable. Human-oriented tables and summaries are valuable, but they should be explicit formatting choices rather than accidental coupling to terminal presentation.

## 10. Error messages are part of the interface

A failing command should teach the user what assumption was violated.

Prefer actionable errors that identify the file, field, name, or rule involved. Silent fallback and vague failures create more operational cost than strict rejection.

## 11. Mechanical verification beats subjective confidence

A change is not "done" because it looks plausible. It is done when the intended behavior is documented, exercised, and verified.

When you find a repeatable failure mode, add or strengthen a test, spec, or validation step so the system learns once. Do not rely on future reviewers noticing the same class of mistake again.

## 12. Optimize for agent legibility, not just human cleverness

The best implementation is one whose boundaries, invariants, and failure modes are obvious from the repository.

Prefer direct module responsibilities, explicit contracts, and small surfaces over dense abstractions that require insider knowledge. A legible codebase improves both autonomous execution and human maintenance.
