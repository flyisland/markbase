use super::detector::{QueryMode, is_reserved_field};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TranslationContext {
    Normal,
    ListContainsFirstArg,
}

struct TranslationState {
    context: TranslationContext,
    paren_depth: usize,
}

impl TranslationState {
    fn new() -> Self {
        Self {
            context: TranslationContext::Normal,
            paren_depth: 0,
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

const DEFAULT_FIELDS: &str = "path, name, mtime, size, tags";

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

            let full = format!(
                "SELECT {} FROM notes {} {}",
                DEFAULT_FIELDS, where_part, suffix_part
            );
            normalize_sql(&full)
        }
        QueryMode::Empty => format!("SELECT {} FROM notes", DEFAULT_FIELDS),
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

            let translated = translate_identifier(&word, next_char, &mut state);
            result.push_str(&translated);
            continue;
        }

        if c == '(' {
            state.paren_depth += 1;
        } else if c == ')' {
            if state.paren_depth > 0 {
                state.paren_depth -= 1;
            }
        } else if c == ',' && state.context == TranslationContext::ListContainsFirstArg {
            state.context = TranslationContext::Normal;
        }

        result.push(c);
        i += 1;
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

    if state.context == TranslationContext::ListContainsFirstArg {
        state.context = TranslationContext::Normal;

        if word.contains('.') {
            let parts: Vec<&str> = word.split('.').collect();
            let first = parts[0];
            if is_reserved_field(first) {
                return word.to_string();
            }
            let json_path = parts
                .iter()
                .map(|p| format!("\"{}\"", p))
                .collect::<Vec<_>>()
                .join(".");
            return format!("(properties->'$.{}')::VARCHAR[]", json_path);
        }

        if is_reserved_field(word) {
            return word.to_string();
        }

        return format!("(properties->'$.\"{}\"')::VARCHAR[]", word);
    }

    if word.contains('.') {
        let parts: Vec<&str> = word.split('.').collect();
        let first = parts[0];
        if is_reserved_field(first) || first.to_lowercase() == "notes" {
            return word.to_string();
        }

        let json_path = parts
            .iter()
            .map(|p| format!("\"{}\"", p))
            .collect::<Vec<_>>()
            .join(".");

        return format!("json_extract_string(properties, '$.{}')", json_path);
    }

    if is_reserved_field(word) {
        return word.to_string();
    }

    format!("json_extract_string(properties, '$.\"{}\"')", word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_reserved_field() {
        assert_eq!(
            translate("SELECT path FROM notes"),
            "SELECT path FROM notes"
        );
        assert_eq!(
            translate("SELECT name, mtime FROM notes"),
            "SELECT name, mtime FROM notes"
        );
    }

    #[test]
    fn test_translate_frontmatter_field() {
        let result = translate("SELECT author FROM notes");
        assert!(result.contains("json_extract_string"));
        assert!(result.contains("author"));
    }

    #[test]
    fn test_translate_where_clause() {
        let result = translate("SELECT * FROM notes WHERE author == 'Tom'");
        assert!(result.contains("json_extract_string"));
        assert!(result.contains("'Tom'"));
    }

    #[test]
    fn test_translate_nested_field() {
        let result = translate("SELECT _schema.strict FROM notes");
        assert!(result.contains("json_extract_string"));
        assert!(result.contains("_schema"));
        assert!(result.contains("strict"));
    }

    #[test]
    fn test_translate_order_by() {
        let result = translate("ORDER BY mtime DESC");
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
        let result = translate("list_contains(categories, 'work')");
        assert_eq!(
            result,
            "list_contains((properties->'$.\"categories\"')::VARCHAR[], 'work')"
        );
    }

    #[test]
    fn test_translate_list_contains_reserved_field() {
        let result = translate("list_contains(tags, 'todo')");
        assert_eq!(result, "list_contains(tags, 'todo')");
    }

    #[test]
    fn test_translate_list_contains_in_where() {
        let result = translate("SELECT * FROM notes WHERE list_contains(categories, 'work')");
        assert!(
            result.contains("list_contains((properties->'$.\"categories\"')::VARCHAR[], 'work')")
        );
    }

    #[test]
    fn test_translate_list_contains_nested_field() {
        let result = translate("list_contains(meta.categories, 'work')");
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
        assert_eq!(translate("SELECT * FROM notes"), "SELECT * FROM notes");
        assert_eq!(
            translate("WHERE author = 'x' AND tags = 'y'"),
            "WHERE json_extract_string(properties, '$.\"author\"') = 'x' AND tags = 'y'"
        );
    }

    #[test]
    fn test_build_select_sql_empty() {
        let mode = QueryMode::Empty;
        let sql = build_select_sql(&mode);
        assert!(sql.contains("SELECT path, name, mtime, size, tags FROM notes"));
    }

    #[test]
    fn test_build_select_sql_expression() {
        let mode = QueryMode::Expression {
            where_clause: Some("author == 'Tom'".to_string()),
            suffix: None,
        };
        let sql = build_select_sql(&mode);
        assert!(sql.contains("SELECT path, name, mtime, size, tags FROM notes"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("author"));
    }

    #[test]
    fn test_build_select_sql_expression_with_suffix() {
        let mode = QueryMode::Expression {
            where_clause: Some("author == 'Tom'".to_string()),
            suffix: Some("ORDER BY mtime DESC LIMIT 10".to_string()),
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
            suffix: Some("ORDER BY mtime DESC".to_string()),
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
        assert!(sql.contains("FROM notes"));
    }

    #[test]
    fn test_translate_complex_query() {
        let sql = "SELECT path, author, mtime FROM notes WHERE author == 'Tom' AND year::INTEGER >= 2024 ORDER BY mtime DESC LIMIT 10";
        let result = translate(sql);
        assert!(result.contains("SELECT"));
        assert!(result.contains("FROM notes"));
        assert!(result.contains("ORDER BY"));
        assert!(result.contains("LIMIT"));
    }

    #[test]
    fn test_translate_ignores_comments() {
        let result = translate("-- comment\nSELECT author FROM notes");
        assert!(result.contains("SELECT"));
        assert!(result.contains("author"));
    }
}
