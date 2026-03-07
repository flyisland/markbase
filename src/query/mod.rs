pub mod detector;
pub mod error_map;
pub mod executor;
pub mod translator;

use std::path::Path;

use crate::output::{OutputValue, render_markdown_table, render_yaml_records};

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
        "list" | "List" => render_yaml_records(field_names, &rows),
        _ => render_markdown_table(field_names, &rows),
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

    #[test]
    fn test_output_results_table() {
        let results = vec![
            vec!["path1".to_string(), "name1".to_string()],
            vec!["path2".to_string(), "name2".to_string()],
        ];
        let fields = vec!["path".to_string(), "name".to_string()];
        let result = output_results(&results, "table", &fields, None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_results_list() {
        let results = vec![
            vec!["path1".to_string(), "name1".to_string()],
            vec!["path2".to_string(), "name2".to_string()],
        ];
        let fields = vec!["path".to_string(), "name".to_string()];
        let result = output_results(&results, "list", &fields, None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_results_empty_list() {
        let results: Vec<Vec<String>> = vec![];
        let fields = vec!["path".to_string()];
        let output = format_results(&results, "list", &fields);
        assert_eq!(output, "[]\n");
    }

    #[test]
    fn test_output_results_empty_table() {
        let results: Vec<Vec<String>> = vec![];
        let fields = vec!["path".to_string()];
        let output = format_results(&results, "table", &fields);
        assert_eq!(output, "| path |\n| --- |\n");
    }

    #[test]
    fn test_output_results_default_to_table() {
        let results = vec![vec!["test".to_string()]];
        let fields = vec!["col0".to_string()];
        let output = format_results(&results, "unknown_format", &fields);
        assert_eq!(output, "| col0 |\n| --- |\n| test |\n");
    }

    #[test]
    fn test_output_list_structure_is_yaml() {
        let results = vec![vec!["path".to_string(), "[tag1, tag2]".to_string()]];
        let fields = vec!["path".to_string(), "tags".to_string()];
        let output = format_results(&results, "list", &fields);
        assert_eq!(output, "- path: path\n  tags:\n    - tag1\n    - tag2\n");
    }

    #[test]
    fn test_output_table_is_markdown() {
        let results = vec![
            vec!["short".to_string(), "longer_value".to_string()],
            vec!["a".to_string(), "b".to_string()],
        ];
        let fields = vec!["col1".to_string(), "col2".to_string()];
        let output = format_results(&results, "table", &fields);
        assert_eq!(
            output,
            "| col1 | col2 |\n| --- | --- |\n| short | longer_value |\n| a | b |\n"
        );
    }

    #[test]
    fn test_output_multiple_rows() {
        let results = vec![
            vec![
                "path1".to_string(),
                "name1".to_string(),
                "tags1".to_string(),
            ],
            vec![
                "path2".to_string(),
                "name2".to_string(),
                "tags2".to_string(),
            ],
            vec![
                "path3".to_string(),
                "name3".to_string(),
                "tags3".to_string(),
            ],
        ];
        let fields = vec!["path".to_string(), "name".to_string(), "tags".to_string()];

        for format in &["table", "list"] {
            let result = output_results(&results, format, &fields, None, false);
            assert!(result.is_ok(), "Failed for format: {}", format);
        }
    }

    #[test]
    fn test_abs_path_converts_path_and_folder() {
        let base_dir = std::path::PathBuf::from("/base");
        let results = vec![
            vec!["notes/test.md".to_string(), "notes".to_string()],
            vec!["notes/other.md".to_string(), "notes".to_string()],
        ];
        let fields = vec!["file.path".to_string(), "file.folder".to_string()];

        let converted = convert_to_absolute_paths(&results, &fields, &base_dir);
        assert_eq!(converted[0][0], "/base/notes/test.md");
        assert_eq!(converted[0][1], "/base/notes");
    }
}
