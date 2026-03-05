#[derive(Debug, Clone)]
pub struct ColumnMeta {
    pub sql_expr: String,
    pub display_name: String,
    pub is_name_col: bool,
    pub is_list_col: bool,
}

pub type Row = Vec<(String, Option<String>)>;

pub fn render_list(rows: &[Row], columns: &[ColumnMeta]) -> String {
    if rows.is_empty() {
        return "(no results)\n".to_string();
    }

    let mut output = String::new();
    for row in rows {
        output.push_str("---\n");
        for (i, (display_name, value)) in row.iter().enumerate() {
            if let Some(val) = value {
                if columns[i].is_name_col {
                    output.push_str(&format!("{}: [[{}]]\n", display_name, val));
                } else if columns[i].is_list_col {
                    output.push_str(&format!("{}:\n", display_name));
                    let items = parse_array_list(val);
                    for item in items {
                        output.push_str(&format!("  - {}\n", item));
                    }
                } else {
                    output.push_str(&format!("{}: {}\n", display_name, val));
                }
            } else {
                output.push_str(&format!("{}:\n", display_name));
            }
        }
    }
    output
}

fn parse_array_list(s: &str) -> Vec<String> {
    let trimmed = s.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return vec![s.to_string()];
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    if inner.is_empty() {
        return vec![];
    }
    inner.split(", ").map(|s| s.to_string()).collect()
}

pub fn render_table(rows: &[Row], columns: &[ColumnMeta]) -> String {
    let mut output = String::new();

    output.push('|');
    for col in columns {
        output.push_str(&format!(" {} |", col.display_name));
    }
    output.push('\n');

    output.push('|');
    for _ in columns {
        output.push_str("---|");
    }
    output.push('\n');

    if rows.is_empty() {
        output.push('|');
        for _ in columns {
            output.push_str(" (no results) |");
        }
        output.push('\n');
    } else {
        for row in rows {
            output.push('|');
            for (i, (_display_name, value)) in row.iter().enumerate() {
                output.push(' ');
                if let Some(val) = value {
                    if columns[i].is_name_col {
                        output.push_str(&format!("[[{}]]", val));
                    } else if columns[i].is_list_col {
                        let items = parse_array_list(val);
                        output.push_str(&items.join(", "));
                    } else {
                        output.push_str(val);
                    }
                }
                output.push_str(" |");
            }
            output.push('\n');
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(name: &str, is_name: bool, is_list: bool) -> ColumnMeta {
        ColumnMeta {
            sql_expr: name.to_string(),
            display_name: name.to_string(),
            is_name_col: is_name,
            is_list_col: is_list,
        }
    }

    fn row(pairs: Vec<(&str, Option<&str>)>) -> Row {
        pairs
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.map(|s| s.to_string())))
            .collect()
    }

    #[test]
    fn test_render_list_basic() {
        let columns = vec![col("name", false, false), col("title", false, false)];
        let rows = vec![row(vec![
            ("name", Some("John")),
            ("title", Some("Engineer")),
        ])];
        let out = render_list(&rows, &columns);
        assert!(out.contains("name: John"));
        assert!(out.contains("title: Engineer"));
    }

    #[test]
    fn test_render_list_name_col() {
        let columns = vec![col("name", true, false)];
        let rows = vec![row(vec![("name", Some("test-note"))])];
        let out = render_list(&rows, &columns);
        assert!(out.contains("name: [[test-note]]"));
    }

    #[test]
    fn test_render_list_list_col() {
        let columns = vec![col("tags", false, true)];
        let rows = vec![row(vec![("tags", Some("[tag1, tag2, tag3]"))])];
        let out = render_list(&rows, &columns);
        assert!(out.contains("tags:"));
        assert!(out.contains("  - tag1"));
        assert!(out.contains("  - tag2"));
        assert!(out.contains("  - tag3"));
    }

    #[test]
    fn test_render_list_empty() {
        let columns = vec![col("name", false, false)];
        let rows: Vec<Row> = vec![];
        let out = render_list(&rows, &columns);
        assert_eq!(out, "(no results)\n");
    }

    #[test]
    fn test_render_table_basic() {
        let columns = vec![col("name", false, false), col("title", false, false)];
        let rows = vec![row(vec![
            ("name", Some("John")),
            ("title", Some("Engineer")),
        ])];
        let out = render_table(&rows, &columns);
        assert!(out.contains("| name | title |"));
        assert!(out.contains("| John | Engineer |"));
    }

    #[test]
    fn test_render_table_list_col() {
        let columns = vec![col("tags", false, true)];
        let rows = vec![row(vec![("tags", Some("[tag1, tag2]"))])];
        let out = render_table(&rows, &columns);
        assert!(out.contains("| tag1, tag2 |"));
    }

    #[test]
    fn test_render_table_empty() {
        let columns = vec![col("name", false, false)];
        let rows: Vec<Row> = vec![];
        let out = render_table(&rows, &columns);
        assert!(out.contains("(no results)"));
    }
}
