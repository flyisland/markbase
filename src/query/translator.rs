use super::detector::{QueryMode, is_file_property, note_field_key};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TranslationContext {
    Normal,
    ListContainsFirstArg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectContext {
    None,       // Not in SELECT clause
    Select,     // In SELECT clause, waiting for first field
    Field,      // In field expression
    AfterField, // After field, expecting comma or FROM
}

struct TranslationState {
    context: TranslationContext,
    paren_depth: usize,
    select_context: SelectContext,
}

impl TranslationState {
    fn new() -> Self {
        Self {
            context: TranslationContext::Normal,
            paren_depth: 0,
            select_context: SelectContext::None,
        }
    }
}

const SQL_KEYWORDS: &[&str] = &[
    "SELECT",
    "FROM",
    "WHERE",
    "AND",
    "OR",
    "NOT",
    "IN",
    "BETWEEN",
    "EXISTS",
    "LIKE",
    "GLOB",
    "ORDER",
    "BY",
    "LIMIT",
    "OFFSET",
    "GROUP",
    "HAVING",
    "JOIN",
    "LEFT",
    "RIGHT",
    "INNER",
    "OUTER",
    "FULL",
    "CROSS",
    "ON",
    "AS",
    "DISTINCT",
    "UNION",
    "INTERSECT",
    "EXCEPT",
    "CASE",
    "WHEN",
    "THEN",
    "ELSE",
    "END",
    "NULL",
    "IS",
    "ASC",
    "DESC",
    "TRUE",
    "FALSE",
    "CAST",
    "VARCHAR",
    "INTEGER",
    "BIGINT",
    "DOUBLE",
    "BOOLEAN",
    "TIMESTAMP",
    "DATE",
    "TIME",
    "ALL",
    "ANY",
    "BETWEEN",
    "OVERLAPS",
    "SIMILAR",
    "ESCAPE",
    "FOR",
    "KEY",
    "REFERENCES",
    "CONSTRAINT",
    "DEFAULT",
    "PRIMARY",
    "UNIQUE",
    "CHECK",
    "CREATE",
    "TABLE",
    "INDEX",
    "VIEW",
    "TRIGGER",
    "IF",
    "NOT",
    "REPLACE",
    "EXISTS",
    "TEMP",
    "TEMPORARY",
    "list_contains",
    "json_extract_string",
    "json_extract",
    "json_type",
    "json_array_length",
    "array_length",
    "array_agg",
    "string_agg",
    "coalesce",
    "nullif",
    "count",
    "sum",
    "avg",
    "min",
    "max",
    "length",
    "lower",
    "upper",
    "trim",
    "substr",
    "concat",
    "split_part",
    "regexp_matches",
    "regexp_replace",
];

const DEFAULT_FIELDS: &[(&str, &str)] = &[
    ("path", "file.path"),
    ("name", "file.name"),
    ("mtime", "file.mtime"),
    ("size", "file.size"),
    ("tags", "file.tags"),
];

fn build_default_select() -> String {
    let fields: Vec<String> = DEFAULT_FIELDS
        .iter()
        .map(|(col, alias)| format!(r#"{} AS "{}""#, col, alias))
        .collect();
    format!("SELECT {} FROM notes", fields.join(", "))
}

pub fn build_select_sql(mode: &QueryMode) -> String {
    match mode {
        QueryMode::Sql(sql) => translate(sql),
        QueryMode::Expression {
            where_clause,
            suffix,
        } => {
            let where_part = where_clause
                .as_ref()
                .map(|w| format!("WHERE {}", translate(w)))
                .unwrap_or_default();

            let suffix_part = suffix.as_ref().map(|s| translate(s)).unwrap_or_default();

            let base = build_default_select();
            let full = format!("{} {} {}", base, where_part, suffix_part);
            normalize_sql(&full)
        }
        QueryMode::Empty => build_default_select(),
    }
}

fn normalize_sql(sql: &str) -> String {
    sql.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn translate(sql: &str) -> String {
    let mut result = String::new();
    let mut in_string = false;
    let mut string_char = ' ';
    let mut i = 0;
    let chars: Vec<char> = sql.chars().collect();
    let mut state = TranslationState::new();
    let mut pending_field_alias: Option<String> = None;

    while i < chars.len() {
        let c = chars[i];

        if !in_string && (c == '\'' || c == '"') {
            in_string = true;
            string_char = c;
            result.push(c);
            i += 1;
            continue;
        }

        if in_string {
            if c == '\\' && i + 1 < chars.len() {
                result.push(c);
                result.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == string_char {
                in_string = false;
            }
            result.push(c);
            i += 1;
            continue;
        }

        if c == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }

        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() {
                let ch = chars[i];
                if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                    i += 1;
                } else {
                    break;
                }
            }
            let word: String = chars[start..i].iter().collect();
            let next_char = chars.get(i).copied();

            // Track SELECT keyword
            if word.to_uppercase() == "SELECT" && state.select_context == SelectContext::None {
                state.select_context = SelectContext::Select;
                result.push_str(&word);
                continue;
            }

            // Track FROM keyword - exits SELECT context
            if word.to_uppercase() == "FROM" {
                // Add pending alias before leaving SELECT context
                if let Some(alias) = pending_field_alias.take() {
                    result.push_str(&format!(r#" AS "{}""#, alias));
                }
                state.select_context = SelectContext::None;
                result.push_str(&word);
                continue;
            }

            // Check if this is a SQL keyword (for SELECT aliasing purposes)
            let is_sql_keyword = SQL_KEYWORDS.contains(&word.to_uppercase().as_str())
                || word.to_lowercase() == "notes";

            let translated = translate_identifier(&word, next_char, &mut state);

            // Handle SELECT field aliasing - only for actual field identifiers
            if !is_sql_keyword && state.select_context != SelectContext::None {
                match state.select_context {
                    SelectContext::Select => {
                        // First field in SELECT
                        if let Some(alias) = pending_field_alias.take() {
                            result.push_str(&format!(r#" AS "{}""#, alias));
                        }
                        pending_field_alias = Some(word.clone());
                        state.select_context = SelectContext::Field;
                    }
                    SelectContext::AfterField => {
                        // New field after comma
                        if let Some(alias) = pending_field_alias.take() {
                            result.push_str(&format!(r#" AS "{}""#, alias));
                        }
                        pending_field_alias = Some(word.clone());
                        state.select_context = SelectContext::Field;
                    }
                    SelectContext::Field if state.paren_depth == 0 => {
                        // Additional identifier in field expression
                        // Don't change the alias, the first one wins
                    }
                    _ => {}
                }
            }

            result.push_str(&translated);
            continue;
        }

        if c == '(' {
            state.paren_depth += 1;
        } else if c == ')' {
            if state.paren_depth > 0 {
                state.paren_depth -= 1;
            }
            if state.paren_depth == 0 && state.select_context == SelectContext::Field {
                state.select_context = SelectContext::AfterField;
            }
        } else if c == ',' {
            if state.context == TranslationContext::ListContainsFirstArg {
                state.context = TranslationContext::Normal;
            }
            // End of field in SELECT
            if state.select_context == SelectContext::Field
                || state.select_context == SelectContext::AfterField
            {
                if let Some(alias) = pending_field_alias.take() {
                    result.push_str(&format!(r#" AS "{}""#, alias));
                }
                state.select_context = SelectContext::Select;
            }
        } else if c.is_whitespace()
            && state.select_context == SelectContext::Field
            && state.paren_depth == 0
        {
            // Whitespace after field expression - mark as after field
            state.select_context = SelectContext::AfterField;
        }

        result.push(c);
        i += 1;
    }

    // Handle trailing field alias at end of SQL
    if let Some(alias) = pending_field_alias.take() {
        result.push_str(&format!(r#" AS "{}""#, alias));
    }

    result
}

fn translate_identifier(
    word: &str,
    next_char: Option<char>,
    state: &mut TranslationState,
) -> String {
    let upper = word.to_uppercase();
    let lower = word.to_lowercase();

    if lower == "list_contains" {
        if next_char == Some('(') {
            state.context = TranslationContext::ListContainsFirstArg;
        }
        return word.to_string();
    }

    if SQL_KEYWORDS.contains(&upper.as_str())
        || SQL_KEYWORDS.contains(&lower.as_str())
        || lower == "notes"
    {
        return word.to_string();
    }

    // NEW: Handle file. prefix - direct column access
    if let Some(field) = word.strip_prefix("file.") {
        // For list_contains first arg context, wrap appropriately
        if state.context == TranslationContext::ListContainsFirstArg {
            state.context = TranslationContext::Normal;
            return field.to_string();
        }
        return field.to_string();
    }

    // NEW: Handle note. prefix - always frontmatter JSON extraction
    if word.starts_with("note.") {
        let key = note_field_key(word);
        let json_path = if key.contains('.') {
            key.split('.')
                .map(|p| format!("\"{}\"", p))
                .collect::<Vec<_>>()
                .join(".")
        } else {
            format!("\"{}\"", key)
        };

        if state.context == TranslationContext::ListContainsFirstArg {
            state.context = TranslationContext::Normal;
            return format!("(properties->'$.{}')::VARCHAR[]", json_path);
        }

        return format!("json_extract_string(properties, '$.{}')", json_path);
    }

    // EXISTING: List contains first arg context
    if state.context == TranslationContext::ListContainsFirstArg {
        state.context = TranslationContext::Normal;

        if word.contains('.') {
            let parts: Vec<&str> = word.split('.').collect();
            let first = parts[0];
            // Bare reserved fields with dots are treated as file properties
            if is_file_property(&format!("file.{}", first)) {
                return word.to_string();
            }
            let json_path = parts
                .iter()
                .map(|p| format!("\"{}\"", p))
                .collect::<Vec<_>>()
                .join(".");
            return format!("(properties->'$.{}')::VARCHAR[]", json_path);
        }

        // Bare reserved fields in list_contains should now be treated as frontmatter
        // (since file.* prefix is required for file properties)
        return format!("(properties->'$.\"{}\"')::VARCHAR[]", word);
    }

    // EXISTING: Nested field path (bare, no prefix)
    if word.contains('.') {
        let parts: Vec<&str> = word.split('.').collect();
        let first = parts[0];
        // Check if it looks like a file property without prefix
        if is_file_property(&format!("file.{}", first)) {
            // This is a reserved field used without prefix - treat as frontmatter
            // User should use file.* prefix for file properties
        }

        let json_path = parts
            .iter()
            .map(|p| format!("\"{}\"", p))
            .collect::<Vec<_>>()
            .join(".");

        return format!("json_extract_string(properties, '$.{}')", json_path);
    }

    // Bare identifier - frontmatter (note.* shorthand)
    format!("json_extract_string(properties, '$.\"{}\"')", word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_file_property() {
        // file.* prefix → direct column access with alias
        let result = translate("SELECT file.path FROM notes");
        assert!(result.contains(r#"AS "file.path""#));
        assert!(result.contains("FROM notes"));

        let result = translate("SELECT file.name, file.mtime FROM notes");
        assert!(result.contains(r#"AS "file.name""#));
        assert!(result.contains(r#"AS "file.mtime""#));
        assert!(result.contains("FROM notes"));

        let result = translate("SELECT file.folder, file.ext, file.size FROM notes");
        assert!(result.contains(r#"AS "file.folder""#));
        assert!(result.contains(r#"AS "file.ext""#));
        assert!(result.contains(r#"AS "file.size""#));
        assert!(result.contains("FROM notes"));
    }

    #[test]
    fn test_translate_note_prefix() {
        // note.* prefix → json_extract_string with alias
        let result = translate("SELECT note.author FROM notes");
        assert!(result.contains(r#"AS "note.author""#));
        assert!(result.contains("json_extract_string"));
        // The AS alias contains note.author, but the raw json_extract_string doesn't
        assert!(!result.contains("json_extract_string(properties, '$.\"author\"') AS \"author\""));

        // note.* with nested path
        let result = translate("SELECT note._schema.strict FROM notes");
        assert!(result.contains(r#"AS "note._schema.strict""#));
        assert!(result.contains("json_extract_string"));
        assert!(result.contains("_schema"));
        assert!(result.contains("strict"));
    }

    #[test]
    fn test_translate_bare_field_is_frontmatter() {
        // Bare identifiers are frontmatter (shorthand for note.*)
        let result = translate("SELECT author FROM notes");
        assert!(result.contains(r#"AS "author""#));
        assert!(result.contains("json_extract_string"));
        assert!(result.contains("author"));
    }

    #[test]
    fn test_translate_frontmatter_field() {
        let result = translate("SELECT author FROM notes");
        assert!(result.contains(r#"AS "author""#));
        assert!(result.contains("json_extract_string"));
        assert!(result.contains("author"));
    }

    #[test]
    fn test_translate_where_clause() {
        // WHERE clause should NOT have aliases (only SELECT fields have aliases)
        let result = translate("SELECT * FROM notes WHERE author == 'Tom'");
        assert!(result.contains("json_extract_string"));
        assert!(result.contains("'Tom'"));
        // Since we use SELECT *, there should be no AS aliases at all
        // The AS keyword should only appear for explicit field selections in SELECT
        assert!(!result.contains(" AS "));
    }

    #[test]
    fn test_translate_nested_field() {
        let result = translate("SELECT _schema.strict FROM notes");
        assert!(result.contains(r#"AS "_schema.strict""#));
        assert!(result.contains("json_extract_string"));
        assert!(result.contains("_schema"));
        assert!(result.contains("strict"));
    }

    #[test]
    fn test_translate_order_by() {
        let result = translate("ORDER BY file.mtime DESC");
        assert!(result.contains("ORDER BY"));
        assert!(result.contains("mtime"));
    }

    #[test]
    fn test_translate_limit() {
        let result = translate("LIMIT 10");
        assert!(result.contains("LIMIT"));
        assert!(result.contains("10"));
    }

    #[test]
    fn test_translate_list_contains_frontmatter_field() {
        // Bare field in list_contains → frontmatter, no alias in function args
        let result = translate("list_contains(categories, 'work')");
        assert_eq!(
            result,
            "list_contains((properties->'$.\"categories\"')::VARCHAR[], 'work')"
        );
    }

    #[test]
    fn test_translate_list_contains_note_prefix() {
        // note.* prefix in list_contains → frontmatter, no alias in function args
        let result = translate("list_contains(note.categories, 'work')");
        assert_eq!(
            result,
            "list_contains((properties->'$.\"categories\"')::VARCHAR[], 'work')"
        );
    }

    #[test]
    fn test_translate_list_contains_file_property() {
        // file.* prefix in list_contains → direct column, no alias in function args
        let result = translate("list_contains(file.tags, 'todo')");
        assert_eq!(result, "list_contains(tags, 'todo')");
    }

    #[test]
    fn test_translate_list_contains_in_where() {
        let result = translate("SELECT * FROM notes WHERE list_contains(note.categories, 'work')");
        assert!(
            result.contains("list_contains((properties->'$.\"categories\"')::VARCHAR[], 'work')")
        );
    }

    #[test]
    fn test_translate_list_contains_nested_field() {
        // Nested bare field → frontmatter, no alias in function args
        let result = translate("list_contains(meta.categories, 'work')");
        assert_eq!(
            result,
            "list_contains((properties->'$.\"meta\".\"categories\"')::VARCHAR[], 'work')"
        );
    }

    #[test]
    fn test_translate_list_contains_note_nested() {
        // note.* prefix with nested path, no alias in function args
        let result = translate("list_contains(note.meta.categories, 'work')");
        assert_eq!(
            result,
            "list_contains((properties->'$.\"meta\".\"categories\"')::VARCHAR[], 'work')"
        );
    }

    #[test]
    fn test_translate_type_cast() {
        let result = translate("year::INTEGER >= 2024");
        assert!(result.contains("::INTEGER"));
        assert!(result.contains("2024"));
    }

    #[test]
    fn test_translate_is_null() {
        let result = translate("author IS NOT NULL");
        assert!(result.contains("IS NOT NULL"));
    }

    #[test]
    fn test_preserve_string_literals() {
        let result = translate("name == 'test'");
        assert!(result.contains("'test'"));
    }

    #[test]
    fn test_ignore_sql_keywords() {
        assert_eq!(translate("SELECT * FROM notes"), r#"SELECT * FROM notes"#);
        assert_eq!(
            translate("WHERE author = 'x' AND file.tags = 'y'"),
            r#"WHERE json_extract_string(properties, '$."author"') = 'x' AND tags = 'y'"#
        );
    }

    #[test]
    fn test_build_select_sql_empty() {
        let mode = QueryMode::Empty;
        let sql = build_select_sql(&mode);
        // Default fields should have file.* aliases
        assert!(sql.contains(r#"path AS "file.path""#));
        assert!(sql.contains(r#"name AS "file.name""#));
        assert!(sql.contains(r#"mtime AS "file.mtime""#));
        assert!(sql.contains(r#"size AS "file.size""#));
        assert!(sql.contains(r#"tags AS "file.tags""#));
        assert!(sql.contains("FROM notes"));
    }

    #[test]
    fn test_build_select_sql_expression() {
        let mode = QueryMode::Expression {
            where_clause: Some("author == 'Tom'".to_string()),
            suffix: None,
        };
        let sql = build_select_sql(&mode);
        // Default fields should have file.* aliases
        assert!(sql.contains(r#"path AS "file.path""#));
        assert!(sql.contains(r#"name AS "file.name""#));
        assert!(sql.contains("FROM notes"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("author"));
    }

    #[test]
    fn test_build_select_sql_expression_with_suffix() {
        let mode = QueryMode::Expression {
            where_clause: Some("author == 'Tom'".to_string()),
            suffix: Some("ORDER BY file.mtime DESC LIMIT 10".to_string()),
        };
        let sql = build_select_sql(&mode);
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("ORDER BY"));
        assert!(sql.contains("LIMIT"));
    }

    #[test]
    fn test_build_select_sql_suffix_only() {
        let mode = QueryMode::Expression {
            where_clause: None,
            suffix: Some("ORDER BY file.mtime DESC".to_string()),
        };
        let sql = build_select_sql(&mode);
        assert!(sql.contains("ORDER BY mtime DESC"));
        assert!(!sql.contains("WHERE"));
    }

    #[test]
    fn test_build_select_sql_sql_mode() {
        let mode =
            QueryMode::Sql("SELECT path, author FROM notes WHERE author = 'Tom'".to_string());
        let sql = build_select_sql(&mode);
        assert!(sql.contains("SELECT"));
        assert!(sql.contains(r#"AS "path""#));
        assert!(sql.contains(r#"AS "author""#));
        assert!(sql.contains("FROM notes"));
    }

    #[test]
    fn test_translate_complex_query() {
        let sql = "SELECT file.path, author, file.mtime FROM notes WHERE author == 'Tom' AND year::INTEGER >= 2024 ORDER BY file.mtime DESC LIMIT 10";
        let result = translate(sql);
        assert!(result.contains("SELECT"));
        assert!(result.contains("FROM notes"));
        assert!(result.contains("ORDER BY"));
        assert!(result.contains("LIMIT"));
        // Verify file.* fields have aliases
        assert!(result.contains(r#"AS "file.path""#));
        assert!(result.contains(r#"AS "file.mtime""#));
        // Verify bare field has alias
        assert!(result.contains(r#"AS "author""#));
        // Verify bare field is frontmatter
        assert!(result.contains("json_extract_string"));
    }

    #[test]
    fn test_translate_ignores_comments() {
        let result = translate("-- comment\nSELECT author FROM notes");
        assert!(result.contains("SELECT"));
        assert!(result.contains("author"));
    }
}
