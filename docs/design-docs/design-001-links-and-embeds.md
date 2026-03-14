# Links and Embeds Design

## 1. Purpose

This document defines how markbase interprets Obsidian-style internal links, embeds, and backlinks.

It has two goals:

- stay compatible with the parts of Obsidian link/embed behavior that matter for vault indexing and note rendering
- define the exact normalization and storage rules used by markbase

This document is the behavioral contract for link/embed parsing. If current implementation differs, the contract in this document wins and the implementation must be updated to match.

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

## 6. Shared Parser Contract

Link and embed parsing is centralized in a shared module, `src/link_syntax.rs`.

The rest of the system must consume this shared contract rather than reimplementing independent parsing rules.

### 6.1 Required API shape

The shared module must expose at least:

- `scan_link_tokens(input: &str, context: ScanContext) -> Vec<LinkToken>`
- `parse_link_target(raw_inner: &str) -> ParsedTarget`

Required `ScanContext` values:

- `MarkdownBody`
- `FrontmatterString`

Required `LinkToken` fields:

- `kind`: `WikiLink` or `Embed`
- `full_span`: byte range of the complete token including `[[...]]` or `![[...]]`
- `inner_span`: byte range of the content inside the brackets
- `raw_inner`: the original bracket contents without outer delimiters
- `parsed`: the parsed target data

Required `ParsedTarget` fields:

- `normalized_target`: logical target name used by indexing, rename matching, verification, and `.base` lookup
- `target_text`: target portion before alias and before anchor normalization
- `anchor`: optional heading / block / view selector text without the leading `#`
- `alias_or_size`: optional alias or embed size text without the leading `|`
- `is_markdown_note`: whether the normalized target refers to a Markdown note identity

### 6.2 Body scanning rules

In `MarkdownBody` context:

- ordinary Markdown containers are scanned normally, including paragraphs, list items, blockquotes, and callout bodies
- fenced code blocks delimited by either ``` or `~~~` are ignored
- inline code spans delimited by matching backtick runs are ignored
- unclosed `[[` / `![[` sequences are ignored

In `FrontmatterString` context:

- the full string is scanned
- there is no code-context suppression

### 6.3 Target parsing rules

Given `raw_inner`, markbase parses it in this order:

1. trim surrounding whitespace
2. split on the first **unescaped** `|` to produce `alias_or_size`
3. split the pre-alias portion on the first **unescaped** `#` to produce `anchor`
4. strip any path prefix before the last `/` from the target portion
5. if the remaining basename ends with `.md`, strip `.md`

Important escape rule:

- `\|` is treated as a Markdown table-cell escape, not as part of the logical target name
- therefore `[[Note\|Alias]]` is semantically equivalent to `[[Note|Alias]]`
- and `![[Image.png\|200]]` is semantically equivalent to `![[Image.png|200]]`

Examples:

```text
[[design]]                    -> "design"
[[notes/design]]              -> "design"
[[design.md]]                 -> "design"
[[design#Overview]]           -> "design"
[[design|Architecture Doc]]   -> "design"
[[design.md#Overview|Doc]]    -> "design"
[[Note\|Alias]]               -> "Note"
![[diagram.png]]              -> "diagram.png"
![[Image.png\|200]]           -> "Image.png"
![[Document.pdf#page=3]]      -> "Document.pdf"
![[customer.base#Table]]      -> "customer.base"
```

Consequences of this normalization:

- Markdown note targets are name-based, not path-based.
- Non-Markdown targets keep their file extension.
- Heading, block, PDF page, PDF height, and display-text suffixes are not stored in `links` or `embeds`.

## 7. Extraction Algorithm

Extraction happens in `src/extractor.rs` on a single Markdown file and must consume the shared parser contract from `src/link_syntax.rs`.

### 7.1 Frontmatter parsing

The file is first parsed with `gray_matter`:

- `frontmatter` becomes structured JSON-like data
- `content_without_fm` becomes the Markdown body

If frontmatter parsing fails, markbase falls back to treating frontmatter as null and the full file as body text.

### 7.2 Scan body tokens

The Markdown body is scanned once with `ScanContext::MarkdownBody`.

For each returned token:

- `WikiLink` contributes its `normalized_target` to `links`
- `Embed` contributes its `normalized_target` to both `links` and `embeds`

The extractor must not run a second independent wiki-link pass after embed extraction.

### 7.3 Scan frontmatter string values

markbase recursively scans frontmatter values:

- string values are checked for `[[...]]`
- arrays are traversed recursively
- nested objects are traversed recursively

Each frontmatter string is scanned with `ScanContext::FrontmatterString`.

In frontmatter strings:

- `WikiLink` tokens contribute their `normalized_target` to `links`
- `Embed` tokens are ignored
- the presence of `![[` must **not** cause the whole string to be skipped

### 7.4 Deduplication

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
| `[[note.md#Heading|alias]]` | `note` | no |
| `![[note]]` | `note` | `note` |
| `![[note#Heading]]` | `note` | `note` |
| `![[Image.png\|200]]` | `Image.png` | `Image.png` |
| `![[image.png]]` | `image.png` | `image.png` |
| `![[Document.pdf#page=3]]` | `Document.pdf` | `Document.pdf` |
| `![[File.base]]` | `File.base` | `File.base` |
| `![[File.base#View]]` | `File.base` | `File.base` |
| frontmatter `related: "see [[note]] and ![[ignored]]"` | `note` | no |
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

markbase must parse these forms with the same logical meaning as their unescaped variants:

- `[[Note\|Alias]]` is a link to `Note` with alias `Alias`
- `![[Image.png\|200]]` is an embed of `Image.png` with size `200`

The backslash exists only to survive Markdown table parsing; it does not become part of the logical target.

### 9.3 Inline code and fenced code

Links and embeds inside Markdown body code contexts are ignored:

- fenced code blocks delimited by ``` or `~~~`
- inline code spans delimited by matching backtick runs

This exclusion applies to indexing and rename scanning. It does not apply to frontmatter string scanning.

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

### 11.1 Files scanned

Current behavior is a full vault scan over `.md` files only.

This is a deliberate correctness choice:

- it does not depend on backlink freshness
- it catches references in files even if the database is stale

### 11.2 What gets rewritten

The rename flow rewrites:

- `[[old]] -> [[new]]`
- `[[old#Heading]] -> [[new#Heading]]`
- `[[old|alias]] -> [[new|alias]]`
- `[[old#Heading|alias]] -> [[new#Heading|alias]]`
- `![[old]] -> ![[new]]`
- `![[old#Heading]] -> ![[new#Heading]]`
- frontmatter string values containing the same `[[...]]` patterns

### 11.3 What is preserved

When the normalized target matches `old_name`, markbase preserves the rest of the original target syntax after the note name:

- heading suffixes
- block suffixes
- display text
- embed size suffixes

For Markdown note targets, rewritten syntax is normalized to the canonical external form:

- no path prefix
- no `.md` extension

Examples:

```text
[[folder/old.md#Section]]     -> [[new#Section]]
[[old#Heading|Alias]]         -> [[new#Heading|Alias]]
![[old.base#Open Tasks]]      -> ![[new.base#Open Tasks]]
```

### 11.4 Reindexing after rename

`renamer.rs` itself performs filesystem rewrite and rename only.

The CLI orchestration in `main.rs` runs indexing after rename completes, which rebuilds `links`, `embeds`, and optionally `backlinks`.

## 12. `.base` File Interaction

Obsidian bases are relevant here for two reasons:

- `.base` is an accepted Obsidian file format
- Obsidian allows bases to be embedded with `![[File.base]]` and view-selected with `![[File.base#View]]`

markbase behavior:

- extractor stores `.base` embed targets in `links` and `embeds` using the full filename, for example `customer.base`
- renderer only gives special rendering treatment to body lines whose **trimmed content** is exactly one `.base` embed token
- non-`.base` embeds are indexed, but `note render` leaves them unchanged in output

For `.base#View` rendering:

- the view selector is the embed anchor text after `#`
- matching is case-sensitive and exact
- when a view selector exists, only the selected view is rendered
- if the selector is missing, markbase must not fall back to rendering all views

Required failure output for missing view:

- stderr:
  `WARN: view '<view-name>' not found in '<base-name>', skipping.`
- stdout:
  `<!-- [markbase] view '<view-name>' not found in '<base-name>' -->`

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
- rendering `.base` embeds when the line also contains blockquote/list/callout marker prefixes
- supporting self-relative forms such as `[[#Heading]]` and `[[^blockid]]`

If any of these change, this document, `ARCHITECTURE.md`, and relevant tests should change together.

## 14. Implementation Ownership

Shared ownership boundaries:

- `src/link_syntax.rs`: canonical token scanning and target normalization
- `src/extractor.rs`: single-file extraction using the shared parser
- `src/scanner.rs`: persistence of extracted `links` and `embeds`, and derived backlink computation
- `src/renamer.rs`: vault-wide rewrite of link/embed syntax during rename using token spans
- `src/renderer/mod.rs`: special runtime handling for `.base` embeds

Architectural rule:

- do not create a second independent definition of link target normalization outside `src/link_syntax.rs`
- if query/render/name-handling behavior depends on link target semantics, it must stay aligned with this document and the shared parser implementation
