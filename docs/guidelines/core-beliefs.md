---
title: "Core Beliefs"
---

# Core Beliefs

These beliefs are decision rules for markbase. They are intentionally fewer and more concrete than coding guidelines.

Use them when multiple implementations are possible and the codebase does not already force one answer.

## 1. Files are the product; DuckDB is the index

**Belief**
The vault filesystem is the source of truth. DuckDB exists to accelerate reads, not to hold unique business state.

**Why**
Markbase is valuable only if users can trust that their Markdown files remain primary and recoverable. If important state lives only in the database, reindexing becomes dangerous instead of safe.

**What this means in practice**
- Prefer designs that can be rebuilt from files.
- Treat stale or broken index state as something to recompute, not manually repair.
- Do not introduce features whose true state exists only in `.markbase/markbase.duckdb`.

## 2. Obsidian compatibility wins when abstraction conflicts with user expectation

**Belief**
When internal elegance and Obsidian-compatible behavior conflict, prefer the behavior that matches how users already think about notes, links, embeds, and frontmatter.

**Why**
Markbase is infrastructure around an existing Markdown workflow. If the tool requires users to adopt a different mental model, it becomes harder to trust and harder for agents to use correctly.

**What this means in practice**
- Keep wiki-link behavior aligned with Obsidian conventions.
- Preserve note-name-based linking instead of inventing path-based addressing.
- Treat frontmatter link values and `.base` embed behavior as compatibility surfaces, not implementation details.

## 3. Global note-name uniqueness is a simplifying constraint worth defending

**Belief**
Unique note names across the vault are not a workaround. They are a core simplification that makes the rest of the system predictable.

**Why**
Rename, resolve, backlink computation, and agent automation all become harder once identical names must be disambiguated by path. Hidden disambiguation creates surprising behavior.

**What this means in practice**
- Fail, warn, or skip when names collide; do not silently choose one.
- Keep note-facing commands centered on names, not relative paths.
- Treat proposals that weaken uniqueness as architectural changes, not local tweaks.

## 4. Friendly syntax is good; hidden translation rules are not

**Belief**
Markbase should offer concise user syntax, but every shortcut must translate to a stable and explainable rule.

**Why**
The tool serves both humans and agents. Convenience helps only when the translation is predictable across commands and modules.

**What this means in practice**
- Bare query identifiers may be shorthand, but their meaning must stay consistent.
- If two modules interpret the same syntax differently, the syntax is wrong or underspecified.
- Avoid adding magic behavior that cannot be explained by a small number of explicit translation rules.

## 5. Keep pure logic separate from command orchestration

**Belief**
Parsing, translation, normalization, and validation should stay as stateless and local as practical. Command modules should make side effects obvious.

**Why**
Markbase changes often at the edges: new query behavior, new render behavior, new validation rules. Those changes are safer when pure transformations can be tested independently from filesystem and database writes.

**What this means in practice**
- Put CLI flow, indexing triggers, and output routing in orchestrators such as `main.rs`.
- Keep parser and translator modules focused on input-to-output transformation.
- Do not hide writes inside modules whose public role sounds read-only.

## 6. Default outputs should favor agents; human views should be explicit

**Belief**
The default interface should be stable and machine-friendly. Human-readable formatting is important, but it should be an explicit presentation choice.

**Why**
Markbase is used as tooling infrastructure. Agent workflows and shell pipelines depend on outputs that are easier to parse than terminal-oriented summaries.

**What this means in practice**
- Prefer stable structured output as the default contract.
- Keep table rendering and other display-oriented formats opt-in.
- Treat output shape as part of the public interface, not cosmetic formatting.

## 7. When a failure mode is repeatable, fix the harness, not just the instance

**Belief**
A bug class is not fully solved until the repo makes it harder to reintroduce.

**Why**
Markbase is maintained by both humans and agents. If correctness depends on someone remembering a subtle rule, the rule will be broken again.

**What this means in practice**
- Add or strengthen tests when fixing regressions.
- Update the relevant spec or README when behavior changes.
- Prefer repository-level guidance and executable checks over one-off review comments.
