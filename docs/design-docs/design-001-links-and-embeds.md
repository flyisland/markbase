# Links and Embeds Design

## 1. Purpose

This document defines how markbase interprets Obsidian-style internal links, embeds, and backlinks.

It has two goals:

- stay compatible with the parts of Obsidian link/embed behavior that matter for vault indexing and note rendering
- define the exact normalization and storage rules used by markbase today

This document is intentionally about current markbase behavior and near-term design intent. If Obsidian supports something that markbase does not currently index, that gap is called out explicitly instead of being implied away.

## 2. References

Primary external references:

- [Obsidian Help: Internal links](https://help.obsidian.md/links)
- [Obsidian Help: Embed files](https://help.obsidian.md/embeds)
- [Obsidian Help: Backlinks](https://help.obsidian.md/plugins/backlinks)
- [Obsidian Help: Obsidian Flavored Markdown](https://help.obsidian.md/obsidian-flavored-markdown)
- [Obsidian Help: Advanced formatting syntax](https://help.obsidian.md/advanced-syntax)
- [Obsidian Help: Create a base](https://help.obsidian.md/bases/create-base)
- [Obsidian Help: Accepted file formats](https://help.obsidian.md/file-formats)

These references establish the user-facing syntax that markbase is trying to understand, especially:

- internal links use `[[...]]`
- embeds use `![[...]]`
- embeds can target notes, headings, blocks, and supported file types
- Obsidian Flavored Markdown lists internal links, embed files, and block references as native syntax extensions
- Advanced formatting syntax requires escaping `|` inside table cells when using aliases or embed sizing, for example `[[Note\|Alias]]` and `![[Image.png\|200]]`
- `.base` files are valid Obsidian files and can be embedded with `![[File.base]]`

## 3. Terminology

### 3.1 Link

A **link** is an internal Obsidian reference written with `[[...]]`.

For indexing purposes, markbase also counts `![[...]]` embeds as outgoing links, because an embed is still a reference from one file to another.

### 3.2 Embed

An **embed** is an internal reference written with `![[...]]`.

Obsidian uses embeds for notes, headings, blocks, media files, PDFs, and base files. Markbase stores embeds separately because rendering behavior depends on them.

### 3.3 Backlink

A **backlink** is the reverse of an outgoing link: if file A links to file B, then B has a backlink from A.

Markbase computes backlinks from indexed `links`; it does not parse backlinks directly from source files.

## 4. Obsidian Model vs Markbase Scope

### 4.1 Obsidian syntax markbase treats as in scope

Markbase is designed to understand these Obsidian patterns:

- `[[note]]`
- `[[note#Heading]]`
- `[[note#^blockid]]`
- `[[note|display text]]`
- `[[note#Heading|display text]]`
- `![[note]]`
- `![[note#Heading]]`
- `![[note#^blockid]]`
- `![[image.png]]`
- `![[Document.pdf#page=3]]`
- `![[File.base]]`
- `![[File.base#View]]`

The same patterns may appear inside callouts and other normal Markdown containers, because Obsidian Flavored Markdown treats wikilinks and embeds as Markdown extensions rather than standalone block types.

### 4.2 Obsidian features that are not currently indexed by markbase

Obsidian also supports embedding some remote content using Markdown syntax such as external images. markbase does **not** treat Markdown links or Markdown image syntax as `links` or `embeds`.

Examples outside current indexing scope:

- `[label](target.md)`
- `![alt](https://example.com/image.png)`
- `<iframe ...>`

This is an intentional scope boundary for now: markbase indexes Obsidian-style internal link syntax, not all renderable Markdown relationships.

### 4.3 Frontmatter scope

markbase scans frontmatter string values for `[[...]]` links and includes them in `links`.

markbase does **not** currently treat `![[...]]` found inside frontmatter string values as embeds or links.

This is a project decision, not a claim about all Obsidian behaviors.

## 5. Storage Model

The `notes` table stores three related arrays:

- `links`: normalized outgoing references
- `embeds`: normalized embed references
- `backlinks`: reverse index of `links`

### 5.1 `links`

`links` contains:

- plain wiki-links from the Markdown body
- wiki-links found in frontmatter string values
- embed targets from the Markdown body

In set notation:

```text
links = body_wikilinks ∪ frontmatter_wikilinks ∪ body_embeds
```

### 5.2 `embeds`

`embeds` contains only embed targets from the Markdown body:

```text
embeds = body_embeds
```

### 5.3 `backlinks`

`backlinks` is derived after indexing by reversing `links`:

```text
backlinks(target) = { source_name | source.links contains target }
```

Backlinks are disabled by default at command level unless indexing is run with `--compute-backlinks` or `MARKBASE_COMPUTE_BACKLINKS`.

## 6. Normalization Rules

Normalization is centralized in `Extractor::normalize_link_name()`.

Given a captured target string, markbase applies these rules in order:

1. trim surrounding whitespace
2. strip a trailing `.md` extension if present
3. strip any path prefix before the last `/`
4. strip alias suffix beginning with `|`
5. strip anchor or block suffix beginning with `#`

Examples:

```text
[[design]]                    -> "design"
[[notes/design]]              -> "design"
[[design.md]]                 -> "design"
[[design#Overview]]           -> "design"
[[design|Architecture Doc]]   -> "design"
![[diagram.png]]              -> "diagram.png"
![[Document.pdf#page=3]]      -> "Document.pdf"
![[customer.base#Table]]      -> "customer.base"
```

Consequences of this normalization:

- Markdown note targets are name-based, not path-based.
- Non-Markdown targets keep their file extension.
- Heading, block, PDF page, PDF height, and display-text suffixes are not stored in `links` or `embeds`.

## 7. Extraction Algorithm

Extraction happens in `src/extractor.rs` on a single Markdown file.

### 7.1 Frontmatter parsing

The file is first parsed with `gray_matter`:

- `frontmatter` becomes structured JSON-like data
- `content_without_fm` becomes the Markdown body

If frontmatter parsing fails, markbase falls back to treating frontmatter as null and the full file as body text.

### 7.2 Code block exclusion

Before link extraction, fenced code blocks in the body are replaced with equal-length whitespace placeholders.

This prevents `[[...]]` or `![[...]]` inside fenced code blocks from being indexed as real references.

### 7.3 Embeds first

Embeds are extracted from the body first using `EMBED_RE`.

This matters because the plain wikilink regex would otherwise also match the `[[...]]` portion inside `![[...]]`.

### 7.4 Remove embeds, then extract plain wiki-links

After embed extraction, markbase removes embed matches from the body text and then runs `WIKILINK_RE` over the remaining body.

This is how current code avoids double-counting embeds as both:

- an embed target
- a plain body wikilink

### 7.5 Scan frontmatter string values

markbase recursively scans frontmatter values:

- string values are checked for `[[...]]`
- arrays are traversed recursively
- nested objects are traversed recursively

If a string contains `![[`, markbase skips that string for frontmatter link extraction.

### 7.6 Deduplication

At the end of extraction:

- `links` is sorted and deduplicated
- `embeds` is sorted and deduplicated

This means repeated references only appear once in stored arrays.

## 8. Supported Forms and Stored Results

| Source syntax | Stored in `links` | Stored in `embeds` |
|---|---:|---:|
| `[[note]]` | `note` | no |
| `[[note#Heading]]` | `note` | no |
| `[[note#^blockid]]` | `note` | no |
| `[[note\|alias]]` | `note` | no |
| `[[folder/note]]` | `note` | no |
| `[[note.md]]` | `note` | no |
| `![[note]]` | `note` | `note` |
| `![[note#Heading]]` | `note` | `note` |
| `![[image.png]]` | `image.png` | `image.png` |
| `![[Document.pdf#page=3]]` | `Document.pdf` | `Document.pdf` |
| `![[File.base]]` | `File.base` | `File.base` |
| frontmatter `related: "[[note]]"` | `note` | no |
| frontmatter `embed: "![[note]]"` | no | no |

## 9. OFM-Specific Edge Cases

Obsidian Flavored Markdown adds a few contexts that matter for link parsing.

### 9.1 Links and embeds inside callouts

Links and embeds inside callout bodies should be treated the same as normal body content.

Current markbase behavior already does this because callouts are just ordinary body lines as far as `extractor.rs` is concerned.

### 9.2 Escaped `\|` inside table cells

Obsidian's table syntax requires escaping `|` inside a wikilink alias or embed size expression when they appear in a table cell.

Examples from Obsidian syntax guidance:

```text
[[Note\|Alias]]
![[Image.png\|200]]
```

Current markbase behavior does **not** have special handling for escaped `\|` inside link targets. The normalization logic splits on the first literal `|`, so these table-oriented escaped forms are a known limitation.

This means the current extractor may store an incorrect normalized target for these forms. Until this is fixed, escaped-pipe table forms should be treated as unsupported indexing edge cases.

## 10. Backlink Computation

Backlinks are computed in `scanner.rs`, not in `extractor.rs`.

Algorithm:

1. read all indexed notes
2. read each note's `links`
3. build an in-memory `target -> [source_name, ...]` map
4. write deduplicated backlink arrays back to each note

Important details:

- backlink entries use the **source note name**, not source path
- only indexed note names receive backlinks
- if backlink computation is disabled, existing backlink arrays are cleared

Because backlinks are derived from `links`, embeds also contribute to backlinks.

## 11. Rename Semantics

`src/renamer.rs` rewrites both body links and frontmatter string links across Markdown files.

### 10.1 Files scanned

Current behavior is a full vault scan over `.md` files only.

This is a deliberate correctness choice:

- it does not depend on backlink freshness
- it catches references in files even if the database is stale

### 10.2 What gets rewritten

The rename flow rewrites:

- `[[old]] -> [[new]]`
- `[[old#Heading]] -> [[new#Heading]]`
- `[[old|alias]] -> [[new|alias]]`
- `[[old#Heading|alias]] -> [[new#Heading|alias]]`
- `![[old]] -> ![[new]]`
- `![[old#Heading]] -> ![[new#Heading]]`
- frontmatter string values containing the same `[[...]]` patterns

### 10.3 What is preserved

When the normalized target matches `old_name`, markbase preserves the rest of the original target syntax after the note name:

- heading suffixes
- block suffixes
- display text
- file extensions already present in the original target string

### 10.4 Reindexing after rename

`renamer.rs` itself performs filesystem rewrite and rename only.

The CLI orchestration in `main.rs` runs indexing after rename completes, which rebuilds `links`, `embeds`, and optionally `backlinks`.

## 12. `.base` File Interaction

Obsidian bases are relevant here for two reasons:

- `.base` is an accepted Obsidian file format
- Obsidian allows bases to be embedded with `![[File.base]]` and view-selected with `![[File.base#View]]`

markbase behavior:

- extractor stores `.base` embed targets in `links` and `embeds` using the full filename, for example `customer.base`
- renderer only gives special rendering treatment to body lines that match a standalone `.base` embed line
- non-`.base` embeds are indexed, but `note render` leaves them unchanged in output

This means indexing and rendering have different responsibilities:

- indexing records all embeds uniformly
- rendering has special execution logic only for `.base` embeds

## 13. Current Boundaries and Known Non-Goals

The following are not current design goals for markbase link/embed indexing:

- parsing Markdown inline links as internal note links
- parsing external image/video embeds as `embeds`
- preserving heading or block suffixes in stored `links` / `embeds`
- resolving duplicate note names by path during indexing
- extracting frontmatter `![[...]]` embeds
- correctly parsing escaped `\|` table-cell forms such as `[[Note\|Alias]]` and `![[Image.png\|200]]`

If any of these change, this document, `ARCHITECTURE.md`, and relevant tests should change together.

## 14. Implementation Ownership

Shared ownership boundaries:

- `src/extractor.rs`: canonical target normalization and single-file extraction
- `src/scanner.rs`: persistence of extracted `links` and `embeds`, and derived backlink computation
- `src/renamer.rs`: vault-wide rewrite of link/embed syntax during rename
- `src/renderer/mod.rs`: special runtime handling for `.base` embeds

Architectural rule:

- do not create a second independent definition of link target normalization outside `Extractor::normalize_link_name()`
- if query/render/name-handling behavior depends on link target semantics, it must stay aligned with this document and the extractor implementation
