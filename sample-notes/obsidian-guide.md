---
title: Obsidian Integration Guide
tags: [obsidian, guide, integration]
compatibility: full
---

# Obsidian Integration Guide

## Compatibility

The Markdown Base CLI is fully compatible with [Obsidian](https://obsidian.md/) markdown files.

### Supported Features

✅ **YAML Frontmatter**
```yaml
---
title: My Note
tags: [tag1, tag2]
author: John Doe
---
```

✅ **WikiLinks**
```markdown
See [[related-note]] for more info.
See [[note|custom display text]].
```

✅ **Embeds**
```markdown
![[embedded-image.png]]
![[another-note]]
```

✅ **Tags**
```markdown
#single-tag
#nested/tag
Inline text with #tag
```

✅ **Markdown Formatting**
- Headers (# ## ###)
- Bold, italic, strikethrough
- Lists (ordered/unordered)
- Code blocks
- Tables
- Links

## Workflow Integration

### 1. Index Your Vault

```bash
# Index your entire Obsidian vault
markbase index -d ~/Documents/Obsidian-Vault

# Or index a specific folder
markbase index -d ~/Documents/Obsidian-Vault/Projects
```

### 2. Search Your Notes

```bash
# Find all notes with a specific tag
markbase search -t "project"

# Search for a concept
markbase search -q "architecture"

# Find notes in a specific folder
markbase search -f "Projects" --limit 10

# Export results as JSON
markbase search -t "documentation" --format json > docs.json
```

### 3. Daily Workflow

**Morning Review:**
```bash
# Find today's daily note
markbase search -f "daily" --limit 5
```

**Project Search:**
```bash
# Find all project-related notes
markbase search -t "project" -o list
```

## Best Practices

1. **Use Consistent Tagging**
   - Prefer nested tags: `#project/website` vs `#website-project`
   - Use lowercase for consistency

2. **Link Liberally**
   - The CLI tracks backlinks automatically
   - More links = better discovery

3. **Keep Frontmatter Clean**
   - Use standard YAML
   - Avoid special characters in values

4. **Regular Indexing**
   - Run `markbase index` after major changes
   - Use incremental updates for daily changes

## Limitations

The following Obsidian features are not indexed:

- ❌ Canvas files (.canvas)
- ❌ Excalidraw drawings
- ❌ Audio/video files
- ❌ PDF annotations
- ❌ Plugin-specific metadata

## Migration from Other Tools

### From Notion

1. Export as Markdown
2. Import to Obsidian
3. Run `markbase index` on the vault

### From Roam Research

1. Use Roam export tools
2. Convert block references to links
3. Index with the CLI

## Example Queries

```bash
# Find notes created this week
markbase search -q "" -f "daily" | head -7

# Find high-priority projects
markbase search -t "high-priority" -f "Projects"

# Export all tags
markbase search -q "" --format json | jq -r '.[].tags[]' | sort -u
```

Related: [[readme]], [[troubleshooting]]

#obsidian #guide #integration
