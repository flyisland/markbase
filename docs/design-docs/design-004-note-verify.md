# `note verify` Design

**Status:** Active  
**Target:** markbase CLI  
**Related docs:** `docs/design-docs/legacy/template_schema.md`, `ARCHITECTURE.md`, `README.md`

## Scope

`markbase note verify <name>` validates one note against:

- note-facing input-shape rules
- a global `description` contract
- the note's referenced template files under `templates/`
- template frontmatter inheritance rules
- `_schema.required` and `_schema.properties`
- link-field target checks
- embedded `.base` existence checks in the Markdown body

The command is read-only. It does not mutate the vault or the index.

## Command Contract

```bash
markbase note verify <name>
```

`<name>` must be a path-free note name without file extension.

Execution is split across:

- `src/main.rs`: CLI validation, index orchestration, stderr formatting, exit code
- `src/verifier.rs`: verification logic and issue collection
- `src/name_validator.rs`: shared note-name validation
- `src/extractor.rs`: Markdown-body embed extraction
- `src/link_syntax.rs`: shared wikilink parsing and normalization
- `src/db.rs`: note lookup by normalized name

## Output And Exit Codes

If there are no issues, stdout prints:

```text
✓ note '<name>' passed all checks against: <template-list>.
```

If there are issues, stderr prints:

1. a header containing the note name, `file.path`, and the templates that contributed schema
2. one line per issue with `[ERROR]` or `[INFO]`
3. an optional `→ Definition:` line for issues that carry schema/global-field context
4. a final summary line

Exit behavior:

- exit code `0`: no issues, or only `INFO`
- exit code `1`: at least one verification `ERROR`
- invalid CLI input such as path-like names or names with extensions fails before verification runs

## Current Verification Flow

### 1. CLI input validation

`src/main.rs` validates the argument with `validate_note_name()`.

- names containing directories fail before lookup
- names with any file extension fail before lookup

These are command errors, not `VerifyIssue`s.

### 2. Index refresh and note lookup

`main.rs` refreshes the derived index, then `verifier::verify_note()` looks up the note by name.

- zero matches: `ERROR`, return immediately
- multiple matches: `ERROR`, return immediately

### 3. Read note content and run global checks

After a single note is found, verifier:

- reads the note file from disk
- extracts embeds from Markdown body content
- checks the global `description` field before any template validation

Global `description` currently reports `ERROR` for:

- missing field
- blank string after `trim()`
- non-string value

### 4. Resolve `templates`

Verifier reads `properties.templates`.

- if it is missing, not an array, or an empty array: `ERROR`, return immediately
- if an element is not a string: `ERROR`, return immediately
- if the first string element is not a pure standalone wikilink: `ERROR`, return immediately
- if later string elements are invalid wikilinks, current implementation silently ignores them instead of reporting an issue

Pure wikilink parsing is shared with the link parser. Acceptable forms include path, `.md`, heading, and alias suffixes as long as the full string is exactly one wikilink token after trimming.

### 5. Load template files

For each resolved template name, verifier reads `templates/<template>.md`.

- missing template file: `ERROR`, return immediately
- unreadable template file: `ERROR`, return immediately
- template frontmatter parse failure: `ERROR`, return immediately
- template without `_schema`: treated as a valid template with no schema constraints

When `_schema.properties` is present, verifier builds a merged field-definition map for later `Definition:` output.

If two templates define the same field with different `type` values, verifier emits an `ERROR` and keeps the first definition.

### 6. Apply template checks

For each loaded template frontmatter object:

- `_schema.location` mismatch: `ERROR`
- missing non-`_schema` template field in note frontmatter: `ERROR`
- list-valued template field not fully contained by note list: `ERROR`
- scalar template field value mismatch: `ERROR`

For `_schema.required` and `_schema.properties`:

- required field missing or empty: `ERROR`
- field type mismatch: `ERROR`
- enum violation: `ERROR`
- `format: link` validation:
  - dangling reference form `"[?[[...]]]"`: `INFO`
  - invalid link syntax: `ERROR`
  - target note missing: `ERROR`
  - target note `type` mismatch against `target`: `ERROR`

### 7. Check embedded `.base` targets

Verifier inspects Markdown-body embeds extracted by `Extractor::extract()`.

- only embed targets ending with `.base` are checked
- non-`.base` embeds are ignored by this phase
- missing embedded `.base` target in the index: `ERROR`
- database lookup failure while checking embedded `.base`: `ERROR`

These `.base` errors are collected and verification continues across the remaining embeds before summary/exit.

## Decision Table

This table reflects the current implementation, not the historical draft behavior.

### Immediate Return = Yes

| Trigger condition | Level | Affects process exit code |
| --- | --- | --- |
| CLI name contains directories | command error | yes |
| CLI name contains file extension | command error | yes |
| Note lookup returns zero rows | `ERROR` | yes |
| Note lookup returns multiple rows | `ERROR` | yes |
| `templates` missing, not array, or empty | `ERROR` | yes |
| `templates` element is non-string | `ERROR` | yes |
| First `templates` string is not a pure wikilink | `ERROR` | yes |
| All `templates` elements are discarded so no template names remain | `ERROR` | yes |
| Template file does not exist | `ERROR` | yes |
| Template file exists but cannot be read | `ERROR` | yes |
| Template frontmatter parse fails | `ERROR` | yes |

### Immediate Return = No

| Trigger condition | Level | Affects process exit code |
| --- | --- | --- |
| Global `description` missing | `ERROR` | yes |
| Global `description` blank after trim | `ERROR` | yes |
| Global `description` is non-string | `ERROR` | yes |
| Later `templates` string is not a pure wikilink | no issue is emitted; element is ignored | no |
| Template lacks `_schema` | no issue is emitted; schema checks are skipped for that template | no |
| Conflicting field `type` definitions across templates | `ERROR` | yes |
| `_schema.location` mismatch | `ERROR` | yes |
| Missing non-`_schema` template field | `ERROR` | yes |
| List-valued template field not fully contained by note list | `ERROR` | yes |
| Scalar template field value mismatch | `ERROR` | yes |
| `_schema.required` field missing or empty | `ERROR` | yes |
| `_schema.properties` type mismatch | `ERROR` | yes |
| `_schema.properties.enum` violation | `ERROR` | yes |
| `format: link` value is dangling reference form | `INFO` | no |
| `format: link` value is not a pure wikilink | `ERROR` | yes |
| `format: link` target note missing | `ERROR` | yes |
| `format: link` target note type mismatch | `ERROR` | yes |
| Embedded `.base` target missing from vault/index | `ERROR` | yes |
| Embedded `.base` check hits DB lookup failure | `ERROR` | yes |

## Definition Line Contract

Some issues emit a compact `Definition:` line after the message.

Current sources:

- global `description` issues use a fixed global definition string
- `_schema.required`, type mismatch, enum violation, and some link-target issues use the merged `SchemaFieldInfo`

Not every issue includes a definition. For example:

- `_schema.location` mismatch has no definition line
- missing non-`_schema` template field has no definition line
- conflicting multi-template field types have no definition line
- `.base` embed errors have no definition line

## Known Current Gaps

These are intentional documentation of current behavior, not proposals:

- verification still uses early returns for note lookup, `templates`, and template file load failures; later issue categories continue collecting once verification passes those gates
- only the first invalid string element in `templates` is rejected; later invalid string elements are silently dropped

## References

- `src/main.rs`
- `src/verifier.rs`
- `src/name_validator.rs`
- `src/extractor.rs`
- `tests/cli_note.rs`
