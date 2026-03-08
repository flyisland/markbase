use crate::output::{OutputValue, render_json_records, render_markdown_table};

#[derive(Debug, Clone)]
pub struct ColumnMeta {
    pub sql_expr: String,
    pub display_name: String,
    pub is_name_col: bool,
    pub is_list_col: bool,
}

pub type Row = Vec<(String, Option<String>)>;

pub fn render_json(rows: &[Row], columns: &[ColumnMeta]) -> String {
    let headers: Vec<String> = columns.iter().map(|col| col.display_name.clone()).collect();
    let records = to_output_rows(rows, columns);
    render_json_records(&headers, &records)
}

pub fn render_table(rows: &[Row], columns: &[ColumnMeta]) -> String {
    let headers: Vec<String> = columns.iter().map(|col| col.display_name.clone()).collect();
    let records = to_output_rows(rows, columns);
    render_markdown_table(&headers, &records)
}

fn to_output_rows(rows: &[Row], columns: &[ColumnMeta]) -> Vec<Vec<OutputValue>> {
    rows.iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(i, (_display_name, value))| match value {
                    Some(val) if columns[i].is_name_col => {
                        OutputValue::Scalar(format!("[[{}]]", val))
                    }
                    Some(val) if columns[i].is_list_col => {
                        OutputValue::List(parse_array_list(val.as_str()))
                    }
                    Some(val) => OutputValue::Scalar(val.clone()),
                    None => OutputValue::Empty,
                })
                .collect()
        })
        .collect()
}

fn parse_array_list(s: &str) -> Vec<String> {
    let trimmed = s.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return vec![s.to_string()];
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    if inner.trim().is_empty() {
        return vec![];
    }

    inner
        .split(',')
        .map(|item| item.trim().trim_matches('"').to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
    fn test_render_json_basic() {
        let columns = vec![col("name", false, false), col("title", false, false)];
        let rows = vec![row(vec![
            ("name", Some("John")),
            ("title", Some("Engineer")),
        ])];
        let out = render_json(&rows, &columns);
        let actual: serde_json::Value = serde_json::from_str(&out).unwrap();
        let expected = json!([{"name": "John", "title": "Engineer"}]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_render_json_name_col() {
        let columns = vec![col("name", true, false)];
        let rows = vec![row(vec![("name", Some("test-note"))])];
        let out = render_json(&rows, &columns);
        assert_eq!(out, "[\n  {\n    \"name\": \"[[test-note]]\"\n  }\n]");
    }

    #[test]
    fn test_render_json_list_col() {
        let columns = vec![col("tags", false, true)];
        let rows = vec![row(vec![("tags", Some("[\"tag1\",\"tag2\",\"tag3\"]"))])];
        let out = render_json(&rows, &columns);
        let actual: serde_json::Value = serde_json::from_str(&out).unwrap();
        let expected = json!([{"tags": ["tag1", "tag2", "tag3"]}]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_render_json_empty() {
        let columns = vec![col("name", false, false)];
        let rows: Vec<Row> = vec![];
        let out = render_json(&rows, &columns);
        assert_eq!(out, "[]");
    }

    #[test]
    fn test_render_table_basic() {
        let columns = vec![col("name", false, false), col("title", false, false)];
        let rows = vec![row(vec![
            ("name", Some("John")),
            ("title", Some("Engineer")),
        ])];
        let out = render_table(&rows, &columns);
        assert_eq!(
            out,
            "| name | title |\n| --- | --- |\n| John | Engineer |\n"
        );
    }

    #[test]
    fn test_render_table_list_col() {
        let columns = vec![col("tags", false, true)];
        let rows = vec![row(vec![("tags", Some("[\"tag1\",\"tag2\"]"))])];
        let out = render_table(&rows, &columns);
        assert_eq!(out, "| tags |\n| --- |\n| tag1, tag2 |\n");
    }

    #[test]
    fn test_render_table_empty() {
        let columns = vec![col("name", false, false)];
        let rows: Vec<Row> = vec![];
        let out = render_table(&rows, &columns);
        assert_eq!(out, "| name |\n| --- |\n");
    }
}
