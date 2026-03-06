pub mod filter;
pub mod output;

use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use gray_matter::engine::YAML;
use gray_matter::Matter;
use regex::Regex;
use serde_json::Value;

use crate::db::Database;
use crate::renderer::filter::{merge_filters, translate_columns, translate_sort, ThisContext};
use crate::renderer::output::{render_list, render_table, Row};

static BASE_EMBED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^!\[\[([^\]]+\.base)\]\]\s*$").unwrap());

#[derive(Debug, Clone, PartialEq)]
pub enum RenderFormat {
    List,
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
        return Err(format!(
            "ERROR: note '{}' not found in index. Run `markbase index` first.",
            name
        )
        .into());
    }

    if rows.len() > 1 {
        return Err(format!("ERROR: multiple notes found with name '{}'", name).into());
    }

    let row = &rows[0];
    let this = ThisContext {
        path: row[0].clone(),
        folder: row[1].clone(),
        name: row[2].clone(),
        ext: row[3].clone(),
        size: row[4].parse().unwrap_or(0),
        ctime: row[5].clone(),
        mtime: row[6].clone(),
        tags: serde_json::from_str(&row[7]).unwrap_or_default(),
        links: serde_json::from_str(&row[8]).unwrap_or_default(),
    };

    let note_path = base_dir.join(&this.path);
    let content = fs::read_to_string(&note_path)?;
    let matter = Matter::<YAML>::new();
    let parsed = matter
        .parse::<Value>(&content)
        .map_err(|e| format!("failed to parse frontmatter: {}", e))?;
    let body = parsed.content;

    for line in body.lines() {
        if let Some(caps) = BASE_EMBED_RE.captures(line) {
            let embed_name = caps.get(1).unwrap().as_str();
            render_base_embed(embed_name, base_dir, db, &this, opts);
        } else {
            println!("{}", line);
        }
    }

    Ok(())
}

fn render_base_embed(
    embed_name: &str,
    base_dir: &Path,
    db: &Database,
    this: &ThisContext,
    opts: &RenderOptions,
) {
    let embed_escaped = embed_name.replace('\'', "''");
    let sql = format!("SELECT path FROM notes WHERE name = '{}'", embed_escaped);

    let result = db.query(&sql, "", usize::MAX);

    let (_, rows) = match result {
        Ok(r) => r,
        Err(_e) => {
            eprintln!(
                "WARN: base file '{}' not found in index, skipping.",
                embed_name
            );
            println!("<!-- [markbase] base '{}' not found -->", embed_name);
            return;
        }
    };

    if rows.is_empty() {
        eprintln!(
            "WARN: base file '{}' not found in index, skipping.",
            embed_name
        );
        println!("<!-- [markbase] base '{}' not found -->", embed_name);
        return;
    }

    let base_path = base_dir.join(&rows[0][0]);
    let base_content = match fs::read_to_string(&base_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("WARN: failed to read '{}': {}", embed_name, e);
            println!("<!-- [markbase] failed to read '{}' -->", embed_name);
            return;
        }
    };

    let base_yaml: Value = match serde_yaml::from_str(&base_content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("WARN: failed to parse '{}': {}", embed_name, e);
            println!(
                "<!-- [markbase] failed to parse '{}': {} -->",
                embed_name, e
            );
            return;
        }
    };

    let global_filter = base_yaml.get("filters");
    let base_properties = base_yaml.get("properties");
    let views = match base_yaml.get("views").and_then(|v| v.as_array()) {
        Some(v) if !v.is_empty() => v,
        _ => return,
    };

    for view in views {
        let view_name = view
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(embed_name);

        let order_vals: &[Value] = view
            .get("order")
            .and_then(|v| v.as_array())
            .map(|a| a.as_slice())
            .unwrap_or(&[]);

        let mut warnings: Vec<String> = Vec::new();

        let where_clause = merge_filters(
            global_filter,
            view.get("filters"),
            this,
            embed_name,
            &mut warnings,
        );
        let columns = translate_columns(order_vals, base_properties, embed_name, &mut warnings);
        let order_by = translate_sort(view.get("sort"), embed_name, &mut warnings);

        for w in &warnings {
            eprintln!("{}", w);
        }

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

        if opts.dry_run {
            println!("<!-- start: [markbase] dry-run from {} -->\n", embed_name);
            println!("> **{}**\n", view_name);
            println!("```sql\n{}\n```", sql);
            println!("<!-- end: [markbase] dry-run from {} -->\n", embed_name);
        } else {
            println!("<!-- start: [markbase] rendered from {} -->\n", embed_name);
            println!("> **{}**\n", view_name);

            match db.query(&sql, "", usize::MAX) {
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

                    let output = match opts.format {
                        RenderFormat::Table => render_table(&rows, &columns),
                        RenderFormat::List => {
                            let list_output = render_list(&rows, &columns);
                            format!("```yaml\n{}```", list_output)
                        }
                    };
                    println!("{}", output);
                }
                Err(e) => {
                    eprintln!(
                        "WARN: query failed for view '{}' in '{}': {}",
                        view_name, embed_name, e
                    );
                    println!(
                        "<!-- end: [markbase] query failed for view '{}' -->",
                        view_name
                    );
                    return;
                }
            }
            println!("<!-- end: [markbase] rendered from {} -->\n", embed_name);
        }
    }
}
