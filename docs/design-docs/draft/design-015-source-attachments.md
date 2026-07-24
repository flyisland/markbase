---
id: design-015
title: "Source Attachments"
status: draft
module: source-attachments
---

# Source Attachments

`markbase source` provides an explicit write path for evidence files belonging
to a Markdown source document. Files and their records remain in the vault;
DuckDB is not an attachment store.

## Source eligibility and layout

The target is a path-free Markdown note whose frontmatter contains
`type: source`. An input must be a readable, non-symlink regular file. It is
copied, never moved, to:

```text
<source-parent>/attachments/<source-file-stem>/<filename>
```

The command streams both copy and SHA-256 computation. An identical existing
record returns `existing`; a distinct file with the same name receives `_02`,
`_03`, and so on before its extension. No target is overwritten.

## Managed Markdown region

The `source_input` template must place these exact comments within its
`## 证据附件` section, after any user-facing guidance:

```markdown
<!-- markbase:source-attachments:start -->
<!-- markbase:source-attachments:end -->
```

Existing source notes are migrated deliberately by adding those two comments
around an initially empty managed region. Existing hand-authored attachment
rows remain outside the region and are not parsed, changed, or assumed to have
complete metadata. A missing, duplicated, or malformed boundary is an error;
the command never guesses based on a natural-language heading or callout.

Each generated list item is human-readable and has an adjacent HTML comment
containing JSON metadata: vault-relative archive path, source input path,
description, SHA-256, byte count, and MIME type. The JSON comment is the
machine contract. `source attachments` reads it in document order, and
`source verify-attachments` rehashes files and reports structured issues
without repairing anything.

## Failure boundary

The source note and input are completely validated before a destination
directory is created. Files are copied through a temporary sibling and renamed
atomically. Source Markdown is likewise replaced atomically; if that final
write fails, a newly copied file is removed. The command never changes
`## 原始输入`.
