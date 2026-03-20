---
id: design-008
title: "Note Resolve"
status: implemented
module: resolver
---

# `note resolve` Design

**Status:** Implemented  
**Target:** markbase CLI  
**Related docs:** `ARCHITECTURE.md`, `README.md`, `docs/design-docs/implemented/design-005-indexing.md`, `docs/design-docs/implemented/design-001-links-and-embeds.md`

## Scope

`markbase note resolve` resolves one or more note-facing identifiers against the derived index.

It exists to provide a cheap, structured lookup path for note reuse and agent alignment without rendering note bodies.

This document covers:

- CLI input-shape rules for `note resolve`
- automatic index refresh before lookup
- exact-name, alias, and deterministic partial-name matching against the indexed `notes` table
- JSON output shape, ordering, and status classification

This document does not define:

- path-based HTTP route resolution for web views
- render-time embed resolution
- rename-time link rewriting
- fuzzy search, semantic search, or alias partial matching

## Command Contract

```bash
markbase note resolve "acme"
markbase note resolve "张伟" "阿里"
```

Each input must be a path-free identifier without file extension.

Execution is split across:

- `src/main.rs`: CLI validation, index orchestration, JSON output
- `src/name_validator.rs`: shared resolve-input validation
- `src/resolver.rs`: matching, ordering, and status classification
- `src/db.rs`: SQL execution against the derived `notes` table

The command is read-only with respect to vault content. It may refresh the derived index before lookup.

## Input Rules

`validate_resolve_input()` applies the same path-free naming model used by other note-facing commands.

Current rules:

- empty input is rejected
- directory components are rejected
- any file extension is rejected

Examples that fail before indexing or lookup:

- `logs/acme`
- `../acme`
- `acme.md`

In a multi-query invocation, all inputs are validated first. If any one input is invalid, the entire command fails and emits no partial JSON results.

Because non-Markdown indexed resources keep their extension as part of logical identity, the extension ban means `note resolve` only targets Markdown-note identities. It does not resolve `.base` files or attachments.

## Index And Lookup Boundary

`main.rs` runs the normal DB-backed command flow:

1. validate all resolve inputs
2. refresh the index with `ensure_index_ready(...)`
3. call `resolver::resolve_names(...)`
4. pretty-print the JSON result array to stdout

`note resolve` depends on the indexing contract from `docs/design-docs/implemented/design-005-indexing.md`:

- Markdown note identity is filename without `.md`
- global note-name uniqueness is enforced during indexing by keeping the first indexed path and skipping later collisions
- aliases live in frontmatter-derived `properties`

As a result, exact-name resolution is name-based, not path-based.

## Matching Contract

For each query string `q`, resolver executes one indexed lookup with this logical shape:

- match rows where `notes.name == q`, case-insensitively
- also match rows where `q` is contained in frontmatter `aliases`, case-insensitively
- also match rows where `notes.name` contains `q`, case-insensitively, and `notes.name != q` under the same case-insensitive comparison
- also match rows where `q` contains `notes.name`, case-insensitively, and `notes.name != q` under the same case-insensitive comparison
- mark exact-name rows as `matched_by = name`
- mark alias rows as `matched_by = alias`
- mark `notes.name LIKE %q%` rows as `matched_by = name_contains_query`
- mark `q LIKE %notes.name%` rows as `matched_by = query_contains_name`
- if one row satisfies more than one rule, keep one row and assign the highest-priority `matched_by`
- sort rows by match priority: `name`, then `alias`, then `name_contains_query`, then `query_contains_name`
- within the same match source, sort by absolute length distance between `name` and `q`, then `name`, then `path`

Current matching is:

- case-insensitive equality for `name` and `alias`
- deterministic case-insensitive substring matching for `notes.name` only
- whitespace-sensitive except for whatever normalization already happened when the note was indexed
- limited to the current contents of the derived index

There is no fuzzy matching, path fallback, extension stripping, alias normalization, alias partial matching, or semantic ranking in the resolver itself.

## Output Contract

The command prints a JSON array in the same order as the input arguments.

Each element has this top-level shape:

```json
{
  "query": "acme",
  "status": "exact",
  "matches": [
    {
      "name": "acme",
      "path": "companies/acme.md",
      "type": "company",
      "description": "Smart home customer",
      "matched_by": "name"
    }
  ]
}
```

Top-level fields:

- `query`: the original input string
- `status`: `exact`, `alias`, `name_contains_query`, `query_contains_name`, `multiple`, or `missing`
- `matches`: ordered candidate list

Each match contains:

- `name`: indexed logical note name
- `path`: indexed `file.path`
- `description`: string or `null`
- `matched_by`: `name`, `alias`, `name_contains_query`, or `query_contains_name`

`type` is currently best-effort:

- if frontmatter `type` is present as a string-like value extractable by `json_extract_string(...)`, it is emitted
- if `type` is missing at serialization time, the field is omitted from that match

`description` differs from `type` in one important way:

- `description` is always serialized
- missing or empty-string descriptions are emitted as `null`

This stable `description` key is covered by tests in both `src/resolver.rs` and `tests/cli_note.rs`.

## Status Classification

Status is classified per input after SQL rows are decoded into ordered matches.

Current rules:

- `missing`: no matches
- `exact`: exactly one match and it came from `name`
- `alias`: exactly one match and it came from `alias`
- `name_contains_query`: exactly one match and it came from `name_contains_query`
- `query_contains_name`: exactly one match and it came from `query_contains_name`
- `multiple`: more than one match, regardless of source mix

With the expanded matching contract, status classification remains source-driven for single-match results and count-driven for multi-match results.

## Current Resolution Flow

### 1. Validation

`main.rs` validates every CLI argument with `validate_resolve_input()`.

These validation failures are command errors, not JSON `status` values.

### 2. Index refresh

After validation succeeds, `main.rs` refreshes the derived index before resolution.

This means resolver sees the latest indexed state of:

- note names
- frontmatter aliases
- frontmatter `type`
- frontmatter `description`

### 3. Query execution

`resolver::resolve_names()` validates again defensively, then resolves each input independently with `resolve_name()`.

Each query becomes one SQL statement against the `notes` table that reads:

- `path`
- `name`
- `properties.type`
- `properties.description`
- a computed `matched_by`

The query compares `name`, `aliases`, and partial-name candidates with case-insensitive string matching while preserving the original stored casing in returned `name`, `path`, `type`, and `description` fields.

### 4. Row normalization

Resolver normalizes extracted scalar strings with `normalize_optional()`.

Current normalization rules treat these values as missing:

- empty string after `trim()`
- case-insensitive textual `"null"`

All other values are preserved as strings.

### 5. JSON serialization

`main.rs` pretty-prints the final `Vec<ResolveResult>` with `serde_json::to_string_pretty(...)`.

Successful resolution always exits through stdout JSON, including `missing` and `multiple` statuses.

## Decision Table

| Condition | Command exit | JSON emitted | Per-query status |
| --- | --- | --- | --- |
| Any CLI input is empty, path-like, or has an extension | non-zero | no | none |
| Query returns zero rows | zero | yes | `missing` |
| Query returns one exact-name row | zero | yes | `exact` |
| Query returns one alias row | zero | yes | `alias` |
| Query returns one `notes.name` row where `name` contains query | zero | yes | `name_contains_query` |
| Query returns one `notes.name` row where query contains `name` | zero | yes | `query_contains_name` |
| Query returns two or more rows | zero | yes | `multiple` |
| DB/index error during setup or query | non-zero | no | none |

## Known Current Limits

These are descriptions of current behavior, not proposals:

- exact matching relies entirely on indexed note names and aliases; there is no secondary lookup against note body text
- alias collisions are allowed and surface as `multiple`
- partial matching applies only to `notes.name`, never to `aliases`
- the command does not score or rank candidates beyond match-priority ordering and deterministic tie-breakers
- `type` is not shape-stable in JSON because missing values are omitted instead of emitted as `null`
- resolver does not perform semantic validation of whether a returned candidate is actually the intended real-world entity; callers must compare returned context such as `description`

## References

- `src/main.rs`
- `src/resolver.rs`
- `src/name_validator.rs`
- `tests/cli_note.rs`
