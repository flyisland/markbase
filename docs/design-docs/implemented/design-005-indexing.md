---
id: design-005
title: "Indexing"
status: implemented
module: indexing
---

# Indexing Design

This document defines the active indexing contract for `markbase index` and automatic indexing.

## 1. Purpose

The indexing pipeline turns vault files under `MARKBASE_BASE_DIR` into derived DuckDB records.

Its responsibilities are:

- traverse the vault
- decide which filesystem entries are eligible for indexing
- extract structured fields from Markdown notes
- preserve non-Markdown resources in the index for name/path-based resolution
- keep the index rebuildable from the filesystem

## 2. Traversal Contract

Indexing traverses the vault recursively from the configured base directory.

Current traversal behavior is:

- symbolic links are followed
- only files with an extension are eligible for indexing
- dot-prefixed hidden files and directories are skipped by default
- paths matched by the root `.gitignore` are skipped
- paths matched by the root `.markbaseignore` are skipped
- when `.gitignore` and `.markbaseignore` both match the same path, `.markbaseignore` is applied later and therefore wins

This filtering happens before file content is parsed or compared for incremental updates.

## 3. Indexed File Classes

### 3.1 Markdown notes

Files with extension `.md` are indexed as Markdown notes.

For these files, markbase stores:

- path, folder, name, extension, size, and timestamps
- parsed frontmatter as `note.*` properties
- extracted body and frontmatter links
- extracted body embeds
- extracted tags from body plus frontmatter `tags`

The logical note name is the filename without the `.md` extension.

### 3.2 Non-Markdown resources

Files with any other extension are indexed as non-Markdown resources.

This includes `.base` files and attachments such as images or other files with extensions.

For these files, markbase stores:

- path, folder, name, extension, size, and timestamps
- empty `tags`, `links`, `embeds`, and `backlinks`
- `null` frontmatter properties

The logical name for non-Markdown resources is the full filename including extension.

## 4. Incremental Update Contract

Traversal still visits the filtered filesystem set on each indexing run, but content parsing is skipped for unchanged files.

A file is treated as unchanged when both of these are true:

- filesystem size matches the indexed size
- filesystem modified time is not newer than the indexed modified time

Unchanged files are not reparsed, but files that disappeared from the filtered traversal set are removed from the index.

## 5. Name Uniqueness During Indexing

Indexing enforces global logical-name uniqueness across the filtered traversal set.

- Markdown notes collide on filename without `.md`
- non-Markdown resources collide on full filename including extension
- on collision, markbase keeps the first indexed path, emits a warning, and skips the later path

This preserves basename-oriented Obsidian link semantics across indexed data.
