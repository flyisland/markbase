use super::parser::AstNode;
use crate::constants::RESERVED_FIELDS;

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

fn is_reserved_field(field: &str) -> bool {
    RESERVED_FIELDS.contains(&field)
}

pub fn resolve_field(field: &str) -> String {
    if is_reserved_field(field) {
        return field.to_string();
    }

    if field.contains('.') {
        let json_path = "$".to_string()
            + &field
                .split('.')
                .map(|p| format!(".\"{}\"", p))
                .collect::<Vec<_>>()
                .join("");
        return format!("json_extract_string(properties, '{}')", json_path);
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
        AstNode::Field(name) => resolve_field(name),
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

                return if is_array_field(&field_name) {
                    format!("array_length({}) > 0", field_name)
                } else if is_reserved_field(&field_name) {
                    format!("{} IS NOT NULL AND {} != ''", field_name, field_name)
                } else {
                    format!(
                        "NOT (json_extract(properties, '$.{}') IS NULL OR (json_type(properties, '$.{}') = 'STRING' AND json_extract_string(properties, '$.{}') = '') OR (json_type(properties, '$.{}') = 'ARRAY' AND json_array_length(properties, '$.{}') = 0))",
                        field_name, field_name, field_name, field_name, field_name
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
    fn test_resolve_reserved_field() {
        assert_eq!(resolve_field("name"), "name");
        assert_eq!(resolve_field("path"), "path");
        assert_eq!(resolve_field("folder"), "folder");
        assert_eq!(resolve_field("ext"), "ext");
        assert_eq!(resolve_field("size"), "size");
        assert_eq!(resolve_field("ctime"), "ctime");
        assert_eq!(resolve_field("mtime"), "mtime");
        assert_eq!(resolve_field("content"), "content");
        assert_eq!(resolve_field("tags"), "tags");
        assert_eq!(resolve_field("links"), "links");
        assert_eq!(resolve_field("backlinks"), "backlinks");
        assert_eq!(resolve_field("embeds"), "embeds");
    }

    #[test]
    fn test_compile_equality() {
        let ast = super::super::parser::parse("name == 'readme'");
        let sql = compile(&ast);
        assert_eq!(sql, "name = 'readme'");
    }

    #[test]
    fn test_compile_inequality() {
        let ast = super::super::parser::parse("name != 'test'");
        let sql = compile(&ast);
        assert_eq!(sql, "name != 'test'");
    }

    #[test]
    fn test_compile_comparison_operators() {
        let cases = vec![
            ("size > 1000", "size > 1000"),
            ("size < 1000", "size < 1000"),
            ("size >= 1000", "size >= 1000"),
            ("size <= 1000", "size <= 1000"),
        ];
        for (query, expected) in cases {
            let ast = super::super::parser::parse(query);
            let sql = compile(&ast);
            assert_eq!(sql, expected, "Failed for query: {}", query);
        }
    }

    #[test]
    fn test_compile_pattern_match() {
        let ast = super::super::parser::parse("name =~ '%test%'");
        let sql = compile(&ast);
        assert_eq!(sql, "name LIKE '%test%'");
    }

    #[test]
    fn test_compile_and_operator() {
        let ast = super::super::parser::parse("name == 'a' and size > 100");
        let sql = compile(&ast);
        assert_eq!(sql, "name = 'a' AND size > 100");
    }

    #[test]
    fn test_compile_or_operator() {
        let ast = super::super::parser::parse("name == 'a' or name == 'b'");
        let sql = compile(&ast);
        assert_eq!(sql, "name = 'a' OR name = 'b'");
    }

    #[test]
    fn test_compile_grouping() {
        let ast = super::super::parser::parse("(name == 'a')");
        let sql = compile(&ast);
        assert_eq!(sql, "(name = 'a')");
    }

    #[test]
    fn test_compile_function_has() {
        let ast = super::super::parser::parse("has(tags, 'important')");
        let sql = compile(&ast);
        assert_eq!(sql, "'important' = ANY(tags)");
    }

    #[test]
    fn test_compile_complex_query() {
        let ast =
            super::super::parser::parse("name == 'readme' and size > 1000 or has(tags, 'todo')");
        let sql = compile(&ast);
        assert_eq!(sql, "name = 'readme' AND size > 1000 OR 'todo' = ANY(tags)");
    }

    #[test]
    fn test_compile_string_escaping() {
        let ast = super::super::parser::parse("name == 'it''s'");
        let sql = compile(&ast);
        assert_eq!(sql, "name = 'it'");
    }

    #[test]
    fn test_build_sql_with_star() {
        let result = build_sql("name == 'test'", "*");
        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("SELECT path, folder, name"));
        assert!(sql.contains("FROM documents"));
        assert!(sql.contains("name = 'test'"));
    }

    #[test]
    fn test_build_sql_with_custom_fields() {
        let result = build_sql("name == 'test'", "path,name");
        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("SELECT path, name"));
        assert!(sql.contains("FROM documents"));
    }

    #[test]
    fn test_build_sql_with_file_fields() {
        let result = build_sql("tags == 'test'", "path,tags");
        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("SELECT path, tags"));
    }

    #[test]
    fn test_build_sql_with_frontmatter_property() {
        let result = build_sql("category == 'test'", "path,category");
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
        let query = "name =~ '%test%'";
        let ast = super::super::parser::parse(query);
        let sql = compile(&ast);
        assert!(
            sql.contains("LIKE"),
            "=~ should use LIKE operator, got: {}",
            sql
        );
    }
}
