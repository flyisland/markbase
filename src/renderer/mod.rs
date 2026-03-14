pub mod filter;
pub mod output;

use std::fs;
use std::path::Path;

use gray_matter::Matter;
use gray_matter::engine::YAML;
use serde_json::Value;

use crate::db::Database;
use crate::link_syntax::{LinkKind, LinkToken, ScanContext, scan_link_tokens};
use crate::name_validator::validate_render_target_name;
use crate::renderer::filter::{ThisContext, merge_filters, translate_columns, translate_sort};
use crate::renderer::output::{Row, render_json, render_table};

#[derive(Debug, Clone, PartialEq)]
pub enum RenderFormat {
    Json,
    Table,
}

pub struct RenderOptions {
    pub format: RenderFormat,
    pub dry_run: bool,
}

pub fn render_note(
    base_dir: &Path,
    db: &Database,
    name: &str,
    opts: &RenderOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_render_target_name(name)?;

    let name_escaped = name.replace('\'', "''");
    let sql = format!(
        "SELECT path, folder, name, ext, size, \
         CAST(ctime AS TEXT), CAST(mtime AS TEXT), \
         to_json(tags), to_json(links), properties \
         FROM notes WHERE name = '{}'",
        name_escaped
    );

    let (_, rows) = db
        .query(&sql, "", usize::MAX)
        .map_err(|e| format!("database query failed: {}", e))?;

    if rows.is_empty() {
        return Err(format!("ERROR: note '{}' not found in the vault.", name).into());
    }

    if rows.len() > 1 {
        return Err(format!("ERROR: multiple notes found with name '{}'", name).into());
    }

    let row = &rows[0];
    let ext = &row[3];

    // Check if the file type is supported
    if ext != "md" && ext != "base" {
        return Err(format!(
            "ERROR: note '{}' has unsupported extension '{}'. Only .md and .base files are supported.",
            name, ext
        )
        .into());
    }

    let is_base_file = ext == "base";
    let this = ThisContext {
        path: row[0].clone(),
        folder: row[1].clone(),
        name: row[2].clone(),
        ext: ext.clone(),
        size: row[4].parse().unwrap_or(0),
        ctime: row[5].clone(),
        mtime: row[6].clone(),
        // For .base files, only use basic file properties (no tags, links, frontmatter)
        tags: if is_base_file {
            vec![]
        } else {
            serde_json::from_str(&row[7]).unwrap_or_default()
        },
        links: if is_base_file {
            vec![]
        } else {
            serde_json::from_str(&row[8]).unwrap_or_default()
        },
    };

    let note_path = base_dir.join(&this.path);
    let content = fs::read_to_string(&note_path)?;

    if is_base_file {
        // For .base files, execute its views directly (as if it's embedded)
        // The base name includes the extension
        let base_name = &this.name;
        render_base_embed(base_name, None, base_dir, db, &this, opts);
    } else {
        // For .md files, parse frontmatter and render body with base embed expansion
        let matter = Matter::<YAML>::new();
        let parsed = matter
            .parse::<Value>(&content)
            .map_err(|e| format!("failed to parse frontmatter: {}", e))?;
        let body = parsed.content;
        let rendered_body = render_body_with_base_embeds(&body, base_dir, db, &this, opts);
        print!("{}", rendered_body);
    }

    Ok(())
}

/// Result of parsing a .base file
struct BaseFileData {
    global_filter: Option<Value>,
    properties: Option<Value>,
    views: Vec<Value>,
}

/// Parse .base file content and extract components
fn parse_base_file(content: &str, embed_name: &str) -> Result<Option<BaseFileData>, String> {
    let base_yaml: Value = match serde_yaml::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("WARN: failed to parse '{}': {}", embed_name, e);
            return Err(format!(
                "<!-- [markbase] failed to parse '{}': {} -->",
                embed_name, e
            ));
        }
    };

    let views = match base_yaml.get("views").and_then(|v| v.as_array()) {
        Some(v) if !v.is_empty() => v.clone(),
        _ => return Ok(None),
    };

    Ok(Some(BaseFileData {
        global_filter: base_yaml.get("filters").cloned(),
        properties: base_yaml.get("properties").cloned(),
        views,
    }))
}

/// Build SQL query for a single view
fn build_view_sql(
    view: &Value,
    global_filter: Option<&Value>,
    base_properties: Option<&Value>,
    this: &ThisContext,
    embed_name: &str,
    warnings: &mut Vec<String>,
) -> (String, Vec<crate::renderer::output::ColumnMeta>) {
    let order_vals: &[Value] = view
        .get("order")
        .and_then(|v| v.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[]);

    let where_clause = merge_filters(
        global_filter,
        view.get("filters"),
        this,
        embed_name,
        warnings,
    );
    let columns = translate_columns(order_vals, base_properties, embed_name, warnings);
    let order_by = translate_sort(view.get("sort"), embed_name, warnings);

    let select_exprs: Vec<&str> = columns.iter().map(|c| c.sql_expr.as_str()).collect();
    let mut sql = format!("SELECT {} FROM notes", select_exprs.join(", "));
    if let Some(w) = &where_clause {
        sql.push_str(&format!(" WHERE {}", w));
    }
    if !order_by.is_empty() {
        sql.push_str(&format!(" ORDER BY {}", order_by));
    }
    if let Some(l) = view.get("limit").and_then(|v| v.as_u64()) {
        sql.push_str(&format!(" LIMIT {}", l));
    }

    (sql, columns)
}

/// Execute query and render output for a single view
fn execute_and_render(
    db: &Database,
    sql: &str,
    columns: &[crate::renderer::output::ColumnMeta],
    view_name: &str,
    embed_name: &str,
    opts: &RenderOptions,
) -> String {
    if opts.dry_run {
        return format!("<!-- start: [markbase] dry-run from {} -->\n\n", embed_name)
            + &format!(
                "> **{}**\n\n```sql\n{}\n```\n<!-- end: [markbase] dry-run from {} -->\n\n",
                view_name, sql, embed_name
            );
    }

    match db.query(sql, "", usize::MAX) {
        Ok((_, raw_rows)) => {
            let rows: Vec<Row> = raw_rows
                .iter()
                .map(|raw| {
                    columns
                        .iter()
                        .enumerate()
                        .map(|(i, col)| {
                            let val = raw.get(i).cloned().filter(|s| !s.is_empty());
                            (col.display_name.clone(), val)
                        })
                        .collect()
                })
                .collect();

            let rendered = match opts.format {
                RenderFormat::Table => render_table(&rows, columns),
                RenderFormat::Json => {
                    let json_output = render_json(&rows, columns);
                    format!("```json\n{}\n```", json_output)
                }
            };

            format!(
                "<!-- start: [markbase] rendered from {} -->\n\n> **{}**\n\n{}\n<!-- end: [markbase] rendered from {} -->\n\n",
                embed_name, view_name, rendered, embed_name
            )
        }
        Err(e) => {
            eprintln!(
                "WARN: query failed for view '{}' in '{}': {}",
                view_name, embed_name, e
            );
            format!(
                "<!-- start: [markbase] rendered from {} -->\n\n> **{}**\n\n<!-- end: [markbase] rendered from {} -->\n\n",
                embed_name, view_name, embed_name
            )
        }
    }
}

/// Main driver for rendering base embeds
fn render_base_embed(
    embed_name: &str,
    view_selector: Option<&str>,
    base_dir: &Path,
    db: &Database,
    this: &ThisContext,
    opts: &RenderOptions,
) {
    print!(
        "{}",
        render_base_embed_to_string(embed_name, view_selector, base_dir, db, this, opts)
    );
}

fn render_base_embed_to_string(
    embed_name: &str,
    view_selector: Option<&str>,
    base_dir: &Path,
    db: &Database,
    this: &ThisContext,
    opts: &RenderOptions,
) -> String {
    // Look up base file in database
    let embed_escaped = embed_name.replace('\'', "''");
    let sql = format!("SELECT path FROM notes WHERE name = '{}'", embed_escaped);
    let result = db.query(&sql, "", usize::MAX);

    let rows = match result {
        Ok((_, r)) if !r.is_empty() => r,
        _ => {
            eprintln!(
                "WARN: base file '{}' not found in index, skipping.",
                embed_name
            );
            return format!("<!-- [markbase] base '{}' not found -->", embed_name);
        }
    };

    // Read and parse base file
    let base_path = base_dir.join(&rows[0][0]);
    let base_content = match fs::read_to_string(&base_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("WARN: failed to read '{}': {}", embed_name, e);
            return format!("<!-- [markbase] failed to read '{}' -->", embed_name);
        }
    };

    let base_data = match parse_base_file(&base_content, embed_name) {
        Ok(Some(d)) => d,
        Ok(None) => return String::new(),
        Err(comment) => return comment,
    };

    // Process each view
    let selected_views: Vec<&Value> = if let Some(selector) = view_selector {
        base_data
            .views
            .iter()
            .filter(|view| view.get("name").and_then(|v| v.as_str()) == Some(selector))
            .collect()
    } else {
        base_data.views.iter().collect()
    };

    if let Some(selector) = view_selector
        && selected_views.is_empty()
    {
        eprintln!(
            "WARN: view '{}' not found in '{}', skipping.",
            selector, embed_name
        );
        return format!(
            "<!-- [markbase] view '{}' not found in '{}' -->",
            selector, embed_name
        );
    }

    let mut output = String::new();

    for view in selected_views {
        let view_name = view
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(embed_name);

        let mut warnings = Vec::new();
        let (sql, columns) = build_view_sql(
            view,
            base_data.global_filter.as_ref(),
            base_data.properties.as_ref(),
            this,
            embed_name,
            &mut warnings,
        );

        for w in &warnings {
            eprintln!("{}", w);
        }

        output.push_str(&execute_and_render(
            db, &sql, &columns, view_name, embed_name, opts,
        ));
    }

    output
}

fn render_body_with_base_embeds(
    body: &str,
    base_dir: &Path,
    db: &Database,
    this: &ThisContext,
    opts: &RenderOptions,
) -> String {
    let mut output = String::new();
    let mut cursor = 0;

    for token in scan_link_tokens(body, ScanContext::MarkdownBody) {
        if token.kind != LinkKind::Embed || !token.parsed.normalized_target.ends_with(".base") {
            continue;
        }

        let (replace_start, replace_end) = replacement_span(body, &token);
        output.push_str(&body[cursor..replace_start]);

        let replacement = render_base_embed_to_string(
            &token.parsed.normalized_target,
            token.parsed.anchor.as_deref(),
            base_dir,
            db,
            this,
            opts,
        );

        if replace_start == token.full_span.start
            && replace_start > 0
            && !body[..replace_start].ends_with('\n')
        {
            output.push('\n');
        }

        output.push_str(&replacement);

        if replace_end == token.full_span.end
            && replace_end < body.len()
            && !body[replace_end..].starts_with('\n')
            && !output.ends_with('\n')
        {
            output.push('\n');
        }

        cursor = replace_end;
    }

    output.push_str(&body[cursor..]);
    output
}

fn replacement_span(body: &str, token: &LinkToken) -> (usize, usize) {
    let line_start = body[..token.full_span.start]
        .rfind('\n')
        .map_or(0, |idx| idx + 1);
    let line_end = body[token.full_span.end..]
        .find('\n')
        .map_or(body.len(), |idx| token.full_span.end + idx);
    let line = &body[line_start..line_end];
    let trimmed = line.trim();

    if trimmed != &body[token.full_span.start..token.full_span.end] {
        return (token.full_span.start, token.full_span.end);
    }

    let line_end = body[line_end..]
        .strip_prefix('\n')
        .map_or(line_end, |_| line_end + 1);

    (line_start, line_end)
}
