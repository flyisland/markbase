pub mod detector;
pub mod error_map;
pub mod executor;
pub mod translator;

use std::path::Path;

use crate::output::{OutputValue, render_json_records, render_markdown_table};

pub use executor::{execute_query, translate_query};

pub fn output_results(
    results: &[Vec<String>],
    format: &str,
    field_names: &[String],
    base_dir: Option<&Path>,
    abs_path: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let results = if abs_path {
        if let Some(dir) = base_dir {
            convert_to_absolute_paths(results, field_names, dir)
        } else {
            results.to_vec()
        }
    } else {
        results.to_vec()
    };

    print!("{}", format_results(&results, format, field_names));
    Ok(())
}

fn format_results(results: &[Vec<String>], format: &str, field_names: &[String]) -> String {
    let rows = to_output_rows(results);
    match format {
        "table" | "Table" => render_markdown_table(field_names, &rows),
        _ => render_json_records(field_names, &rows),
    }
}

fn to_output_rows(results: &[Vec<String>]) -> Vec<Vec<OutputValue>> {
    results
        .iter()
        .map(|row| row.iter().map(|value| parse_output_value(value)).collect())
        .collect()
}

fn parse_output_value(value: &str) -> OutputValue {
    if let Some(items) = parse_array_list(value) {
        OutputValue::List(items)
    } else {
        OutputValue::Scalar(value.to_string())
    }
}

fn parse_array_list(value: &str) -> Option<Vec<String>> {
    let trimmed = value.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return None;
    }

    let inner = &trimmed[1..trimmed.len() - 1];
    if inner.trim().is_empty() {
        return Some(vec![]);
    }

    Some(
        inner
            .split(',')
            .map(|item| item.trim().trim_matches('"').to_string())
            .collect(),
    )
}

fn convert_to_absolute_paths(
    results: &[Vec<String>],
    field_names: &[String],
    base_dir: &Path,
) -> Vec<Vec<String>> {
    results
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(i, value)| {
                    let name = field_names.get(i).map_or("", |s| s.as_str());
                    if name == "file.path" || name == "file.folder" {
                        let abs = base_dir.join(value);
                        abs.to_string_lossy().to_string()
                    } else {
                        value.clone()
                    }
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_output_results_table() {
        let results = vec![vec!["notes/a.md".to_string(), "a".to_string()]];
        let fields = vec!["path".to_string(), "name".to_string()];
        let result = output_results(&results, "table", &fields, None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_format_results_json() {
        let results = vec![vec![
            "readme".to_string(),
            "[\"docs\",\"important\"]".to_string(),
        ]];
        let fields = vec!["file.name".to_string(), "file.tags".to_string()];
        let output = format_results(&results, "json", &fields);
        let actual: serde_json::Value = serde_json::from_str(&output).unwrap();
        let expected = json!([
            {
                "file.name": "readme",
                "file.tags": ["docs", "important"]
            }
        ]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_format_results_empty_json() {
        let results = vec![];
        let fields = vec!["name".to_string()];
        let output = format_results(&results, "json", &fields);
        assert_eq!(output, "[]");
    }

    #[test]
    fn test_format_results_table() {
        let results = vec![vec!["Alice".to_string(), "Engineer".to_string()]];
        let fields = vec!["name".to_string(), "title".to_string()];
        let output = format_results(&results, "table", &fields);
        assert_eq!(
            output,
            "| name | title |\n| --- | --- |\n| Alice | Engineer |\n"
        );
    }

    #[test]
    fn test_unknown_format_defaults_to_json() {
        let results = vec![vec!["Alice".to_string()]];
        let fields = vec!["name".to_string()];
        let output = format_results(&results, "unknown_format", &fields);
        assert_eq!(output, "[\n  {\n    \"name\": \"Alice\"\n  }\n]");
    }

    #[test]
    fn test_convert_to_absolute_paths() {
        let results = vec![vec!["notes/a.md".to_string(), "a".to_string()]];
        let fields = vec!["file.path".to_string(), "file.name".to_string()];
        let base_dir = Path::new("/vault");
        let converted = convert_to_absolute_paths(&results, &fields, base_dir);
        assert_eq!(converted[0][0], "/vault/notes/a.md");
        assert_eq!(converted[0][1], "a");
    }
}
