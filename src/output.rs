use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputValue {
    Empty,
    Scalar(String),
    List(Vec<String>),
}

impl OutputValue {
    fn to_table_cell(&self) -> String {
        match self {
            Self::Empty => String::new(),
            Self::Scalar(value) => escape_markdown_cell(value),
            Self::List(items) => escape_markdown_cell(&items.join(", ")),
        }
    }

    fn to_json_value(&self) -> Value {
        match self {
            Self::Empty => Value::Null,
            Self::Scalar(value) => Value::String(value.clone()),
            Self::List(items) => {
                Value::Array(items.iter().cloned().map(Value::String).collect::<Vec<_>>())
            }
        }
    }
}

pub fn render_markdown_table(headers: &[String], rows: &[Vec<OutputValue>]) -> String {
    if headers.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    output.push('|');
    for header in headers {
        output.push(' ');
        output.push_str(&escape_markdown_cell(header));
        output.push_str(" |");
    }
    output.push('\n');

    output.push('|');
    for _ in headers {
        output.push_str(" --- |");
    }
    output.push('\n');

    for row in rows {
        output.push('|');
        for index in 0..headers.len() {
            output.push(' ');
            output.push_str(
                &row.get(index)
                    .unwrap_or(&OutputValue::Empty)
                    .to_table_cell(),
            );
            output.push_str(" |");
        }
        output.push('\n');
    }

    output
}

pub fn render_json_records(headers: &[String], rows: &[Vec<OutputValue>]) -> String {
    let records: Vec<Value> = rows
        .iter()
        .map(|row| {
            let object = headers
                .iter()
                .enumerate()
                .map(|(index, header)| {
                    let value = row
                        .get(index)
                        .unwrap_or(&OutputValue::Empty)
                        .to_json_value();
                    (header.clone(), value)
                })
                .collect::<Map<String, Value>>();
            Value::Object(object)
        })
        .collect();

    serde_json::to_string_pretty(&records).unwrap_or_else(|_| "[]".to_string())
}

fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', "<br>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_render_json_records_empty() {
        let output = render_json_records(&["name".to_string()], &[]);
        assert_eq!(output, "[]");
    }

    #[test]
    fn test_render_json_records_scalars_and_lists() {
        let headers = vec![
            "name".to_string(),
            "tags".to_string(),
            "description".to_string(),
        ];
        let rows = vec![vec![
            OutputValue::Scalar("demo".to_string()),
            OutputValue::List(vec!["alpha".to_string(), "beta".to_string()]),
            OutputValue::Empty,
        ]];

        let output = render_json_records(&headers, &rows);
        let actual: Value = serde_json::from_str(&output).unwrap();
        let expected = json!([
            {
                "name": "demo",
                "tags": ["alpha", "beta"],
                "description": null
            }
        ]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_render_markdown_table_compact() {
        let headers = vec!["name".to_string(), "title".to_string()];
        let rows = vec![vec![
            OutputValue::Scalar("John".to_string()),
            OutputValue::Scalar("Engineer".to_string()),
        ]];

        let output = render_markdown_table(&headers, &rows);
        assert_eq!(
            output,
            "| name | title |\n| --- | --- |\n| John | Engineer |\n"
        );
    }

    #[test]
    fn test_render_markdown_table_escapes_pipes() {
        let headers = vec!["title".to_string()];
        let rows = vec![vec![OutputValue::Scalar("a | b".to_string())]];

        let output = render_markdown_table(&headers, &rows);
        assert!(output.contains("a \\| b"));
    }
}
