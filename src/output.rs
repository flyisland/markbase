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
}

pub fn render_yaml_records(headers: &[String], rows: &[Vec<OutputValue>]) -> String {
    if rows.is_empty() {
        return "[]\n".to_string();
    }

    let mut output = String::new();
    for row in rows {
        for (index, header) in headers.iter().enumerate() {
            let prefix = if index == 0 { "- " } else { "  " };
            match row.get(index).unwrap_or(&OutputValue::Empty) {
                OutputValue::Empty => {
                    output.push_str(prefix);
                    output.push_str(header);
                    output.push_str(":\n");
                }
                OutputValue::Scalar(value) => {
                    output.push_str(prefix);
                    output.push_str(header);
                    output.push_str(": ");
                    output.push_str(&yaml_scalar(value));
                    output.push('\n');
                }
                OutputValue::List(items) => {
                    output.push_str(prefix);
                    output.push_str(header);
                    output.push_str(":\n");
                    for item in items {
                        output.push_str("    - ");
                        output.push_str(&yaml_scalar(item));
                        output.push('\n');
                    }
                }
            }
        }
    }

    output
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

fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', "<br>")
}

fn yaml_scalar(value: &str) -> String {
    match serde_yaml::to_string(value) {
        Ok(rendered) => rendered
            .strip_prefix("---\n")
            .unwrap_or(&rendered)
            .trim_end()
            .to_string(),
        Err(_) => format!("'{}'", value.replace('\'', "''")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_yaml_records_empty() {
        let output = render_yaml_records(&["name".to_string()], &[]);
        assert_eq!(output, "[]\n");
    }

    #[test]
    fn test_render_yaml_records_scalars_and_lists() {
        let headers = vec!["name".to_string(), "tags".to_string()];
        let rows = vec![vec![
            OutputValue::Scalar("demo".to_string()),
            OutputValue::List(vec!["alpha".to_string(), "beta".to_string()]),
        ]];

        let output = render_yaml_records(&headers, &rows);
        assert_eq!(output, "- name: demo\n  tags:\n    - alpha\n    - beta\n");
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
