#![allow(clippy::collapsible_if, clippy::manual_strip, dead_code)]

use crate::link_syntax::parse_link_target;
use crate::renderer::output::ColumnMeta;
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ThisContext {
    pub path: String,
    pub folder: String,
    pub name: String,
    pub ext: String,
    pub size: i64,
    pub ctime: String,
    pub mtime: String,
    pub tags: Vec<String>,
    pub links: Vec<String>,
}

static LINK_THIS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"link\(this\)").unwrap());
static LINK_QUOTED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"link\("([^"]+)"\)"#).unwrap());
static HAS_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^file\.hasLink\(this\.file\)$").unwrap());
static HAS_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^file\.hasTag\((.+)\)$").unwrap());
static IN_FOLDER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^file\.inFolder\("([^"]+)"\)$"#).unwrap());
static FILE_PROP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^file\.(\w+)\s*([><=!]+)\s*(.+)$").unwrap());
static THIS_FILE_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"this\.file\.name").unwrap());
static IS_EMPTY_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(.+)\.isEmpty\(\)$").unwrap());
static CONTAINS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.+)\.contains\((.+)\)$").unwrap());
static COMPARE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.+?)\s*([><=!]+)\s*(.+)$").unwrap());
static DATE_INTERVAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d+)\s*([a-zA-Z]+)$").unwrap());

pub fn translate_filter(
    filter: &Value,
    this: &ThisContext,
    base_name: &str,
    warnings: &mut Vec<String>,
) -> Option<String> {
    if let Value::Object(obj) = filter {
        if let Some(and_arr) = obj.get("and").and_then(|v| v.as_array()) {
            let mut parts = Vec::new();
            for item in and_arr {
                if let Some(s) = translate_filter_item(item, this, base_name, warnings) {
                    parts.push(s);
                }
            }
            if parts.is_empty() {
                return None;
            }
            return Some(format!("({})", parts.join(" AND ")));
        }
        if let Some(or_arr) = obj.get("or").and_then(|v| v.as_array()) {
            let mut parts = Vec::new();
            for item in or_arr {
                if let Some(s) = translate_filter_item(item, this, base_name, warnings) {
                    parts.push(s);
                }
            }
            if parts.is_empty() {
                return None;
            }
            return Some(format!("({})", parts.join(" OR ")));
        }
        if let Some(not_arr) = obj.get("not").and_then(|v| v.as_array()) {
            if let Some(first) = not_arr.first() {
                if let Some(s) = translate_filter_item(first, this, base_name, warnings) {
                    return Some(format!("NOT ({})", s));
                }
            }
            return None;
        }
    }
    if let Value::String(s) = filter {
        return translate_string_filter(s, this, base_name, warnings);
    }
    warnings.push(format!(
        "WARN: unsupported filter type in '{}', condition ignored.",
        base_name
    ));
    None
}

fn translate_filter_item(
    item: &Value,
    this: &ThisContext,
    base_name: &str,
    warnings: &mut Vec<String>,
) -> Option<String> {
    if let Value::String(s) = item {
        translate_string_filter(s, this, base_name, warnings)
    } else {
        translate_filter(item, this, base_name, warnings)
    }
}

fn translate_string_filter(
    s: &str,
    this: &ThisContext,
    base_name: &str,
    warnings: &mut Vec<String>,
) -> Option<String> {
    let mut s = s.to_string();

    s = LINK_THIS_RE
        .replace_all(&s, &format!("\"[[{}]]\"", this.name))
        .to_string();
    s = LINK_QUOTED_RE
        .replace_all(&s, |caps: &regex::Captures| format!("\"[[{}]]\"", &caps[1]))
        .to_string();
    s = THIS_FILE_NAME_RE
        .replace_all(&s, &format!("\"{}\"", this.name))
        .to_string();

    if HAS_LINK_RE.is_match(&s) {
        return Some(format!("list_contains(links, '{}')", this.name));
    }

    if let Some(caps) = HAS_TAG_RE.captures(&s) {
        let args = caps[1].split(',').map(|a| a.trim()).collect::<Vec<_>>();
        let parts: Vec<String> = args
            .iter()
            .map(|arg| {
                let tag = arg.trim_matches('"').trim_matches('\'');
                format!(
                    "(list_contains(tags, '{}') OR array_any(tags, x -> x LIKE '{}%'))",
                    tag, tag
                )
            })
            .collect();
        if parts.len() == 1 {
            return Some(parts[0].clone());
        }
        return Some(format!("({})", parts.join(" OR ")));
    }

    if let Some(caps) = IN_FOLDER_RE.captures(&s) {
        let folder = caps[1].trim_matches('/');
        let folder = format!("{}/", folder);
        return Some(format!(
            "(folder = '{}' OR folder LIKE '{}%')",
            folder, folder
        ));
    }

    if let Some(caps) = FILE_PROP_RE.captures(&s) {
        let prop = &caps[1];
        let op = &caps[2];
        let expr = &caps[3];
        let col = file_prop_to_col(prop)?;
        let translated_expr = translate_date_expr(expr).unwrap_or_else(|| {
            if (expr.starts_with('"') && expr.ends_with('"'))
                || (expr.starts_with('\'') && expr.ends_with('\''))
            {
                let inner = &expr[1..expr.len() - 1];
                format!("'{}'", unescape_string_literal(inner))
            } else if expr.parse::<f64>().is_ok() {
                format!("{}::DOUBLE", expr)
            } else {
                format!("'{}'", expr.replace('\'', "''"))
            }
        });

        let col_with_cast = if matches!(prop, "ctime" | "mtime") {
            format!("CAST({} AS TIMESTAMP)", col)
        } else {
            col.to_string()
        };

        return Some(format!("{} {} {}", col_with_cast, op, translated_expr));
    }

    if let Some(caps) = COMPARE_RE.captures(&s) {
        let attr = &caps[1];
        let op = &caps[2];
        let raw_value = caps[3].trim();
        let value = raw_value.trim_matches('"').trim_matches('\'').to_string();

        let (sql_expr, is_frontmatter) = parse_attribute(attr)?;
        let sql_expr = if is_frontmatter {
            normalize_pure_wikilink_sql(&sql_expr)
        } else {
            sql_expr
        };

        let translated_value = if is_quoted_string(raw_value) {
            let inner = &raw_value[1..raw_value.len() - 1];
            let normalized = normalize_pure_wikilink_literal(inner);
            format!("'{}'", unescape_string_literal(&normalized))
        } else if value.parse::<f64>().is_ok() {
            format!("{}::DOUBLE", value)
        } else if translate_date_expr(&value).is_some() {
            translate_date_expr(&value).unwrap()
        } else {
            format!("'{}'", value.replace('\'', "''"))
        };

        return Some(format!("{} {} {}", sql_expr, op, translated_value));
    }

    if let Some(caps) = IS_EMPTY_RE.captures(&s) {
        let attr = &caps[1];
        if attr == "file.tags" {
            return Some("(tags IS NULL OR len(tags) = 0)".to_string());
        }
        let (sql_expr, _) = parse_attribute(attr)?;
        return Some(format!("({} IS NULL OR {} = '')", sql_expr, sql_expr));
    }

    if let Some(caps) = CONTAINS_RE.captures(&s) {
        let attr = &caps[1];
        let arg = &caps[2];

        let field_name = if attr.starts_with("file.") {
            return None;
        } else if attr.starts_with("note.") {
            attr[5..].to_string()
        } else {
            attr.to_string()
        };

        let val = if arg.starts_with('"') || arg.starts_with('\'') {
            let inner = arg.trim_matches('"').trim_matches('\'');
            if inner.starts_with("[[") && inner.ends_with("]]") {
                inner.to_string()
            } else {
                unescape_string_literal(inner)
            }
        } else {
            arg.to_string()
        };

        return Some(format!(
            "list_contains((properties->'$.\"{}\"')::VARCHAR[], '{}')",
            field_name, val
        ));
    }

    warnings.push(format!(
        "WARN: unsupported filter '{}' in '{}', condition ignored.",
        s, base_name
    ));
    None
}

fn unescape_string_literal(s: &str) -> String {
    s.replace('\'', "''")
}

fn is_quoted_string(value: &str) -> bool {
    (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
}

fn normalize_pure_wikilink_literal(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.starts_with("[[") && trimmed.ends_with("]]") && trimmed.len() >= 4 {
        return parse_link_target(&trimmed[2..trimmed.len() - 2]).normalized_target;
    }
    trimmed.to_string()
}

fn normalize_pure_wikilink_sql(expr: &str) -> String {
    format!(
        concat!(
            "CASE ",
            "WHEN {expr} IS NULL THEN NULL ",
            "WHEN regexp_matches({expr}, '^\\s*\\[\\[[^\\]]+\\]\\]\\s*$') THEN ",
            "regexp_replace(",
            "regexp_replace(",
            "split_part(split_part(regexp_extract({expr}, '^\\s*\\[\\[([^\\]]+)\\]\\]\\s*$', 1), '|', 1), '#', 1), ",
            "'^.*/', ''",
            "), ",
            "'\\\\.md$', ''",
            ") ",
            "ELSE {expr} ",
            "END"
        ),
        expr = expr
    )
}

fn file_prop_to_col(prop: &str) -> Option<String> {
    match prop {
        "name" => Some("name".to_string()),
        "path" => Some("path".to_string()),
        "ext" => Some("ext".to_string()),
        "folder" => Some("folder".to_string()),
        "size" => Some("size".to_string()),
        "ctime" => Some("ctime".to_string()),
        "mtime" => Some("mtime".to_string()),
        "tags" => Some("tags".to_string()),
        "links" => Some("links".to_string()),
        "embeds" => Some("embeds".to_string()),
        _ => None,
    }
}

fn parse_attribute(attr: &str) -> Option<(String, bool)> {
    if attr.starts_with("file.") {
        let prop = &attr[5..];
        if let Some(col) = file_prop_to_col(prop) {
            return Some((col, false));
        }
    }
    if attr.starts_with("note.") {
        let field = &attr[5..];
        return Some((
            format!("json_extract_string(properties, '$.\"{}\"')", field),
            true,
        ));
    }
    let field = attr;
    Some((
        format!("json_extract_string(properties, '$.\"{}\"')", field),
        true,
    ))
}

fn translate_date_expr(expr: &str) -> Option<String> {
    let expr = expr.trim();

    if expr == "now()" {
        return Some("NOW()".to_string());
    }
    if expr == "today()" {
        return Some("CURRENT_DATE".to_string());
    }

    let re = Regex::new(r#"^(now\(\)|today\(\))\s*([+-])\s*(?:"|')?(\d+)\s*([a-zA-Z]+)(?:"|')?$"#)
        .unwrap();
    if let Some(caps) = re.captures(expr) {
        let base = &caps[1];
        let _sign = &caps[2];
        let num = &caps[3];
        let unit = &caps[4];

        let interval = translate_interval(num, unit)?;

        if base == "now()" {
            return Some(format!(
                "CAST(NOW() AS TIMESTAMP) - INTERVAL '{} {}'",
                num, interval
            ));
        } else {
            return Some(format!("CURRENT_DATE - INTERVAL '{} {}'", num, interval));
        }
    }

    None
}

fn translate_interval(_num: &str, unit: &str) -> Option<String> {
    let unit_lower = unit.to_lowercase();
    let interval_unit = match unit_lower.as_str() {
        "y" | "year" | "years" => "YEAR",
        "M" | "month" | "months" => "MONTH",
        "w" | "week" | "weeks" => "WEEK",
        "d" | "day" | "days" => "DAY",
        "h" | "hour" | "hours" => "HOUR",
        "m" | "minute" | "minutes" => "MINUTE",
        "s" | "second" | "seconds" => "SECOND",
        _ => return None,
    };
    Some(interval_unit.to_string())
}

pub fn merge_filters(
    global: Option<&Value>,
    view: Option<&Value>,
    this: &ThisContext,
    base_name: &str,
    warnings: &mut Vec<String>,
) -> Option<String> {
    let global_where = global.and_then(|f| translate_filter(f, this, base_name, warnings));
    let view_where = view.and_then(|f| translate_filter(f, this, base_name, warnings));

    match (global_where, view_where) {
        (Some(g), Some(v)) => Some(format!("({}) AND ({})", g, v)),
        (Some(g), None) => Some(g),
        (None, Some(v)) => Some(v),
        (None, None) => None,
    }
}

fn strip_prefix(col_name: &str) -> String {
    if let Some(stripped) = col_name.strip_prefix("file.") {
        stripped.to_string()
    } else if let Some(stripped) = col_name.strip_prefix("note.") {
        stripped.to_string()
    } else {
        col_name.to_string()
    }
}

pub fn translate_columns(
    order_vals: &[Value],
    properties: Option<&Value>,
    base_name: &str,
    warnings: &mut Vec<String>,
) -> Vec<ColumnMeta> {
    if order_vals.is_empty() {
        return vec![
            ColumnMeta {
                sql_expr: "name".to_string(),
                display_name: "name".to_string(),
                is_name_col: true,
                is_list_col: false,
            },
            ColumnMeta {
                sql_expr: "path".to_string(),
                display_name: "path".to_string(),
                is_name_col: false,
                is_list_col: false,
            },
            ColumnMeta {
                sql_expr: "mtime".to_string(),
                display_name: "mtime".to_string(),
                is_name_col: false,
                is_list_col: false,
            },
        ];
    }

    let mut columns = Vec::new();
    for val in order_vals {
        if let Value::String(col_name) = val {
            let col_name = col_name.as_str();

            if col_name.starts_with("formula.") {
                warnings.push(format!(
                    "WARN: unsupported column '{}' in '{}', column ignored.",
                    col_name, base_name
                ));
                continue;
            }

            let (sql_expr, is_list) = if col_name.starts_with("file.") {
                let prop = &col_name[5..];
                let is_list = matches!(prop, "tags" | "links" | "embeds");
                (
                    file_prop_to_col(prop).unwrap_or_else(|| col_name.to_string()),
                    is_list,
                )
            } else if col_name.starts_with("note.") {
                let field = &col_name[5..];
                (
                    format!("json_extract_string(properties, '$.\"{}\"')", field),
                    false,
                )
            } else {
                (
                    format!("json_extract_string(properties, '$.\"{}\"')", col_name),
                    false,
                )
            };

            let display_name = if let Some(props) = properties {
                if let Some(Value::Object(prop_obj)) = props.get(col_name) {
                    if let Some(display) = prop_obj.get("displayName").and_then(|v| v.as_str()) {
                        display.to_string()
                    } else {
                        strip_prefix(col_name)
                    }
                } else {
                    strip_prefix(col_name)
                }
            } else {
                strip_prefix(col_name)
            };

            let is_name_col = col_name == "file.name";

            columns.push(ColumnMeta {
                sql_expr,
                display_name,
                is_name_col,
                is_list_col: is_list,
            });
        }
    }

    if columns.is_empty() {
        return vec![
            ColumnMeta {
                sql_expr: "name".to_string(),
                display_name: "name".to_string(),
                is_name_col: true,
                is_list_col: false,
            },
            ColumnMeta {
                sql_expr: "path".to_string(),
                display_name: "path".to_string(),
                is_name_col: false,
                is_list_col: false,
            },
            ColumnMeta {
                sql_expr: "mtime".to_string(),
                display_name: "mtime".to_string(),
                is_name_col: false,
                is_list_col: false,
            },
        ];
    }

    columns
}

pub fn translate_sort(
    sort_val: Option<&Value>,
    base_name: &str,
    warnings: &mut Vec<String>,
) -> String {
    let Some(arr) = sort_val.and_then(|v| v.as_array()) else {
        return String::new();
    };

    if arr.is_empty() {
        return String::new();
    }

    let mut parts = Vec::new();
    for item in arr {
        if let Value::Object(obj) = item {
            let prop = obj.get("property").and_then(|v| v.as_str()).unwrap_or("");
            let dir = obj
                .get("direction")
                .and_then(|v| v.as_str())
                .unwrap_or("ASC");

            let (sql_expr, _) = if prop.starts_with("file.") {
                let p = &prop[5..];
                (
                    file_prop_to_col(p).unwrap_or_else(|| prop.to_string()),
                    false,
                )
            } else if prop.starts_with("note.") {
                let field = &prop[5..];
                (
                    format!("json_extract_string(properties, '$.\"{}\"')", field),
                    false,
                )
            } else {
                (
                    format!("json_extract_string(properties, '$.\"{}\"')", prop),
                    false,
                )
            };

            let direction = if dir.eq_ignore_ascii_case("DESC") {
                "DESC"
            } else {
                if !dir.eq_ignore_ascii_case("ASC") {
                    warnings.push(format!(
                        "WARN: invalid direction '{}' in '{}', defaulting to ASC.",
                        dir, base_name
                    ));
                }
                "ASC"
            };

            parts.push(format!("{} {}", sql_expr, direction));
        }
    }

    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ThisContext {
        ThisContext {
            path: "acme.md".to_string(),
            folder: "".to_string(),
            name: "acme".to_string(),
            ext: "md".to_string(),
            size: 100,
            ctime: "2025-01-01".to_string(),
            mtime: "2025-01-01".to_string(),
            tags: vec![],
            links: vec![],
        }
    }

    #[test]
    fn test_translate_link_this_equality() {
        let this = ctx();
        let mut warnings = Vec::new();
        let filter = serde_json::json!("related_customer == link(this)");
        let result = translate_filter(&filter, &this, "test.base", &mut warnings);
        let sql = result.unwrap();
        assert!(sql.contains("regexp_extract"));
        assert!(sql.contains("'acme'"));
        assert!(!sql.contains("'[[acme]]'"));
    }

    #[test]
    fn test_translate_frontmatter_link_field_matches_plain_note_name() {
        let this = ctx();
        let mut warnings = Vec::new();
        let filter = serde_json::json!("company == this.file.name");
        let result = translate_filter(&filter, &this, "test.base", &mut warnings);
        let sql = result.unwrap();
        assert!(sql.contains("regexp_extract"));
        assert!(sql.contains("'acme'"));
    }

    #[test]
    fn test_translate_has_link_this_file() {
        let this = ctx();
        let mut warnings = Vec::new();
        let filter = serde_json::json!("file.hasLink(this.file)");
        let result = translate_filter(&filter, &this, "test.base", &mut warnings);
        assert_eq!(result.unwrap(), "list_contains(links, 'acme')");
    }

    #[test]
    fn test_translate_file_name_equals_this_file_name() {
        let this = ctx();
        let mut warnings = Vec::new();
        let filter = serde_json::json!("file.name == this.file.name");
        let result = translate_filter(&filter, &this, "test.base", &mut warnings);
        assert_eq!(result.unwrap(), "name == 'acme'");
    }

    #[test]
    fn test_translate_has_tag_single() {
        let this = ctx();
        let mut warnings = Vec::new();
        let filter = serde_json::json!("file.hasTag(\"t1\")");
        let result = translate_filter(&filter, &this, "test.base", &mut warnings);
        let r = result.unwrap();
        assert!(r.contains("list_contains(tags, 't1')"));
        assert!(r.contains("array_any(tags, x -> x LIKE 't1%')"));
    }

    #[test]
    fn test_translate_has_tag_multi() {
        let this = ctx();
        let mut warnings = Vec::new();
        let filter = serde_json::json!("file.hasTag(\"t1\", \"t2\")");
        let result = translate_filter(&filter, &this, "test.base", &mut warnings);
        let r = result.unwrap();
        assert!(r.contains("OR"));
    }

    #[test]
    fn test_translate_in_folder_no_slash() {
        let this = ctx();
        let mut warnings = Vec::new();
        let filter = serde_json::json!("file.inFolder(\"notes\")");
        let result = translate_filter(&filter, &this, "test.base", &mut warnings);
        assert!(result.unwrap().contains("folder = 'notes/'"));
    }

    #[test]
    fn test_translate_date_30d() {
        let this = ctx();
        let mut warnings = Vec::new();
        let filter = serde_json::json!("file.ctime > now() - 30d");
        let result = translate_filter(&filter, &this, "test.base", &mut warnings);
        assert!(result.unwrap().contains("INTERVAL '30 DAY'"));
    }

    #[test]
    fn test_translate_is_empty_note() {
        let this = ctx();
        let mut warnings = Vec::new();
        let filter = serde_json::json!("note.status.isEmpty()");
        let result = translate_filter(&filter, &this, "test.base", &mut warnings);
        assert!(result.unwrap().contains("IS NULL OR"));
    }

    #[test]
    fn test_translate_is_empty_file_tags() {
        let this = ctx();
        let mut warnings = Vec::new();
        let filter = serde_json::json!("file.tags.isEmpty()");
        let result = translate_filter(&filter, &this, "test.base", &mut warnings);
        assert!(result.unwrap().contains("len(tags) = 0"));
    }

    #[test]
    fn test_translate_bare_equality() {
        let this = ctx();
        let mut warnings = Vec::new();
        let filter = serde_json::json!("status == \"done\"");
        let result = translate_filter(&filter, &this, "test.base", &mut warnings);
        assert!(result.unwrap().contains("json_extract_string"));
    }

    #[test]
    fn test_translate_nested_and_in_and() {
        let this = ctx();
        let mut warnings = Vec::new();
        let filter = serde_json::json!({
            "and": [
                "status == \"done\"",
                {"and": ["type == \"a\"", "priority == \"high\""]}
            ]
        });
        let result = translate_filter(&filter, &this, "test.base", &mut warnings);
        let r = result.unwrap();
        assert!(r.contains("AND"));
        assert!(r.contains("status"));
        assert!(r.contains("type"));
    }

    #[test]
    fn test_merge_filters_both() {
        let this = ctx();
        let mut warnings = Vec::new();
        let global = serde_json::json!("status == \"active\"");
        let view = serde_json::json!("priority == \"high\"");
        let result = merge_filters(
            Some(&global),
            Some(&view),
            &this,
            "test.base",
            &mut warnings,
        );
        let r = result.unwrap();
        assert!(r.contains("AND"));
    }

    #[test]
    fn test_translate_columns_empty_uses_defaults() {
        let mut warnings = Vec::new();
        let cols = translate_columns(&[], None, "test.base", &mut warnings);
        assert_eq!(cols.len(), 3);
        assert!(cols[0].is_name_col);
    }

    #[test]
    fn test_translate_columns_bare_name_is_json_extract() {
        let mut warnings = Vec::new();
        let cols = translate_columns(
            &[serde_json::json!("name")],
            None,
            "test.base",
            &mut warnings,
        );
        assert!(cols[0].sql_expr.contains("json_extract_string"));
        assert!(!cols[0].sql_expr.starts_with("name"));
    }

    #[test]
    fn test_translate_columns_file_tags_is_list() {
        let mut warnings = Vec::new();
        let cols = translate_columns(
            &[serde_json::json!("file.tags")],
            None,
            "test.base",
            &mut warnings,
        );
        assert!(cols[0].is_list_col);
    }

    #[test]
    fn test_translate_sort_basic() {
        let mut warnings = Vec::new();
        let sort = serde_json::json!([
            {"property": "file.name", "direction": "DESC"}
        ]);
        let result = translate_sort(Some(&sort), "test.base", &mut warnings);
        assert!(!result.is_empty());
        assert!(result.contains("DESC"));
    }

    #[test]
    fn test_translate_sort_bare_property() {
        let mut warnings = Vec::new();
        let sort = serde_json::json!([
            {"property": "stage", "direction": "ASC"}
        ]);
        let result = translate_sort(Some(&sort), "test.base", &mut warnings);
        assert!(result.contains("json_extract_string"));
    }

    #[test]
    fn test_translate_bare_column_not_direct_db_col() {
        let cols = translate_columns(
            &[serde_json::json!("name")],
            None,
            "t.base",
            &mut Vec::new(),
        );
        assert!(cols[0].sql_expr.contains("json_extract_string"));
        assert!(!cols[0].is_name_col);
    }

    #[test]
    fn test_translate_sort_invalid_direction_warns_and_defaults_asc() {
        let sort = serde_json::json!([{"property": "file.name", "direction": "INVALID"}]);
        let mut warnings = vec![];
        let result = translate_sort(Some(&sort), "t.base", &mut warnings);
        assert!(result.contains("ASC"));
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_sql_injection_single_quote_escaping() {
        let mut ctx = ctx();
        ctx.name = "O'Brien".to_string();
        let filter = serde_json::json!("related_customer == link(this)");
        let mut warnings = vec![];
        let result = translate_filter(&filter, &ctx, "t.base", &mut warnings);
        let sql = result.unwrap();
        assert!(
            sql.contains("O''Brien"),
            "Single quote must be escaped: {}",
            sql
        );
    }

    #[test]
    fn test_empty_and_array_returns_none() {
        let filter = serde_json::json!({"and": []});
        let mut warnings = vec![];
        let result = translate_filter(&filter, &ctx(), "t.base", &mut warnings);
        assert!(result.is_none());
    }

    #[test]
    fn test_views_empty_silently_skipped() {
        let cols = translate_columns(&[], None, "t.base", &mut Vec::new());
        assert_eq!(cols.len(), 3);
    }
}
