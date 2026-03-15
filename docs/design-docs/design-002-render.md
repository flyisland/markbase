---
id: design-002
title: "Render Subsystem"
status: active
type: design
module: renderer
---

# Render Subsystem

## Purpose

The render subsystem exists to give agents and scripts a read-only way to expand a note into a richer working view without changing vault files.

Its primary job is to render Markdown note bodies to stdout, expand eligible Markdown note embeds into rendered note bodies, and replace eligible `.base` embeds with database-backed query results derived from the indexed vault. It is not a write path, it does not introduce new durable state, and it must remain rebuildable from the filesystem plus the derived DuckDB index.

## Interface Contract

### CLI Surface

The public command surface is:

```bash
markbase note render <name> [-o json|table] [--dry-run]
```

`<name>` must be a note-facing target name, not a path:

- Markdown notes are addressed by note name without extension
- Base files are addressed by full `.base` filename
- path-like input is rejected
- non-`.base` non-Markdown filenames are rejected

### Supported Targets

`note render` only supports indexed `.md` notes and indexed `.base` files.

- Rendering a Markdown note reads its body and recursively expands eligible Markdown note embeds and eligible `.base` embeds
- Rendering a `.base` file skips body scanning and directly renders that Base file's views
- Other indexed resource types are not valid render targets

### Default Output And Wrappers

The default render format is `json`.

- `-o json` is a first-class supported format
- default output for each rendered Base view is a fenced JSON block embedded in Markdown
- `-o table` renders each Base view as a compact Markdown table for human inspection

Each rendered or dry-run Base section uses stable wrappers:

````md
<!-- start: [markbase] rendered from tasks.base -->

> **Open Tasks**

```json
[
  {
    "name": "[[task-a]]"
  }
]
```

<!-- end: [markbase] rendered from tasks.base -->
````

Dry-run output keeps the same section structure but replaces query results with a fenced SQL block and uses `dry-run` in the wrapper text.

### Markdown Note Embed Execution Rule

Within Markdown note bodies, render-time note expansion only happens when all of the following are true:

- the token is an Obsidian embed token, not a plain link
- the normalized target refers to a Markdown note identity
- the embed does not include an anchor selector after `#`

As a consequence:

- `![[note1]]` expands to the rendered body of `note1`
- `![[note1|Alias]]` expands the same rendered body; aliases do not change runtime render behavior
- embedded Markdown note rendering uses the embedded note's Markdown body after frontmatter removal; embedded frontmatter is never emitted into render output
- `Before![[note1]]After` becomes:

```md
Before
<rendered body of note1>
After
```

- note embed output is block-oriented: render always inserts a line break before and after the expanded note body, even when the token appears inline with surrounding text
- the expanded note body is emitted without an extra wrapper, title, or provenance comment
- embedded note rendering reuses the same Markdown-body scan rules as top-level note rendering, so nested note embeds and `.base` embeds inside the embedded note body are expanded recursively
- recursive note rendering rebinds the render-note context at each note boundary; when a nested `.base` embed executes inside an embedded note body, `this` refers to that embedded note rather than the original top-level render target
- when a note embed appears inside a blockquote, list item, or callout body, the emitted multi-line note body does not preserve that container prefix on each line
- this means embedded note output may break out of the original Markdown container; callers that need stable Markdown structure should place note embeds on ordinary body lines

This contract is based on shared token scanning plus render-time target classification, not on a renderer-specific regex.

### Recursive Expansion And Cycle Guard

Recursive note expansion is part of the active render contract.

- render descends depth-first through embedded Markdown notes
- each recursive note render uses the same body-scanning rules, `.base` execution rules, output format, and `--dry-run` behavior as a top-level note render
- each recursive note render also establishes a new current-note `this` context for any `.base` embeds encountered within that embedded note body
- recursion is guarded by the active Markdown note render stack, keyed by normalized note name
- if an embedded note target is already on the active stack, render must not recurse into it again
- cycle detection is a soft failure: render writes a warning to stderr and emits a placeholder comment at that output position, then continues rendering the rest of the document

The current cycle warning contract is:

```text
WARN: recursive note embed skipped for 'note1' to avoid cycle.
```

The current cycle placeholder contract is:

```md
<!-- [markbase] recursive note embed skipped for 'note1' -->
```

The current missing embedded-note warning contract is:

```text
WARN: embedded note 'note1' not found in index, skipping.
```

The current missing embedded-note placeholder contract is:

```md
<!-- [markbase] note 'note1' not found -->
```

The current embedded-note read-failure warning contract is:

```text
WARN: failed to read 'note1': <os error text>
```

The current embedded-note read-failure placeholder contract is:

```md
<!-- [markbase] failed to read 'note1' -->
```

### `.base` Embed Execution Rule

Within Markdown note bodies, render-time Base expansion only happens when all of the following are true:

- the token is an Obsidian embed token, not a plain link
- the normalized target ends with `.base`

As a consequence:

- `![[tasks.base]]` on its own line is rendered
- `Before ![[tasks.base]] After` is rendered, with the Base output inserted at the embed position
- blockquotes, list items, and callout bodies are eligible if the embed appears in normal Markdown body content rather than code context
- surrounding text remains in output; render may split the original line with inserted block output when needed
- `.base` embeds found inside recursively expanded note bodies are rendered using the same rules as `.base` embeds in the top-level note body
- when an embed appears inside a blockquote, list item, or callout body, the rendered Base block does not inherit or preserve that container prefix on each emitted line
- this means the Base output may visually break out of the original Markdown container; this is accepted behavior, not a render bug
- callers that need stable Markdown structure should place `.base` embeds on ordinary body lines rather than inside blockquote/list/callout-prefixed lines

This contract is based on shared token scanning, not on a renderer-specific regex.

### `.base#View` Selector Rule

`![[File.base#View Name]]` is supported as a render selector.

- the selector matches the Base view `name` field using case-sensitive exact matching
- when the selector matches, only that single view is rendered
- when the selector does not match any view, render writes a warning to stderr and emits an HTML placeholder comment at that output position
- render must not fall back to other views when a selector is present but unresolved

The current placeholder contract is:

```md
<!-- [markbase] view 'Missing View' not found in 'tasks.base' -->
```

### Code-Context Exclusion

Render-time body scanning for Markdown note embeds and `.base` embeds must consume the shared `ScanContext::MarkdownBody` contract from `src/link_syntax.rs`.

That means:

- fenced code blocks are never treated as live Markdown note embeds or live `.base` embeds
- inline code spans are never treated as live Markdown note embeds or live `.base` embeds
- ordinary Markdown body content is scanned normally

### Other Embed Targets

Embeds that are not eligible Markdown note embeds or eligible `.base` embeds remain literal output.

- non-Markdown, non-`.base` embeds may still be indexed as embeds by the indexing pipeline, but `note render` does not give them special runtime treatment
- note embeds with a heading or block selector, such as `![[note#Heading]]` and `![[note#^blockid]]`, are not part of the current render contract and remain literal output
- the original token text is preserved in output for these pass-through cases

### `--dry-run`

`--dry-run` preserves render structure but does not execute Base queries.

- the database must still be available so render can resolve targets and build SQL
- note/body/Base parsing still occurs
- each selected view emits the translated SQL instead of query results

### Errors And Warnings

Hard errors terminate the command with non-zero exit status. Current hard-error cases include:

- target note or base file not found as a render target
- unsupported render target extension
- top-level render target file read failure

Soft failures stay within the rendered stream and report warnings on stderr. Current soft-failure cases include:

- embedded Markdown note not found in the index
- embedded Markdown note file read failure
- recursive note embed cycle detected
- embedded Base file not found in the index
- embedded Base file read failure
- embedded Base YAML parse failure
- selected Base view not found
- per-view query translation or execution warnings

For soft failures, render preserves positional context by emitting a placeholder comment or partial section output instead of aborting the entire note render.

## Subsystem Design

### Entry And Orchestration

`src/main.rs` owns the CLI-facing orchestration:

- parse `note render` arguments
- validate render target naming rules before execution
- ensure the index is current for normal execution
- open the existing database directly in `--dry-run` mode
- map shared CLI output options onto renderer-specific formats
- route final hard failures to stderr and process exit

### Render Driver

`src/renderer/mod.rs` owns the runtime render flow:

- dispatch between note rendering and direct `.base` rendering
- fetch the target note record that defines the `this` context
- read Markdown note content or Base file content from disk
- scan body tokens using shared link/embed parsing
- recursively render eligible embedded Markdown notes while maintaining a cycle-guard stack
- identify `.base` embed tokens that should be expanded
- resolve embedded Base files via indexed `notes` entries
- select views, build SQL, execute or dry-run them, and print wrapped output

This module is an orchestration layer. It may compose filesystem reads and DB reads, but it must not hide writes.

### Filter Translation

`src/renderer/filter.rs` translates Base filter and sort configuration into DuckDB SQL fragments.

Its responsibilities include:

- translating Base filter objects and string predicates
- materializing the `this` context used by expressions such as `link(this)`
- translating column selections and sort clauses
- preserving shared namespace semantics for bare fields, `file.*`, and `note.*`

This translator is coupled to query semantics by contract. If field resolution changes in query translation, renderer filter translation must be updated in the same change.

### Output Formatting

`src/renderer/output.rs` shapes row-oriented query results into stable renderer output formats.

- JSON output is agent-first and uses shared scalar/list formatting rules
- table output is an explicit human-facing presentation mode
- output formatting stays downstream of SQL execution and does not change selection semantics

### Dependency Boundaries

The renderer depends on shared system modules but should not absorb their responsibilities:

- `src/db.rs` provides SQL execution and row retrieval
- `src/link_syntax.rs` defines body scanning and target normalization
- `src/name_validator.rs` owns note-facing target validation
- `src/renderer/filter.rs` and `src/renderer/output.rs` remain renderer-local helpers

The renderer must not:

- reimplement link/embed parsing separately from `src/link_syntax.rs`
- introduce hidden writes to the vault or database
- move CLI argument handling out of `src/main.rs`

## Key Decisions

- Render is a read-only, DB-backed expansion path. It consumes the derived index and vault files but does not create durable business state.
- Markdown note embeds are expanded as raw rendered note bodies, not as links, wrappers, or titled sections.
- Recursive note expansion is depth-first and must be cycle-safe; repeating an active note target is a soft failure, not a hard abort.
- `this` is note-local during recursive rendering: nested `.base` execution uses the note currently being expanded, not the original top-level note.
- Embedded Base lookup uses indexed `notes` entries, not filesystem globbing. This keeps lookup aligned with markbase's indexed-name model.
- Body scanning reuses `ScanContext::MarkdownBody` from `src/link_syntax.rs` so render, indexing, and rename stay aligned on code-context exclusion.
- Renderer filter translation must preserve the same bare-field, `file.*`, and `note.*` meaning as the query translator.
- Direct `.base` rendering is a supported mode. In that mode, the `.base` file itself defines the `this` context and its views are rendered directly.
- Richer Obsidian Base presentation fields that are not part of current execution semantics may be ignored. markbase only executes the subset already implemented as query/filter/output behavior.
- View title formatting is currently part of the stable render wrapper shape: rendered views use `> **View Name**` between the section wrapper and the rendered content.
- Container-prefixed Markdown lines are scanned for `.base` embeds, but render does not attempt container-aware rewrapping of the emitted multi-line Base output. The complexity is not justified for a rare and structurally awkward authoring pattern.

## Constraints

These constraints should be inherited by future Task Specs that touch render behavior:

- Do not introduce writes into render paths.
- Do not fork link/embed parsing away from `src/link_syntax.rs`.
- Do not change renderer filter namespace semantics without matching query-layer changes.
- Do not implement recursive note expansion without an explicit cycle guard keyed by normalized note name.
- Do not reintroduce a whole-line-only restriction for `.base` embed expansion unless the contract, README, and tests change together.
- Do not regress default output from JSON back to legacy list-style output.
- Do not treat note-embed newline splitting, recursive note expansion, `.base#View`, code-context exclusion, or pass-through handling for unsupported embeds/selectors as optional behavior; they are part of the active contract.

## Relationship To Other Docs

- `ARCHITECTURE.md` defines system-level boundaries, ownership, and invariants for rendering as part of the larger markbase architecture.
- `docs/design-docs/design-001-links-and-embeds.md` defines the shared parsing and normalization contract that render consumes for body scanning and `.base#View` selector handling.
- `README.md` documents the user-visible command behavior and examples, but does not repeat renderer-internal module boundaries.
- `docs/design-docs/legacy/note_render_design.md` remains historical context only. When it differs from current implementation, this document is the active contract.

## Test And Acceptance Mapping

The following tests anchor the active render contract and should remain aligned with this document:

- `test_note_render_accepts_base_filename`
- `test_note_render_base_embed_with_view_selector`
- `test_note_render_inline_base_embed_is_expanded`
- `test_note_render_base_embed_missing_view_selector`
- `test_note_render_non_base_embed_passthrough_after_parser_change`
- `test_note_render_base_embed_inside_fenced_code_is_not_expanded`
- `test_note_render_dry_run`
- `test_note_render_table_format`
- `test_render_view_selector_matches_documented_behavior`

When recursive note embed support is implemented, add or update coverage for:

- inline note embeds that split surrounding text onto separate lines
- nested note embeds that also expand nested `.base` embeds
- cycle detection that emits the documented placeholder and warning without aborting the whole render

If this document and executable behavior diverge, resolve the mismatch against current code, tests, and README, then update the stale side explicitly rather than preserving legacy wording.

## Change Log

### 2026-03-14: Replace legacy render design as the active contract

The older `legacy/note_render_design.md` captured an earlier render design centered on list-style output and pre-unification parsing assumptions. Since then, the active implementation has moved to:

- JSON as the default structured output
- explicit `-o json` and `-o table` support
- direct `.base` render targets
- shared link/embed scanning for token-position `.base` detection
- formal `.base#View` selection behavior
- code-context exclusion aligned with the shared parser

### 2026-03-14: Add recursive Markdown note embed rendering to the active contract

The active render design now also requires:

- render-time expansion of whole-note Markdown embeds such as `![[note1]]`
- block-oriented insertion for inline note embeds, so surrounding text is split onto separate lines
- recursive expansion of nested note and `.base` embeds inside embedded note bodies
- cycle detection based on the active note render stack, with warning-plus-placeholder soft-failure behavior

Because the legacy document no longer matches the shipped implementation and regression tests, `design-002-render.md` is now the active render contract and the legacy document remains historical reference only.
