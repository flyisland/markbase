#[derive(Debug)]
pub enum QueryMode {
    Sql(String),
    Expression {
        where_clause: Option<String>,
        suffix: Option<String>,
    },
    Empty,
}

pub fn detect_mode(input: Option<&str>) -> Result<QueryMode, String> {
    let input = match input {
        Some(s) => s.trim(),
        None => "",
    };

    if input.is_empty() {
        return Ok(QueryMode::Empty);
    }

    let upper = input.to_uppercase();
    if upper.starts_with("SELECT") {
        return Ok(QueryMode::Sql(input.to_string()));
    }

    let (where_clause, suffix) = split_expression(input)?;

    Ok(QueryMode::Expression {
        where_clause,
        suffix,
    })
}

fn split_expression(input: &str) -> Result<(Option<String>, Option<String>), String> {
    let mut paren_depth = 0;
    let mut in_string = false;
    let mut string_char = ' ';
    let mut clause_start: Option<usize> = None;
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if !in_string && (c == '\'' || c == '"') {
            in_string = true;
            string_char = c;
            i += 1;
            continue;
        }

        if in_string {
            if c == string_char && (i == 0 || chars[i - 1] != '\\') {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if c == '(' {
            paren_depth += 1;
            i += 1;
            continue;
        }
        if c == ')' {
            paren_depth -= 1;
            i += 1;
            continue;
        }

        if paren_depth > 0 {
            i += 1;
            continue;
        }

        if i + 7 <= chars.len() {
            let rest: String = chars[i..].iter().collect();
            let upper = rest.to_uppercase();
            if upper.starts_with("ORDER ") || upper.starts_with("LIMIT ") {
                if let Some(_start) = clause_start {
                    return Ok((
                        Some(input[..i].trim().to_string()),
                        Some(input[i..].trim().to_string()),
                    ));
                }
                return Ok((None, Some(input[i..].trim().to_string())));
            }
            if upper.starts_with("GROUP ") || upper.starts_with("HAVING ") {
                if let Some(_start) = clause_start {
                    return Ok((
                        Some(input[..i].trim().to_string()),
                        Some(input[i..].trim().to_string()),
                    ));
                }
                return Ok((None, Some(input[i..].trim().to_string())));
            }
        }

        if clause_start.is_none() && c.is_alphabetic() {
            clause_start = Some(i);
        }

        i += 1;
    }

    Ok((Some(input.trim().to_string()), None))
}

pub fn validate_safety(sql: &str) -> Result<(), String> {
    let upper = sql.trim().to_uppercase();

    if !upper.starts_with("SELECT") {
        let keyword = upper.split_whitespace().next().unwrap_or("");
        return Err(format!(
            "Error: query command only supports SELECT statements, {} is not allowed",
            keyword
        ));
    }

    if sql.contains(';') {
        let parts: Vec<&str> = sql.split(';').filter(|s| !s.trim().is_empty()).collect();
        if parts.len() > 1 {
            return Err("Error: multiple statements are not allowed".to_string());
        }
    }

    Ok(())
}

/// Kept for backward compatibility during transition.
/// This function is deprecated; use `is_file_property()` instead.
#[allow(dead_code)]
pub fn is_reserved_field(field: &str) -> bool {
    matches!(
        field,
        "path"
            | "folder"
            | "name"
            | "ext"
            | "size"
            | "ctime"
            | "mtime"
            | "tags"
            | "links"
            | "backlinks"
            | "embeds"
    )
}

/// Returns true for file property prefixes: file.path, file.folder, etc.
pub fn is_file_property(field: &str) -> bool {
    matches!(
        field,
        "file.path"
            | "file.folder"
            | "file.name"
            | "file.ext"
            | "file.size"
            | "file.ctime"
            | "file.mtime"
            | "file.tags"
            | "file.links"
            | "file.backlinks"
            | "file.embeds"
    )
}

/// Strips "note." prefix if present; bare identifiers returned unchanged.
/// Both "note.author" and "author" refer to the same frontmatter field.
pub fn note_field_key(field: &str) -> &str {
    field.strip_prefix("note.").unwrap_or(field)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let result = detect_mode(None);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), QueryMode::Empty));
    }

    #[test]
    fn test_empty_string_input() {
        let result = detect_mode(Some(""));
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), QueryMode::Empty));
    }

    #[test]
    fn test_sql_mode_detected() {
        let result = detect_mode(Some("SELECT path, name FROM notes"));
        assert!(result.is_ok());
        let mode = result.unwrap();
        assert!(matches!(mode, QueryMode::Sql(_)));
        if let QueryMode::Sql(sql) = mode {
            assert!(sql.starts_with("SELECT"));
        }
    }

    #[test]
    fn test_sql_mode_case_insensitive() {
        let result = detect_mode(Some("select path from notes"));
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), QueryMode::Sql(_)));
    }

    #[test]
    fn test_expression_only_where() {
        let result = detect_mode(Some("author == 'Tom'"));
        assert!(result.is_ok());
        let mode = result.unwrap();
        assert!(matches!(
            mode,
            QueryMode::Expression {
                where_clause: Some(_),
                suffix: None
            }
        ));
    }

    #[test]
    fn test_expression_with_where_and_suffix() {
        let result = detect_mode(Some("author == 'Tom' ORDER BY mtime DESC LIMIT 10"));
        assert!(result.is_ok());
        let mode = result.unwrap();
        assert!(matches!(
            mode,
            QueryMode::Expression {
                where_clause: Some(_),
                suffix: Some(_)
            }
        ));
    }

    #[test]
    fn test_expression_only_suffix() {
        let result = detect_mode(Some("ORDER BY mtime DESC LIMIT 10"));
        assert!(result.is_ok());
        let mode = result.unwrap();
        assert!(matches!(
            mode,
            QueryMode::Expression {
                where_clause: None,
                suffix: Some(_)
            }
        ));
    }

    #[test]
    fn test_expression_with_limit_only() {
        let result = detect_mode(Some("LIMIT 50"));
        assert!(result.is_ok());
        let mode = result.unwrap();
        assert!(matches!(
            mode,
            QueryMode::Expression {
                where_clause: None,
                suffix: Some(_)
            }
        ));
    }

    #[test]
    fn test_validate_safety_rejects_delete() {
        let result = validate_safety("DELETE FROM notes WHERE name == 'old'");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("DELETE"));
    }

    #[test]
    fn test_validate_safety_rejects_insert() {
        let result = validate_safety("INSERT INTO notes (path) VALUES ('test')");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("INSERT"));
    }

    #[test]
    fn test_validate_safety_rejects_update() {
        let result = validate_safety("UPDATE notes SET name = 'new'");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("UPDATE"));
    }

    #[test]
    fn test_validate_safety_rejects_drop() {
        let result = validate_safety("DROP TABLE notes");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("DROP"));
    }

    #[test]
    fn test_validate_safety_rejects_multiple_statements() {
        let result = validate_safety("SELECT * FROM notes; DELETE FROM notes");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("multiple"));
    }

    #[test]
    fn test_validate_safety_accepts_select() {
        let result = validate_safety("SELECT path, name FROM notes WHERE author == 'Tom'");
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_reserved_field() {
        let reserved = [
            "path",
            "folder",
            "name",
            "ext",
            "size",
            "ctime",
            "mtime",
            "tags",
            "links",
            "backlinks",
            "embeds",
        ];
        for field in reserved {
            assert!(
                is_reserved_field(field),
                "Expected {} to be reserved",
                field
            );
        }

        assert!(!is_reserved_field("author"));
        assert!(!is_reserved_field("category"));
    }

    #[test]
    fn test_ignore_keywords_in_strings() {
        let result = detect_mode(Some("name == 'ORDER BY'"));
        assert!(result.is_ok());
        let mode = result.unwrap();
        assert!(matches!(
            mode,
            QueryMode::Expression {
                where_clause: Some(_),
                suffix: None
            }
        ));
    }

    #[test]
    fn test_ignore_keywords_in_function_args() {
        let result = detect_mode(Some("list_contains(tags, 'order') ORDER BY mtime"));
        assert!(result.is_ok());
        let mode = result.unwrap();
        assert!(matches!(
            mode,
            QueryMode::Expression {
                where_clause: Some(_),
                suffix: Some(_)
            }
        ));
    }

    #[test]
    fn test_is_file_property() {
        // Valid file properties
        assert!(is_file_property("file.path"));
        assert!(is_file_property("file.folder"));
        assert!(is_file_property("file.name"));
        assert!(is_file_property("file.ext"));
        assert!(is_file_property("file.size"));
        assert!(is_file_property("file.ctime"));
        assert!(is_file_property("file.mtime"));
        assert!(is_file_property("file.tags"));
        assert!(is_file_property("file.links"));
        assert!(is_file_property("file.backlinks"));
        assert!(is_file_property("file.embeds"));

        // Not file properties
        assert!(!is_file_property("file.author")); // Not a file property
        assert!(!is_file_property("author")); // Bare field
        assert!(!is_file_property("note.author")); // Note prefix
        assert!(!is_file_property("name")); // Bare reserved field
        assert!(!is_file_property("path")); // Bare reserved field
    }

    #[test]
    fn test_note_field_key() {
        // Strips note. prefix
        assert_eq!(note_field_key("note.author"), "author");
        assert_eq!(note_field_key("note._schema.strict"), "_schema.strict");
        assert_eq!(note_field_key("note.tags"), "tags");

        // Bare identifiers unchanged
        assert_eq!(note_field_key("author"), "author");
        assert_eq!(note_field_key("_schema.strict"), "_schema.strict");
        assert_eq!(note_field_key("tags"), "tags");

        // Other prefixes unchanged
        assert_eq!(note_field_key("file.name"), "file.name");
    }
}
