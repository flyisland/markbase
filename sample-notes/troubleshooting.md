---
title: Troubleshooting Guide
tags: [help, troubleshooting, support]
category: documentation
---

# Troubleshooting Guide

## Common Issues

### Issue: Database Locked

**Error:**
```
Error: Database is locked
```

**Solution:**
1. Check if another process is using the database
2. Kill any hanging processes
3. Restart the CLI tool

```bash
# Find processes using docs.duckdb
lsof docs.duckdb

# Kill if necessary
kill -9 <PID>
```

### Issue: No Results Found

**Symptoms:** Search returns no results even when files exist.

**Solutions:**
1. Check if database exists:
   ```bash
   ls -lh docs.duckdb
   ```

2. Re-index your notes:
   ```bash
   rm docs.duckdb
   markbase index -d ./notes --force
   ```

3. Verify search query:
   ```bash
   markbase search -q "test" --verbose
   ```

### Issue: Slow Indexing

**Possible Causes:**
- Too many files (>10,000)
- Very large individual files (>1MB)
- Slow disk I/O

**Solutions:**
1. Use incremental indexing (default behavior)
2. Exclude large directories:
   ```bash
   # Move large files to separate folder
   mv ./notes/archives ./archives
   ```

3. Check disk space:
   ```bash
   df -h
   ```

### Issue: Bun Crash

**Note:** If you see a Bun crash report, it's likely due to the DuckDB native module. With the new `@duckdb/node-api` package, this should not occur.

**If it does:**
1. Update Bun to latest version
2. Reinstall dependencies:
   ```bash
   rm -rf node_modules bun.lock
   bun install
   ```

## Getting Help

### Debug Mode

Run commands with `--verbose` flag:
```bash
markbase index -d ./notes --verbose
markbase search -q "test" --verbose
```

### Check Database Contents

```bash
# List all indexed files
markbase search -q "" --format list

# Get file count
markbase search -q "" --format json | jq '. | length'
```

### Report Issues

When reporting issues, include:
1. Bun version: `bun --version`
2. Error message (full stack trace)
3. Command that caused the issue
4. Sample of your markdown files (anonymized)

## Related

- [[readme]] - Getting started guide
- [[performance-tips]] - Optimization guide
- [[architecture]] - System design

#help #troubleshooting #support
