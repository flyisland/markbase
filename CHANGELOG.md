# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.2.0]: https://github.com/flyisland/markbase/compare/0.1.0...0.2.0
[0.1.0]: https://github.com/flyisland/markbase/releases/tag/0.1.0