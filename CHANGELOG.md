# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2026-03-15

### Added

- **Recursive note embed rendering** - `markbase note render` now expands Markdown note embeds like `![[note]]` inline, strips embedded note frontmatter, keeps rendering through nested note and `.base` embeds, and soft-fails with warnings plus HTML comment placeholders for missing notes, read failures, and recursion cycles.
- **Scoped `.base` view selection** - Base embeds can now target a single view with selectors like `![[tasks.base#Open Tasks]]`, and inline `.base` embeds no longer need to occupy an entire line by themselves.
- **Shared link syntax parser** - Link extraction, rename rewrites, render embed scanning, and link-field verification now share a single parser that distinguishes wikilinks from embeds, skips fenced and inline code contexts, and preserves escaped pipe forms.

### Changed

- **Rename rewrite fidelity** - `markbase note rename` now normalizes Markdown-note targets to path-free, extension-free names, preserves anchors, block IDs, aliases, and escaped separators, and avoids rewriting fenced code blocks or inline code spans.
- **Link field verification** - `note verify` now accepts pure wikilink strings with optional paths, `.md` suffixes, headings, and aliases, while still rejecting surrounding prose that is not a single standalone wikilink.
- **Documentation structure** - Architecture, documentation, design, and execution-plan docs were reorganized so the current design contracts and completed plans live under the new docs indexes.

## [0.5.3] - 2026-03-13

### Fixed

- **Fast-fail note command validation** - `note verify`, `note render`, `note resolve`, and `note rename` now reject path-like inputs before lookup or indexing, so invalid names such as `logs/foo/bar.md` report the real input-shape problem instead of a misleading "not found" error.
- **Command-specific suffix rules** - `note new` and `note verify` now require pure note names without file extensions, `note resolve` rejects file-style inputs with extensions, `note render` accepts note names plus `.base` filenames only, and `note rename` continues to allow extension-bearing resource names like `aaa.jpeg` so embed rewrites remain supported.

## [0.5.1] - 2026-03-08

### Changed

- **Verify diagnostics for agents** - `markbase note verify` now includes `file.path` in the verification header so agents can identify the exact file being checked.
- **Global description repair hints** - Global `description` warnings now print a structured `Definition:` line with the required type, non-empty constraint, and the spec-aligned guidance `一句话说明这个 note 是什么`.

## [0.5.0] - 2026-03-08

### Changed

- **JSON-first CLI defaults** - `markbase query`, `markbase template list`, and Base view rendering in `markbase note render` now default to JSON output for agent and script workflows.
- **Output format simplification** - The legacy `list` output mode has been removed in favor of `json` and `table`.
- **Documentation clarity** - Template and description docs now explicitly distinguish `_schema.description`, `_schema.properties.description`, and the instance frontmatter `description` field.

## [0.4.0] - 2026-03-08

### Added

- **`note resolve` command** - Added agent-friendly entity alignment for one or more names with JSON output and match statuses: `exact`, `alias`, `multiple`, and `missing`.
- **Description-aware matches** - Resolve results now include normalized `description` data alongside `name`, `path`, `type`, and `matched_by` to make disambiguation cheaper for agents.
- **Implicit indexing for DB-backed commands** - Commands that rely on the DuckDB index now trigger indexing automatically before execution, reducing stale-query footguns.

### Changed

- **Render output defaults** - `markbase note render` now defaults to table output, and unsupported `json` output is rejected explicitly.
- **Shared output rendering** - YAML list and Markdown table rendering paths now use unified formatting helpers for more consistent output.

### Fixed

- **Verification diagnostics** - `note verify` now reports field-definition context in warnings and detects missing embedded `.base` files.
- **Version output** - Crates.io installs continue to show plain version output without git metadata, while git builds retain SHA and timestamp details.

## [0.3.2] - 2026-03-06

### Fixed

- **Version output** - Fixed version display for crates.io installs. When installed from crates.io (no git repo), shows just the version number without git metadata: `markbase 0.3.2`. When built from git repo, includes SHA and timestamp: `markbase 0.3.2 (abc123 2026-03-06 10:30:00)`.

## [0.3.1] - 2026-03-06

### Fixed

- **Version output** - Fixed build script to handle missing git information gracefully. Version string now shows git SHA and timestamp correctly, with "unknown" fallback and build timestamp when git is unavailable.

## [0.3.0] - 2026-03-06

### Added

#### Note Rendering
- **New `note render` command** — renders a note to stdout with Obsidian Base embed expansion:
  - `link(this)` and `link("name")` translated to wikilink string literals for property matching
  - `file.hasLink(this.file)`, `file.hasTag()` (with nested tag support), `file.inFolder()`,
    date arithmetic (`"30d"`, `"1 year"` formats), `isEmpty()`, `contains()` filters supported
  - `order` field maps to SELECT columns; `sort` field maps to ORDER BY (independent fields)
  - bare column names in `order`/`sort` resolve to note properties, not DB columns
  - list and table output formats; `--dry-run` for SQL inspection
  - warnings (unsupported filters, missing base files) to stderr; exit 0 on warnings only

#### Note Verification
- **New `note verify` command** - Validates notes against MTS template schemas with comprehensive checks:
  - **Template Resolution** - Reads `templates` field from note frontmatter and loads corresponding template files from `templates/` directory
  - **Location Validation** - Verifies note is in correct folder as specified by `_schema.location`
  - **Field Presence Checks** - Ensures all template-defined fields exist in the note
  - **Value Consistency** - Validates non-list field values match template defaults
  - **List Inclusion** - Checks that note arrays contain all required template values
  - **Required Fields** - Validates `_schema.required` fields are present and non-empty
  - **Type Validation** - Supports `text`, `number`, `boolean`, `date`, `datetime`, and `list` types
  - **Enum Constraints** - Validates values against allowed enumerations
  - **Link Validation** - Verifies wiki-link format and optional target note type constraints
  - **Multi-Template Support** - Validates against multiple templates with conflict detection
- **Structured Output** - Reports issues as ERROR (fatal) or WARN (field-level), with summary statistics
- **Exit Codes** - Returns `0` for pass/warnings only, `1` for errors

## [0.2.0] - 2025-03-04

### Added

#### Indexing
- **All Files Support** - Index now includes all files in the vault, not just `.md` files. Non-markdown files are tracked with their full filename including extension.
- **Ignore File Support** - Added support for `.markbaseignore` and `.gitignore` files to exclude specific paths from indexing.
- **Deletion Tracking** - `index` command now shows deleted files count and total notes in output.

#### Query System
- **Namespace Prefixes** - Introduced explicit `file.*` and `note.*` namespace prefixes for field access:
  - `file.name`, `file.path`, `file.size`, `file.mtime`, etc. for file properties
  - `note.*` or bare identifiers for frontmatter properties
- **Field Aliases** - SELECT output now shows field aliases with namespace prefixes for clarity.
- **Tag Array Support** - Frontmatter tags are now merged into the `file.tags` array alongside content tags.

#### Tags
- **Obsidian Tag Format** - Implemented strict Obsidian tag format validation and normalization:
  - Supports alphanumeric characters, underscores, and hyphens
  - Validates tag syntax and provides helpful error messages
  - Normalizes tags for consistent storage and querying

#### Creator & Templates
- **MKS v1.11 Callouts** - Updated template system to support MKS v1.11 callout format (removed directive filtering).

### Changed

#### Query Translation
- **Unified SQL Generation** - Removed `DEFAULT_FIELDS` in favor of explicit namespace-based field translation.
- **Breaking**: Query fields now require explicit namespace prefixes. Bare identifiers are treated as frontmatter properties (shorthand for `note.*`).

### Fixed

- **Query Column Names** - Fixed issue where query results didn't return proper column names.
- **Backlinks Storage** - Backlinks now store note names instead of full file paths for consistency.
- **Template List Query** - Updated template list to use explicit SQL with template-specific fields.

### Documentation

- Added comprehensive `properties_design.md` specification documenting the namespace system.
- Added integration test coverage for all CLI commands.

## [0.1.0] - 2025-01-15

### Added

- Initial release of markbase
- Core indexing functionality for Markdown files
- DuckDB-based storage with incremental updates
- Wiki-links and backlinks tracking
- Basic query system with expression and SQL modes
- Note creation with template support
- Note renaming with link updates
- Multiple output formats (table, json, list)

[0.5.0]: https://github.com/flyisland/markbase/compare/0.4.0...0.5.0
[0.4.0]: https://github.com/flyisland/markbase/compare/0.3.2...0.4.0
[0.3.2]: https://github.com/flyisland/markbase/compare/0.3.1...0.3.2
[0.3.1]: https://github.com/flyisland/markbase/compare/0.3.0...0.3.1
[0.3.0]: https://github.com/flyisland/markbase/compare/0.2.0...0.3.0
[0.2.0]: https://github.com/flyisland/markbase/compare/0.1.0...0.2.0
[0.1.0]: https://github.com/flyisland/markbase/releases/tag/0.1.0
[0.6.0]: https://github.com/flyisland/markbase/compare/0.5.3...0.6.0
[0.5.3]: https://github.com/flyisland/markbase/compare/0.5.1...0.5.3
[0.5.1]: https://github.com/flyisland/markbase/compare/0.5.0...0.5.1
