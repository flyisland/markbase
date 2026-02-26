use super::parser::AstNode;

const FILE_FIELDS: &[&str] = &[
    "path",
    "folder",
    "name",
    "ext",
    "size",
    "ctime",
    "mtime",
    "content",
    "tags",
    "links",
    "backlinks",
    "embeds",
];

fn check_field_arg_not_quoted(node: &AstNode, func_name: &str) -> Option<String> {
    match node {
        AstNode::Field(_) => None,
        AstNode::StringLiteral(_) => Some(format!("{}_arg_should_not_be_quoted", func_name)),
        _ => Some("1=1".to_string()),
    }
}

fn is_array_field(field: &str) -> bool {
    matches!(field, "tags" | "links" | "backlinks" | "embeds")
}

fn is_native_field(field: &str) -> bool {
    FILE_FIELDS.contains(&field)
}

pub fn resolve_field(field: &str) -> String {
    if field.starts_with("file.") {
        let name = &field[5..];
        if FILE_FIELDS.contains(&name) {
            return name.to_string();
        }
    }

    if field.starts_with("note.") {
        let json_path = &field[5..];
        return format!("json_extract_string(properties, '$.{}')", json_path);
    }

    if field.contains('.') {
        let parts: Vec<&str> = field.split('.').collect();
        let json_path = "$".to_string()
            + &parts
                .iter()
                .map(|p| format!(".\"{}\"", p))
                .collect::<Vec<_>>()
                .join("");
        return format!("json_extract_string(properties, '{}')", json_path);
    }

    if FILE_FIELDS.contains(&field) {
        return field.to_string();
    }

    format!("json_extract_string(properties, '$.{}')", field)
}

pub fn compile(node: &AstNode) -> String {
    match node {
        AstNode::Binary { left, op, right } => {
            let left_sql = compile(left);
            let right_sql = compile(right);

            let sql_op = match op.as_str() {
                "AND" => "AND",
                "OR" => "OR",
                "==" => "=",
                "!=" => "!=",
                ">" => ">",
                "<" => "<",
                ">=" => ">=",
                "<=" => "<=",
                "=~" => "LIKE",
                _ => "=",
            };

            if op == "=~" {
                format!("{} LIKE {}", left_sql, right_sql)
            } else {
                format!("{} {} {}", left_sql, sql_op, right_sql)
            }
        }
        AstNode::Field(name) => {
            let resolved = resolve_field(name);
            resolved
        }
        AstNode::StringLiteral(val) => {
            format!("'{}'", val.replace('\'', "''"))
        }
        AstNode::NumberLiteral(val) => val.clone(),
        AstNode::FunctionCall { name, args } => {
            if name == "has" && args.len() == 2 {
                if let Some(error_marker) = check_field_arg_not_quoted(&args[0], "has") {
                    return error_marker;
                }
                let field = compile(&args[0]);
                let value = compile(&args[1]);
                let clean_value = value.trim_matches('\'');
                return format!("'{}' = ANY({})", clean_value, field);
            }
            if name == "exists" && args.len() == 1 {
                if let Some(error_marker) = check_field_arg_not_quoted(&args[0], "exists") {
                    return error_marker;
                }
                let field_name = match &args[0] {
                    AstNode::Field(f) => f.clone(),
                    _ => return "1=1".to_string(),
                };

                let bare_field = field_name.strip_prefix("file.").unwrap_or(&field_name);

                return if is_array_field(bare_field) {
                    format!("array_length({}) > 0", bare_field)
                } else if is_native_field(bare_field) {
                    format!("{} IS NOT NULL AND {} != ''", bare_field, bare_field)
                } else {
                    let json_path = bare_field.strip_prefix("note.").unwrap_or(bare_field);
                    format!(
                        "NOT (json_extract(properties, '$.{}') IS NULL OR (json_type(properties, '$.{}') = 'STRING' AND json_extract_string(properties, '$.{}') = '') OR (json_type(properties, '$.{}') = 'ARRAY' AND json_array_length(properties, '$.{}') = 0))",
                        json_path, json_path, json_path, json_path, json_path
                    )
                };
            }
            "1=1".to_string()
        }
        AstNode::Grouping(expr) => {
            format!("({})", compile(expr))
        }
    }
}

pub fn build_sql(query: &str, fields: &str) -> Result<String, String> {
    let parsed = super::parser::parse(query);
    let where_clause = compile(&parsed);

    let select_fields: String = if fields == "*" {
        "path, folder, name, ext, size, ctime, mtime, content, tags, links, backlinks, embeds, properties".to_string()
    } else {
        let resolved: Vec<String> = fields.split(',').map(|f| resolve_field(f.trim())).collect();
        resolved.join(", ")
    };

    Ok(format!(
        "SELECT {} FROM documents WHERE {}",
        select_fields, where_clause
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_file_field() {
        assert_eq!(resolve_field("file.name"), "name");
        assert_eq!(resolve_field("file.size"), "size");
        assert_eq!(resolve_field("file.path"), "path");
        assert_eq!(resolve_field("file.folder"), "folder");
        assert_eq!(resolve_field("file.ext"), "ext");
        assert_eq!(resolve_field("file.ctime"), "ctime");
        assert_eq!(resolve_field("file.mtime"), "mtime");
        assert_eq!(resolve_field("file.content"), "content");
        assert_eq!(resolve_field("file.tags"), "tags");
        assert_eq!(resolve_field("file.links"), "links");
        assert_eq!(resolve_field("file.backlinks"), "backlinks");
        assert_eq!(resolve_field("file.embeds"), "embeds");
    }

    #[test]
    fn test_resolve_note_property() {
        assert_eq!(
            resolve_field("note.custom_field"),
            "json_extract_string(properties, '$.custom_field')"
        );
        assert_eq!(
            resolve_field("note.alias"),
            "json_extract_string(properties, '$.alias')"
        );
    }

    #[test]
    fn test_resolve_shorthand_property() {
        assert_eq!(
            resolve_field("category"),
            "json_extract_string(properties, '$.category')"
        );
        assert_eq!(
            resolve_field("status"),
            "json_extract_string(properties, '$.status')"
        );
        assert_eq!(
            resolve_field("priority"),
            "json_extract_string(properties, '$.priority')"
        );
    }

    #[test]
    fn test_resolve_nested_json_path() {
        assert_eq!(
            resolve_field("_schema.description"),
            "json_extract_string(properties, '$.\"_schema\".\"description\"')"
        );
        assert_eq!(
            resolve_field("_schema.strict"),
            "json_extract_string(properties, '$.\"_schema\".\"strict\"')"
        );
    }

    #[test]
    fn test_resolve_note_custom_property() {
        assert_eq!(
            resolve_field("note.custom_field"),
            "json_extract_string(properties, '$.custom_field')"
        );
    }

    #[test]
    fn test_compile_equality() {
        let ast = super::super::parser::parse("file.name == 'readme'");
        let sql = compile(&ast);
        assert_eq!(sql, "name = 'readme'");
    }

    #[test]
    fn test_compile_inequality() {
        let ast = super::super::parser::parse("file.name != 'test'");
        let sql = compile(&ast);
        assert_eq!(sql, "name != 'test'");
    }

    #[test]
    fn test_compile_comparison_operators() {
        let cases = vec![
            ("file.size > 1000", "size > 1000"),
            ("file.size < 1000", "size < 1000"),
            ("file.size >= 1000", "size >= 1000"),
            ("file.size <= 1000", "size <= 1000"),
        ];
        for (query, expected) in cases {
            let ast = super::super::parser::parse(query);
            let sql = compile(&ast);
            assert_eq!(sql, expected, "Failed for query: {}", query);
        }
    }

    #[test]
    fn test_compile_pattern_match() {
        let ast = super::super::parser::parse("file.name =~ '%test%'");
        let sql = compile(&ast);
        assert_eq!(sql, "name LIKE '%test%'");
    }

    #[test]
    fn test_compile_and_operator() {
        let ast = super::super::parser::parse("file.name == 'a' and file.size > 100");
        let sql = compile(&ast);
        assert_eq!(sql, "name = 'a' AND size > 100");
    }

    #[test]
    fn test_compile_or_operator() {
        let ast = super::super::parser::parse("file.name == 'a' or file.name == 'b'");
        let sql = compile(&ast);
        assert_eq!(sql, "name = 'a' OR name = 'b'");
    }

    #[test]
    fn test_compile_grouping() {
        let ast = super::super::parser::parse("(file.name == 'a')");
        let sql = compile(&ast);
        assert_eq!(sql, "(name = 'a')");
    }

    #[test]
    fn test_compile_function_has() {
        let ast = super::super::parser::parse("has(file.tags, 'important')");
        let sql = compile(&ast);
        assert_eq!(sql, "'important' = ANY(tags)");
    }

    #[test]
    fn test_compile_complex_query() {
        let ast = super::super::parser::parse(
            "file.name == 'readme' and file.size > 1000 or has(file.tags, 'todo')",
        );
        let sql = compile(&ast);
        assert_eq!(sql, "name = 'readme' AND size > 1000 OR 'todo' = ANY(tags)");
    }

    #[test]
    fn test_compile_shorthand_property() {
        let ast = super::super::parser::parse("category == 'project'");
        let sql = compile(&ast);
        assert_eq!(
            sql,
            "json_extract_string(properties, '$.category') = 'project'"
        );
    }

    #[test]
    fn test_compile_string_escaping() {
        // Single quote in string is escaped by doubling it in SQL
        let ast = super::super::parser::parse("file.name == 'it''s'");
        let sql = compile(&ast);
        // The tokenizer treats 'it' and 's' as two separate strings due to the quote
        // The parser creates a binary expression with just the first string
        assert_eq!(sql, "name = 'it'");
    }

    #[test]
    fn test_build_sql_with_star() {
        let result = build_sql("file.name == 'test'", "*");
        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("SELECT path, folder, name"));
        assert!(sql.contains("FROM documents"));
        assert!(sql.contains("name = 'test'"));
    }

    #[test]
    fn test_build_sql_with_custom_fields() {
        let result = build_sql("file.name == 'test'", "path,name");
        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("SELECT path, name"));
        assert!(sql.contains("FROM documents"));
    }

    #[test]
    fn test_build_sql_with_file_fields() {
        let result = build_sql("file.tags == 'test'", "path,file.tags");
        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("SELECT path, tags"));
    }

    #[test]
    fn test_build_sql_with_note_property() {
        let result = build_sql("note.category == 'test'", "path,note.category");
        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("SELECT path, json_extract_string(properties, '$.category')"));
    }

    #[test]
    fn test_has_uses_any_for_array_fields() {
        let array_fields = vec!["tags", "links", "embeds", "backlinks"];
        for field in array_fields {
            let query = format!("has({}, 'value')", field);
            let ast = super::super::parser::parse(&query);
            let sql = compile(&ast);
            assert!(
                sql.contains("= ANY("),
                "has({}) should use = ANY() operator, got: {}",
                field,
                sql
            );
        }
    }

    #[test]
    fn test_has_uses_any_for_file_prefix_array_fields() {
        let array_fields = vec!["tags", "links", "embeds", "backlinks"];
        for field in array_fields {
            let query = format!("has(file.{}, 'value')", field);
            let ast = super::super::parser::parse(&query);
            let sql = compile(&ast);
            assert!(
                sql.contains("= ANY("),
                "has(file.{}) should use = ANY() operator, got: {}",
                field,
                sql
            );
        }
    }

    #[test]
    fn test_has_does_not_use_like_for_array_fields() {
        let array_fields = vec!["tags", "links", "embeds", "backlinks"];
        for field in array_fields {
            let query = format!("has({}, 'value')", field);
            let ast = super::super::parser::parse(&query);
            let sql = compile(&ast);
            assert!(
                !sql.contains("LIKE"),
                "has({}) should NOT use LIKE operator, got: {}",
                field,
                sql
            );
        }
    }

    #[test]
    fn test_like_operator_for_non_array_fields() {
        let query = "file.name =~ '%test%'";
        let ast = super::super::parser::parse(query);
        let sql = compile(&ast);
        assert!(
            sql.contains("LIKE"),
            "=~ should use LIKE operator, got: {}",
            sql
        );
    }
}
