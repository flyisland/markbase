pub mod compiler;
pub mod parser;
pub mod tokenizer;

pub use compiler::build_sql;

pub fn output_results(
    results: &[Vec<String>],
    format: &str,
    field_names: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    match format {
        "json" | "Json" => output_json(results, field_names),
        "list" | "List" => output_list(results, field_names),
        _ => output_table(results, field_names),
    }
}

fn output_json(
    results: &[Vec<String>],
    field_names: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let json_results: Vec<serde_json::Value> = results
        .iter()
        .map(|row| {
            let obj: serde_json::Map<String, serde_json::Value> = row
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let name = field_names
                        .get(i)
                        .map_or_else(|| format!("col{}", i), |name| name.clone());
                    (name, serde_json::Value::String(v.clone()))
                })
                .collect();
            serde_json::Value::Object(obj)
        })
        .collect();

    let output = serde_json::json!({
        "metadata": {
            "count": results.len()
        },
        "results": json_results
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn output_list(
    results: &[Vec<String>],
    field_names: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    for row in results {
        for (i, col) in row.iter().enumerate() {
            let name = field_names
                .get(i)
                .map_or_else(|| format!("col{}", i), |name| name.clone());
            println!("{}: {}", name, col);
        }
        println!("---");
    }
    println!("\n{} results", results.len());
    Ok(())
}

fn output_table(
    results: &[Vec<String>],
    field_names: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if results.is_empty() {
        return Ok(());
    }

    let col_count = results[0].len();
    let display_names: Vec<String> = (0..col_count)
        .map(|i| {
            field_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("col{}", i))
        })
        .collect();

    let col_widths: Vec<usize> = (0..col_count)
        .map(|i| {
            let name_width = display_names[i].len();
            let data_width = results
                .iter()
                .map(|row| row.get(i).map_or(0, |s| s.len()))
                .max()
                .unwrap_or(0);
            name_width.max(data_width).max(10)
        })
        .collect();

    let header_cells: Vec<String> = display_names
        .iter()
        .enumerate()
        .map(|(i, name)| format!("{:<width$}", name, width = col_widths[i]))
        .collect();
    println!("{}", header_cells.join(" | "));

    let separator: String = col_widths
        .iter()
        .map(|w| "-".repeat(*w))
        .collect::<Vec<_>>()
        .join("-+-");
    println!("{}", separator);

    for row in results {
        let cells: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                if i < col_widths.len() {
                    format!("{:<width$}", cell, width = col_widths[i])
                } else {
                    cell.clone()
                }
            })
            .collect();
        println!("{}", cells.join(" | "));
    }
    println!("\n{} results", results.len());
    Ok(())
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
        let result = output_results(&results, "table", &fields);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_results_json() {
        let results = vec![
            vec!["path1".to_string(), "name1".to_string()],
            vec!["path2".to_string(), "name2".to_string()],
        ];
        let fields = vec!["path".to_string(), "name".to_string()];
        let result = output_results(&results, "json", &fields);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_results_list() {
        let results = vec![
            vec!["path1".to_string(), "name1".to_string()],
            vec!["path2".to_string(), "name2".to_string()],
        ];
        let fields = vec!["path".to_string(), "name".to_string()];
        let result = output_results(&results, "list", &fields);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_results_empty() {
        let results: Vec<Vec<String>> = vec![];
        let fields = vec!["path".to_string()];
        let result = output_results(&results, "table", &fields);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_results_default_to_table() {
        let results = vec![vec!["test".to_string()]];
        let fields = vec!["col0".to_string()];
        let result = output_results(&results, "unknown_format", &fields);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_json_structure() {
        let results = vec![vec!["val1".to_string(), "val2".to_string()]];
        let fields = vec!["field1".to_string(), "field2".to_string()];
        let result = output_json(&results, &fields);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_list_structure() {
        let results = vec![vec!["path".to_string(), "name".to_string()]];
        let fields = vec!["path".to_string(), "name".to_string()];
        let result = output_list(&results, &fields);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_table_calculates_widths() {
        let results = vec![
            vec!["short".to_string(), "longer_value".to_string()],
            vec!["a".to_string(), "b".to_string()],
        ];
        let fields = vec!["col1".to_string(), "col2".to_string()];
        let result = output_table(&results, &fields);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_table_empty_results() {
        let results: Vec<Vec<String>> = vec![];
        let fields = vec!["path".to_string()];
        let result = output_table(&results, &fields);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_multiple_rows() {
        let results = vec![
            vec![
                "path1".to_string(),
                "name1".to_string(),
                "content1".to_string(),
            ],
            vec![
                "path2".to_string(),
                "name2".to_string(),
                "content2".to_string(),
            ],
            vec![
                "path3".to_string(),
                "name3".to_string(),
                "content3".to_string(),
            ],
        ];
        let fields = vec![
            "path".to_string(),
            "name".to_string(),
            "content".to_string(),
        ];

        for format in &["table", "json", "list"] {
            let result = output_results(&results, format, &fields);
            assert!(result.is_ok(), "Failed for format: {}", format);
        }
    }
}
