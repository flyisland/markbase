use crate::db::{Database, QueryResult};
use crate::query::detector::{QueryMode, detect_mode, validate_safety};
use crate::query::error_map::map_error;
use crate::query::translator::build_select_sql;

const DEFAULT_LIMIT: usize = 1000;

fn finalize_sql(sql: &str, limit: usize) -> String {
    if sql.to_uppercase().contains("LIMIT") {
        sql.to_string()
    } else {
        format!("{} LIMIT {}", sql, limit)
    }
}

pub fn execute_query(db: &Database, sql_input: Option<&str>) -> Result<QueryResult, String> {
    let mode = detect_mode(sql_input)?;
    let full_sql = build_select_sql(&mode);

    if let QueryMode::Sql(sql) = &mode {
        validate_safety(sql)?;
    }

    let final_sql = finalize_sql(&full_sql, DEFAULT_LIMIT);

    db.query(&final_sql, "", DEFAULT_LIMIT)
        .map_err(|e| map_error(&e.to_string(), sql_input.unwrap_or("")))
}

pub fn translate_query(sql_input: Option<&str>) -> Result<String, String> {
    let mode = detect_mode(sql_input)?;
    let full_sql = build_select_sql(&mode);

    if let QueryMode::Sql(sql) = &mode {
        validate_safety(sql)?;
    }

    let final_sql = finalize_sql(&full_sql, DEFAULT_LIMIT);

    Ok(final_sql)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_empty_input() {
        let result = translate_query(None);
        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("FROM notes"));
        assert!(sql.contains("LIMIT 1000"));
    }

    #[test]
    fn test_translate_expression() {
        let result = translate_query(Some("author == 'Tom'"));
        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("LIMIT 1000"));
    }

    #[test]
    fn test_translate_expression_with_suffix() {
        let result = translate_query(Some("author == 'Tom' ORDER BY mtime DESC LIMIT 10"));
        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("ORDER BY"));
        assert!(sql.contains("LIMIT 10"));
        assert!(!sql.contains("LIMIT 1000"));
    }

    #[test]
    fn test_translate_sql_mode() {
        let result = translate_query(Some("SELECT path, author FROM notes WHERE author = 'Tom'"));
        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("FROM notes"));
        assert!(sql.contains("LIMIT 1000"));
    }

    #[test]
    fn test_validate_safety_in_sql_mode() {
        let result = translate_query(Some("SELECT * FROM notes; DELETE FROM notes"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("multiple"));
    }
}
