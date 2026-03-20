use crate::db::Database;
use crate::name_validator::validate_resolve_input;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MatchSource {
    Name,
    Alias,
    NameContainsQuery,
    QueryContainsName,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolveMatch {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    pub description: Option<String>,
    pub matched_by: MatchSource,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolveStatus {
    Exact,
    Alias,
    NameContainsQuery,
    QueryContainsName,
    Multiple,
    Missing,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolveResult {
    pub query: String,
    pub status: ResolveStatus,
    pub matches: Vec<ResolveMatch>,
}

pub fn resolve_names(
    db: &Database,
    names: &[String],
) -> Result<Vec<ResolveResult>, Box<dyn std::error::Error>> {
    for name in names {
        validate_resolve_input(name)?;
    }

    names.iter().map(|name| resolve_name(db, name)).collect()
}

fn resolve_name(db: &Database, name: &str) -> Result<ResolveResult, Box<dyn std::error::Error>> {
    let escaped = name.replace('\'', "''");
    let name_matches = format!("lower(name) = lower('{escaped}')");
    let alias_matches = format!(
        "EXISTS (\
             SELECT 1 \
             FROM UNNEST((properties->'$.\"aliases\"')::VARCHAR[]) AS alias(alias_value) \
             WHERE lower(alias_value) = lower('{escaped}')\
         )"
    );
    let name_contains_query =
        format!("contains(lower(name), lower('{escaped}')) AND lower(name) != lower('{escaped}')");
    let query_contains_name =
        format!("contains(lower('{escaped}'), lower(name)) AND lower(name) != lower('{escaped}')");
    let sql = format!(
        "SELECT path, name, json_extract_string(properties, '$.\"type\"') AS type, \
         json_extract_string(properties, '$.\"description\"') AS description, \
         CASE \
             WHEN {name_matches} THEN 'name' \
             WHEN {alias_matches} THEN 'alias' \
             WHEN {name_contains_query} THEN 'name_contains_query' \
             WHEN {query_contains_name} THEN 'query_contains_name' \
         END AS matched_by \
         FROM notes \
         WHERE {name_matches} \
            OR {alias_matches} \
            OR ({name_contains_query}) \
            OR ({query_contains_name}) \
         ORDER BY CASE \
             WHEN {name_matches} THEN 0 \
             WHEN {alias_matches} THEN 1 \
             WHEN {name_contains_query} THEN 2 \
             ELSE 3 \
         END, \
         abs(length(name) - length('{escaped}')), \
         name, path",
        name_matches = name_matches,
        alias_matches = alias_matches,
        name_contains_query = name_contains_query,
        query_contains_name = query_contains_name,
    );

    let (field_names, rows) = db.query(&sql, "*", usize::MAX)?;
    debug_assert_eq!(field_names.len(), 5);

    let matches: Vec<ResolveMatch> = rows
        .into_iter()
        .map(|row| ResolveMatch {
            path: row[0].clone(),
            name: row[1].clone(),
            r#type: normalize_optional(&row[2]),
            description: normalize_optional(&row[3]),
            matched_by: match row[4].as_str() {
                "name" => MatchSource::Name,
                "alias" => MatchSource::Alias,
                "name_contains_query" => MatchSource::NameContainsQuery,
                "query_contains_name" => MatchSource::QueryContainsName,
                _ => unreachable!("unexpected match source"),
            },
        })
        .collect();

    let status = classify_status(name, &matches);

    Ok(ResolveResult {
        query: name.to_string(),
        status,
        matches,
    })
}

fn classify_status(_query: &str, matches: &[ResolveMatch]) -> ResolveStatus {
    match matches {
        [] => ResolveStatus::Missing,
        [single] => match single.matched_by {
            MatchSource::Name => ResolveStatus::Exact,
            MatchSource::Alias => ResolveStatus::Alias,
            MatchSource::NameContainsQuery => ResolveStatus::NameContainsQuery,
            MatchSource::QueryContainsName => ResolveStatus::QueryContainsName,
        },
        _ => ResolveStatus::Multiple,
    }
}

fn normalize_optional(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Database, Note};
    use serde_json::json;
    use tempfile::TempDir;

    fn make_db() -> (TempDir, Database) {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::new(&temp_dir.path().join("test.duckdb")).unwrap();
        (temp_dir, db)
    }

    fn note(
        name: &str,
        path: &str,
        aliases: &[&str],
        note_type: Option<&str>,
        description: Option<&str>,
    ) -> Note {
        let mut properties = json!({ "aliases": aliases });
        if let Some(note_type) = note_type {
            properties["type"] = json!(note_type);
        }
        if let Some(description) = description {
            properties["description"] = json!(description);
        }

        Note {
            path: path.to_string(),
            folder: String::new(),
            name: name.to_string(),
            ext: "md".to_string(),
            size: 0,
            ctime: 0,
            mtime: 0,
            tags: Vec::new(),
            links: Vec::new(),
            backlinks: Vec::new(),
            embeds: Vec::new(),
            properties,
        }
    }

    #[test]
    fn test_resolve_exact_match() {
        let (_dir, db) = make_db();
        db.upsert_note(&note(
            "acme",
            "companies/acme.md",
            &["ACME Corp"],
            Some("company"),
            Some("A customer company"),
        ))
        .unwrap();

        let results = resolve_names(&db, &["acme".to_string()]).unwrap();

        assert_eq!(results[0].status, ResolveStatus::Exact);
        assert_eq!(results[0].matches[0].matched_by, MatchSource::Name);
        assert_eq!(results[0].matches[0].r#type.as_deref(), Some("company"));
        assert_eq!(
            results[0].matches[0].description.as_deref(),
            Some("A customer company")
        );
    }

    #[test]
    fn test_resolve_exact_match_is_case_insensitive() {
        let (_dir, db) = make_db();
        db.upsert_note(&note(
            "acme",
            "companies/acme.md",
            &["ACME Corp"],
            Some("company"),
            Some("A customer company"),
        ))
        .unwrap();

        let results = resolve_names(&db, &["ACME".to_string()]).unwrap();

        assert_eq!(results[0].status, ResolveStatus::Exact);
        assert_eq!(results[0].matches[0].matched_by, MatchSource::Name);
        assert_eq!(results[0].matches[0].name, "acme");
    }

    #[test]
    fn test_resolve_alias_match() {
        let (_dir, db) = make_db();
        db.upsert_note(&note(
            "acme",
            "companies/acme.md",
            &["阿里"],
            Some("company"),
            Some("A customer company"),
        ))
        .unwrap();

        let results = resolve_names(&db, &["阿里".to_string()]).unwrap();

        assert_eq!(results[0].status, ResolveStatus::Alias);
        assert_eq!(results[0].matches[0].matched_by, MatchSource::Alias);
        assert_eq!(
            results[0].matches[0].description.as_deref(),
            Some("A customer company")
        );
    }

    #[test]
    fn test_resolve_alias_match_is_case_insensitive() {
        let (_dir, db) = make_db();
        db.upsert_note(&note(
            "acme",
            "companies/acme.md",
            &["ACME Corp"],
            Some("company"),
            Some("A customer company"),
        ))
        .unwrap();

        let results = resolve_names(&db, &["acme corp".to_string()]).unwrap();

        assert_eq!(results[0].status, ResolveStatus::Alias);
        assert_eq!(results[0].matches[0].matched_by, MatchSource::Alias);
        assert_eq!(results[0].matches[0].name, "acme");
    }

    #[test]
    fn test_resolve_multiple_matches() {
        let (_dir, db) = make_db();
        db.upsert_note(&note(
            "zhangwei-person",
            "people/zhangwei-person.md",
            &["张伟"],
            Some("person"),
            Some("Shanghai contact"),
        ))
        .unwrap();
        db.upsert_note(&note(
            "zhangwei-shanghai",
            "people/zhangwei-shanghai.md",
            &["张伟"],
            Some("person"),
            Some("Another person"),
        ))
        .unwrap();

        let results = resolve_names(&db, &["张伟".to_string()]).unwrap();

        assert_eq!(results[0].status, ResolveStatus::Multiple);
        assert_eq!(results[0].matches.len(), 2);
    }

    #[test]
    fn test_resolve_name_contains_query_match() {
        let (_dir, db) = make_db();
        db.upsert_note(&note(
            "绿联科技",
            "companies/ugreen.md",
            &[],
            Some("company"),
            Some("A company"),
        ))
        .unwrap();

        let results = resolve_names(&db, &["绿联".to_string()]).unwrap();

        assert_eq!(results[0].status, ResolveStatus::NameContainsQuery);
        assert_eq!(
            results[0].matches[0].matched_by,
            MatchSource::NameContainsQuery
        );
    }

    #[test]
    fn test_resolve_name_contains_query_match_is_case_insensitive() {
        let (_dir, db) = make_db();
        db.upsert_note(&note(
            "AcmePlatform",
            "companies/acme-platform.md",
            &[],
            Some("company"),
            Some("A company"),
        ))
        .unwrap();

        let results = resolve_names(&db, &["platform".to_string()]).unwrap();

        assert_eq!(results[0].status, ResolveStatus::NameContainsQuery);
        assert_eq!(
            results[0].matches[0].matched_by,
            MatchSource::NameContainsQuery
        );
        assert_eq!(results[0].matches[0].name, "AcmePlatform");
    }

    #[test]
    fn test_resolve_query_contains_name_match() {
        let (_dir, db) = make_db();
        db.upsert_note(&note(
            "绿联科技",
            "companies/ugreen.md",
            &[],
            Some("company"),
            Some("A company"),
        ))
        .unwrap();

        let results = resolve_names(&db, &["深圳绿联科技有限公司".to_string()]).unwrap();

        assert_eq!(results[0].status, ResolveStatus::QueryContainsName);
        assert_eq!(
            results[0].matches[0].matched_by,
            MatchSource::QueryContainsName
        );
    }

    #[test]
    fn test_resolve_query_contains_name_match_is_case_insensitive() {
        let (_dir, db) = make_db();
        db.upsert_note(&note(
            "Acme",
            "companies/acme.md",
            &[],
            Some("company"),
            Some("A company"),
        ))
        .unwrap();

        let results = resolve_names(&db, &["bestACMEcustomer".to_string()]).unwrap();

        assert_eq!(results[0].status, ResolveStatus::QueryContainsName);
        assert_eq!(
            results[0].matches[0].matched_by,
            MatchSource::QueryContainsName
        );
        assert_eq!(results[0].matches[0].name, "Acme");
    }

    #[test]
    fn test_resolve_alias_ranks_before_partial_name_matches() {
        let (_dir, db) = make_db();
        db.upsert_note(&note(
            "绿联科技",
            "companies/ugreen.md",
            &[],
            Some("company"),
            Some("Partial name match"),
        ))
        .unwrap();
        db.upsert_note(&note(
            "networking-brand",
            "companies/networking-brand.md",
            &["绿联"],
            Some("company"),
            Some("Alias match"),
        ))
        .unwrap();

        let results = resolve_names(&db, &["绿联".to_string()]).unwrap();

        assert_eq!(results[0].status, ResolveStatus::Multiple);
        assert_eq!(results[0].matches.len(), 2);
        assert_eq!(results[0].matches[0].name, "networking-brand");
        assert_eq!(results[0].matches[0].matched_by, MatchSource::Alias);
        assert_eq!(
            results[0].matches[1].matched_by,
            MatchSource::NameContainsQuery
        );
    }

    #[test]
    fn test_resolve_deduplicates_match_sources_by_priority() {
        let (_dir, db) = make_db();
        db.upsert_note(&note(
            "绿联科技",
            "companies/ugreen.md",
            &["绿联"],
            Some("company"),
            Some("Alias and partial candidate"),
        ))
        .unwrap();

        let results = resolve_names(&db, &["绿联".to_string()]).unwrap();

        assert_eq!(results[0].status, ResolveStatus::Alias);
        assert_eq!(results[0].matches.len(), 1);
        assert_eq!(results[0].matches[0].matched_by, MatchSource::Alias);
    }

    #[test]
    fn test_resolve_missing() {
        let (_dir, db) = make_db();

        let results = resolve_names(&db, &["missing".to_string()]).unwrap();

        assert_eq!(results[0].status, ResolveStatus::Missing);
        assert!(results[0].matches.is_empty());
    }

    #[test]
    fn test_resolve_missing_description_serializes_as_null() {
        let (_dir, db) = make_db();
        db.upsert_note(&note(
            "acme",
            "companies/acme.md",
            &["ACME Corp"],
            Some("company"),
            None,
        ))
        .unwrap();

        let results = resolve_names(&db, &["acme".to_string()]).unwrap();
        let value = serde_json::to_value(&results).unwrap();

        assert!(value[0]["matches"][0].get("description").is_some());
        assert!(value[0]["matches"][0]["description"].is_null());
    }
}
