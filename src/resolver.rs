use crate::db::Database;
use crate::name_validator::validate_resolve_input;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MatchSource {
    Name,
    Alias,
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
    let sql = format!(
        "SELECT path, name, json_extract_string(properties, '$.\"type\"') AS type, \
         json_extract_string(properties, '$.\"description\"') AS description, \
         CASE WHEN name = '{escaped}' THEN 'name' ELSE 'alias' END AS matched_by \
         FROM notes \
         WHERE name = '{escaped}' OR list_contains((properties->'$.\"aliases\"')::VARCHAR[], '{escaped}') \
         ORDER BY CASE WHEN name = '{escaped}' THEN 0 ELSE 1 END, name, path"
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
                _ => MatchSource::Alias,
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

fn classify_status(query: &str, matches: &[ResolveMatch]) -> ResolveStatus {
    match matches {
        [] => ResolveStatus::Missing,
        [single] => match single.matched_by {
            MatchSource::Name if single.name == query => ResolveStatus::Exact,
            MatchSource::Alias => ResolveStatus::Alias,
            MatchSource::Name => ResolveStatus::Exact,
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
