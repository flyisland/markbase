pub fn map_error(error: &str, original_input: &str) -> String {
    if error.contains("Conversion Error") {
        return map_conversion_error(error, original_input);
    }

    if error.contains("Column") && error.contains("not found") {
        return map_column_not_found(error, original_input);
    }

    if error.contains("json") && error.contains("path") {
        return map_json_path_error(error, original_input);
    }

    if error.contains("Parse") && error.contains("error") {
        return map_syntax_error(error, original_input);
    }

    if error.contains("Binder") && error.contains("error") {
        return map_binder_error(error, original_input);
    }

    error.to_string()
}

fn map_conversion_error(error: &str, _original_input: &str) -> String {
    if let Some(start) = error.find("'") {
        let rest = &error[start + 1..];
        if let Some(end) = rest.find("'") {
            let value = &rest[..end];
            if let Some(field_start) = error.rfind("for column ") {
                let field_part = &error[field_start + 11..];
                return format!(
                    "Error: cannot convert value '{}' for field '{}', expected type mismatch. Use explicit cast: {}::TYPE",
                    value,
                    field_part.split_whitespace().next().unwrap_or("field"),
                    value
                );
            }
        }
    }

    "Error: cannot convert value, type mismatch. If comparing with a number, use explicit cast like year::INTEGER >= 2024".to_string()
}

fn map_column_not_found(error: &str, _original_input: &str) -> String {
    if let Some(start) = error.find("Column ") {
        let rest = &error[start + 7..];
        if let Some(end) = rest.find(" not") {
            let column = rest[..end].trim();
            if column.contains('.') || column.starts_with('$') {
                return format!(
                    "Error: invalid JSON path '{}', check the syntax e.g. _schema.strict",
                    column
                );
            }
            return format!(
                "Error: unknown field '{}'. Use file.* prefix for file properties (file.name, file.mtime), or note.* prefix for frontmatter (note.author). Bare identifiers reference frontmatter by default.",
                column
            );
        }
    }

    "Error: unknown field. Use file.* prefix for file properties, note.* prefix for frontmatter"
        .to_string()
}

fn map_json_path_error(error: &str, _original_input: &str) -> String {
    if error.contains("Invalid path") || error.contains("invalid escape") {
        return "Error: invalid nested property path, check the syntax e.g. _schema.strict"
            .to_string();
    }

    "Error: invalid JSON path syntax in field name".to_string()
}

fn map_syntax_error(error: &str, original_input: &str) -> String {
    if error.contains("syntax error") {
        if original_input.is_empty() {
            return "Error: empty query".to_string();
        }
        return format!(
            "Error: syntax error in query '{}', check for missing quotes, parentheses, or operators",
            original_input
        );
    }

    error.to_string()
}

fn map_binder_error(error: &str, original_input: &str) -> String {
    if error.contains("type mismatch") {
        return "Error: type mismatch. For frontmatter fields, use explicit casts like year::INTEGER".to_string();
    }

    format!("Error: query binding error in '{}'", original_input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_error_mapping() {
        let error = "Conversion Error: Could not convert string 'abc' to integer";
        let mapped = map_error(error, "year == 'abc'");
        assert!(mapped.contains("cannot convert"));
    }

    #[test]
    fn test_column_not_found_mapping() {
        let error = "Column author not found";
        let mapped = map_error(error, "author == 'Tom'");
        assert!(mapped.contains("unknown field"));
    }

    #[test]
    fn test_json_path_error_mapping() {
        let error = "Invalid JSON path";
        let mapped = map_error(error, "_schema.strict");
        assert!(mapped.contains("JSON") || mapped.contains("path"));
    }

    #[test]
    fn test_syntax_error_mapping() {
        let error = "Parse Error: syntax error";
        let mapped = map_error(error, "author ==");
        assert!(mapped.contains("syntax error"));
    }

    #[test]
    fn test_type_mismatch_error() {
        let error = "Binder Error: type mismatch";
        let mapped = map_error(error, "year > 2024");
        assert!(mapped.contains("type mismatch"));
    }

    #[test]
    fn test_unknown_error_passthrough() {
        let error = "Some unknown error";
        let mapped = map_error(error, "test");
        assert_eq!(mapped, error);
    }

    #[test]
    fn test_conversion_error_with_column() {
        let error = "Conversion Error: Could not convert string 'test' for column year";
        let mapped = map_error(error, "year == 'test'");
        assert!(mapped.contains("test"));
        assert!(mapped.contains("cast"));
    }
}
