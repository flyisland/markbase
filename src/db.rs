use duckdb::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub type QueryResult = (Vec<String>, Vec<Vec<String>>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub path: String,
    pub folder: String,
    pub name: String,
    pub ext: String,
    pub size: u64,
    pub ctime: i64,
    pub mtime: i64,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    pub backlinks: Vec<String>,
    pub embeds: Vec<String>,
    pub properties: serde_json::Value,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }

    pub fn open_existing(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(path)?;
        Ok(Database { conn })
    }

    fn init_schema(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS notes (
                path TEXT PRIMARY KEY,
                folder TEXT NOT NULL,
                name TEXT NOT NULL,
                ext TEXT NOT NULL,
                size INTEGER NOT NULL,
                ctime TIMESTAMPTZ NOT NULL,
                mtime TIMESTAMPTZ NOT NULL,
                tags VARCHAR[],
                links VARCHAR[],
                backlinks VARCHAR[],
                embeds VARCHAR[],
                properties JSON
            )",
            [],
        )?;

        self.conn
            .execute("CREATE INDEX IF NOT EXISTS idx_mtime ON notes(mtime)", [])?;
        self.conn
            .execute("CREATE INDEX IF NOT EXISTS idx_folder ON notes(folder)", [])?;
        self.conn
            .execute("CREATE INDEX IF NOT EXISTS idx_name ON notes(name)", [])?;

        Ok(())
    }

    pub fn upsert_note(&self, note: &Note) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "INSERT INTO notes 
             (path, folder, name, ext, size, ctime, mtime, tags, links, backlinks, embeds, properties)
             VALUES (?, ?, ?, ?, ?, to_timestamp(?), to_timestamp(?), ?, ?, ?, ?, ?)
             ON CONFLICT (path) DO UPDATE SET
                folder = excluded.folder,
                name = excluded.name,
                ext = excluded.ext,
                size = excluded.size,
                ctime = excluded.ctime,
                mtime = excluded.mtime,
                tags = excluded.tags,
                links = excluded.links,
                backlinks = excluded.backlinks,
                embeds = excluded.embeds,
                properties = excluded.properties",
            params![
                &note.path,
                &note.folder,
                &note.name,
                &note.ext,
                note.size as i64,
                note.ctime,
                note.mtime,
                serde_json::to_string(&note.tags)?,
                serde_json::to_string(&note.links)?,
                serde_json::to_string(&note.backlinks)?,
                serde_json::to_string(&note.embeds)?,
                serde_json::to_string(&note.properties)?,
            ],
        )?;
        Ok(())
    }

    pub fn get_all_mtime_and_size(
        &self,
    ) -> Result<std::collections::HashMap<String, (i64, u64)>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare("SELECT path, mtime, size FROM notes")?;
        let mut rows = stmt.query([])?;

        let mut map = std::collections::HashMap::new();
        while let Some(row) = rows.next()? {
            let path: String = row.get(0)?;
            let mtime: chrono::DateTime<chrono::Utc> = row.get(1)?;
            let size: i64 = row.get(2)?;
            map.insert(path, (mtime.timestamp(), size as u64));
        }
        Ok(map)
    }

    pub fn get_notes_by_name(&self, name: &str) -> Result<Vec<Note>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare("SELECT * FROM notes WHERE name = ?")?;
        let mut rows = stmt.query(params![name])?;

        let mut notes = Vec::new();
        while let Some(row) = rows.next()? {
            notes.push(self.row_to_note(row)?);
        }
        Ok(notes)
    }

    pub fn get_all_notes(&self) -> Result<Vec<Note>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare("SELECT * FROM notes")?;
        let mut rows = stmt.query([])?;

        let mut notes = Vec::new();
        while let Some(row) = rows.next()? {
            notes.push(self.row_to_note(row)?);
        }
        Ok(notes)
    }

    pub fn delete_note(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.conn
            .execute("DELETE FROM notes WHERE path = ?", params![path])?;
        Ok(())
    }

    fn row_to_note(&self, row: &duckdb::Row) -> Result<Note, Box<dyn std::error::Error>> {
        let path: String = row.get(0)?;
        let folder: String = row.get(1)?;
        let name: String = row.get(2)?;
        let ext: String = row.get(3)?;
        let size: i64 = row.get(4)?;
        let ctime: chrono::DateTime<chrono::Utc> = row.get(5)?;
        let mtime: chrono::DateTime<chrono::Utc> = row.get(6)?;

        let tags: Vec<String> = {
            let val: duckdb::types::Value = row.get(7)?;
            match val {
                duckdb::types::Value::List(list) => list
                    .iter()
                    .filter_map(|v| match v {
                        duckdb::types::Value::Text(t) => Some(t.clone()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            }
        };

        let links: Vec<String> = {
            let val: duckdb::types::Value = row.get(8)?;
            match val {
                duckdb::types::Value::List(list) => list
                    .iter()
                    .filter_map(|v| match v {
                        duckdb::types::Value::Text(t) => Some(t.clone()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            }
        };

        let backlinks: Vec<String> = {
            let val: duckdb::types::Value = row.get(9)?;
            match val {
                duckdb::types::Value::List(list) => list
                    .iter()
                    .filter_map(|v| match v {
                        duckdb::types::Value::Text(t) => Some(t.clone()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            }
        };

        let embeds: Vec<String> = {
            let val: duckdb::types::Value = row.get(10)?;
            match val {
                duckdb::types::Value::List(list) => list
                    .iter()
                    .filter_map(|v| match v {
                        duckdb::types::Value::Text(t) => Some(t.clone()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            }
        };

        let properties_json: String = row.get(11)?;
        let properties: serde_json::Value =
            serde_json::from_str(&properties_json).unwrap_or(serde_json::Value::Null);

        Ok(Note {
            path,
            folder,
            name,
            ext,
            size: size as u64,
            ctime: ctime.timestamp(),
            mtime: mtime.timestamp(),
            tags,
            links,
            backlinks,
            embeds,
            properties,
        })
    }

    pub fn count_notes(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn get_all_links(
        &self,
    ) -> Result<std::collections::HashMap<String, Vec<String>>, Box<dyn std::error::Error>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, to_json(links) FROM notes")?;
        let mut rows = stmt.query([])?;

        let mut link_map = std::collections::HashMap::new();
        while let Some(row) = rows.next()? {
            let path: String = row.get(0)?;
            let links_json: String = row.get(1)?;
            let links: Vec<String> = serde_json::from_str(&links_json).unwrap_or_default();
            link_map.insert(path, links);
        }

        Ok(link_map)
    }

    pub fn query(
        &self,
        sql: &str,
        _fields: &str,
        _limit: usize,
    ) -> Result<QueryResult, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        let con = self
            .conn
            .try_clone()
            .map_err(|e| format!("Clone error: {}", e))?;

        let mut stmt = con.prepare(sql)?;
        stmt.execute([])?;
        let column_count = stmt.column_count();
        let column_names: Vec<String> = (0..column_count)
            .map(|i| stmt.column_name(i).map_or("", |v| v).to_string())
            .collect();
        let mut rows = stmt.query([])?;

        while let Some(row) = rows.next()? {
            let mut result_row = Vec::new();
            for i in 0..column_count {
                let val: duckdb::types::Value = row.get(i)?;
                let s = match val {
                    duckdb::types::Value::Text(t) => t,
                    duckdb::types::Value::Int(i) => i.to_string(),
                    duckdb::types::Value::BigInt(n) => n.to_string(),
                    duckdb::types::Value::Double(d) => d.to_string(),
                    duckdb::types::Value::Float(f) => f.to_string(),
                    duckdb::types::Value::Boolean(b) => b.to_string(),
                    duckdb::types::Value::Timestamp(_, ts) => {
                        let dt = chrono::DateTime::from_timestamp_micros(ts);
                        if let Some(dt) = dt {
                            dt.format("%Y-%m-%d %H:%M:%S").to_string()
                        } else {
                            ts.to_string()
                        }
                    }
                    duckdb::types::Value::List(list) => {
                        let items: Vec<String> = list
                            .iter()
                            .map(|v| match v {
                                duckdb::types::Value::Text(t) => t.clone(),
                                _ => format!("{:?}", v),
                            })
                            .collect();
                        serde_json::to_string(&items).unwrap_or_default()
                    }
                    _ => String::new(),
                };
                result_row.push(s);
            }
            results.push(result_row);
        }

        Ok((column_names, results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn get_unique_id() -> u64 {
        TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    fn create_test_note(name: &str) -> Note {
        Note {
            path: format!("/test/{}.md", name),
            folder: "/test".to_string(),
            name: name.to_string(),
            ext: "md".to_string(),
            size: 1000,
            ctime: 1704067200,
            mtime: 1704067200,
            tags: vec!["test".to_string(), "example".to_string()],
            links: vec!["link1".to_string()],
            backlinks: vec![],
            embeds: vec!["embed1.png".to_string()],
            properties: serde_json::json!({
                "title": name,
                "category": "test"
            }),
        }
    }

    fn cleanup_db(db_path: &std::path::Path) {
        let _ = std::fs::remove_file(db_path);
        let _ = std::fs::remove_file(db_path.with_extension("duckdb.wal"));
    }

    #[test]
    fn test_database_initialization() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!(
            "test_mdb_{}_{}.duckdb",
            std::process::id(),
            get_unique_id()
        ));
        let result = Database::new(&db_path);
        assert!(result.is_ok());
        cleanup_db(&db_path);
    }

    #[test]
    fn test_get_all_mtime_and_size() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!(
            "test_mdb_{}_{}.duckdb",
            std::process::id(),
            get_unique_id()
        ));
        let db = Database::new(&db_path).unwrap();

        let note1 = create_test_note("note1");
        let note2 = create_test_note("note2");
        db.upsert_note(&note1).unwrap();
        db.upsert_note(&note2).unwrap();

        let records = db.get_all_mtime_and_size().unwrap();
        assert_eq!(records.len(), 2);
        assert!(records.contains_key(&note1.path));
        assert!(records.contains_key(&note2.path));

        let (mtime, size) = records.get(&note1.path).unwrap();
        assert_eq!(*mtime, note1.mtime);
        assert_eq!(*size, note1.size);

        cleanup_db(&db_path);
    }

    #[test]
    fn test_get_all_links() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!(
            "test_mdb_{}_{}.duckdb",
            std::process::id(),
            get_unique_id()
        ));
        let db = Database::new(&db_path).unwrap();

        let note1 = create_test_note("note1");
        let mut note2 = create_test_note("note2");
        note2.links = vec!["note1".to_string()];

        db.upsert_note(&note1).unwrap();
        db.upsert_note(&note2).unwrap();

        let link_map = db.get_all_links().unwrap();
        assert_eq!(link_map.len(), 2);
        assert!(link_map.contains_key(&note1.path));
        assert!(link_map.contains_key(&note2.path));
        assert_eq!(link_map[&note2.path], vec!["note1"]);

        cleanup_db(&db_path);
    }

    #[test]
    fn test_query_notes() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!(
            "test_mdb_{}_{}.duckdb",
            std::process::id(),
            get_unique_id()
        ));
        let db = Database::new(&db_path).unwrap();

        let note1 = create_test_note("note1");
        let mut note2 = create_test_note("note2");
        note2.name = "other".to_string();

        db.upsert_note(&note1).unwrap();
        db.upsert_note(&note2).unwrap();

        let (_column_names, results) = db.query("SELECT * FROM notes", "*", 10).unwrap();
        assert_eq!(results.len(), 2);

        cleanup_db(&db_path);
    }

    #[test]
    fn test_query_with_filter() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!(
            "test_mdb_{}_{}.duckdb",
            std::process::id(),
            get_unique_id()
        ));
        let db = Database::new(&db_path).unwrap();

        let note1 = create_test_note("special");
        let note2 = create_test_note("other");

        db.upsert_note(&note1).unwrap();
        db.upsert_note(&note2).unwrap();

        let (_column_names, results) = db
            .query("SELECT * FROM notes WHERE name = 'special'", "*", 10)
            .unwrap();
        assert_eq!(results.len(), 1);

        cleanup_db(&db_path);
    }

    #[test]
    fn test_query_limit() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!(
            "test_mdb_{}_{}.duckdb",
            std::process::id(),
            get_unique_id()
        ));
        let db = Database::new(&db_path).unwrap();

        for i in 0..10 {
            let note = create_test_note(&format!("note{}", i));
            db.upsert_note(&note).unwrap();
        }

        let (_column_names, results) = db.query("SELECT * FROM notes LIMIT 5", "*", 5).unwrap();
        assert_eq!(results.len(), 5);

        cleanup_db(&db_path);
    }

    #[test]
    fn test_query_has_tags_integration() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!(
            "test_mdb_{}_{}.duckdb",
            std::process::id(),
            get_unique_id()
        ));
        let db = Database::new(&db_path).unwrap();

        let mut note1 = create_test_note("note1");
        note1.tags = vec!["design".to_string(), "technical".to_string()];

        let mut note2 = create_test_note("note2");
        note2.tags = vec!["todo".to_string()];

        db.upsert_note(&note1).unwrap();
        db.upsert_note(&note2).unwrap();

        let (_column_names, results) = db
            .query("SELECT * FROM notes WHERE 'design' = ANY(tags)", "*", 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0][0].contains("note1"));

        cleanup_db(&db_path);
    }

    #[test]
    fn test_query_has_links_integration() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!(
            "test_mdb_{}_{}.duckdb",
            std::process::id(),
            get_unique_id()
        ));
        let db = Database::new(&db_path).unwrap();

        let mut note1 = create_test_note("note1");
        note1.links = vec!["architecture".to_string(), "readme".to_string()];

        let mut note2 = create_test_note("note2");
        note2.links = vec!["other".to_string()];

        db.upsert_note(&note1).unwrap();
        db.upsert_note(&note2).unwrap();

        let (_column_names, results) = db
            .query(
                "SELECT * FROM notes WHERE 'architecture' = ANY(links)",
                "*",
                10,
            )
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0][0].contains("note1"));

        cleanup_db(&db_path);
    }

    #[test]
    fn test_query_has_embeds_integration() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!(
            "test_mdb_{}_{}.duckdb",
            std::process::id(),
            get_unique_id()
        ));
        let db = Database::new(&db_path).unwrap();

        let mut note1 = create_test_note("note1");
        note1.embeds = vec!["diagram.png".to_string(), "chart.jpg".to_string()];

        let mut note2 = create_test_note("note2");
        note2.embeds = vec!["other.png".to_string()];

        db.upsert_note(&note1).unwrap();
        db.upsert_note(&note2).unwrap();

        let (_column_names, results) = db
            .query(
                "SELECT * FROM notes WHERE 'diagram.png' = ANY(embeds)",
                "*",
                10,
            )
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0][0].contains("note1"));

        cleanup_db(&db_path);
    }

    #[test]
    fn test_query_has_empty_array() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!(
            "test_mdb_{}_{}.duckdb",
            std::process::id(),
            get_unique_id()
        ));
        let db = Database::new(&db_path).unwrap();

        let mut note1 = create_test_note("note1");
        note1.tags = vec![];

        let mut note2 = create_test_note("note2");
        note2.tags = vec!["tag1".to_string()];

        db.upsert_note(&note1).unwrap();
        db.upsert_note(&note2).unwrap();

        let (_column_names, results) = db
            .query("SELECT * FROM notes WHERE 'tag1' = ANY(tags)", "*", 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0][0].contains("note2"));

        cleanup_db(&db_path);
    }

    #[test]
    fn test_get_notes_by_name() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!(
            "test_mdb_{}_{}.duckdb",
            std::process::id(),
            get_unique_id()
        ));
        let db = Database::new(&db_path).unwrap();

        let mut note = create_test_note("unique-name");
        note.tags = vec!["tag1".to_string(), "tag2".to_string()];
        note.links = vec!["link1".to_string(), "link2".to_string()];
        note.backlinks = vec!["backlink1.md".to_string()];
        note.embeds = vec!["image.png".to_string()];

        db.upsert_note(&note).unwrap();

        let notes = db.get_notes_by_name("unique-name").unwrap();
        assert_eq!(notes.len(), 1);

        let retrieved = &notes[0];
        assert_eq!(retrieved.name, "unique-name");
        assert_eq!(retrieved.tags, vec!["tag1", "tag2"]);
        assert_eq!(retrieved.links, vec!["link1", "link2"]);
        assert_eq!(retrieved.backlinks, vec!["backlink1.md"]);
        assert_eq!(retrieved.embeds, vec!["image.png"]);
        assert_eq!(retrieved.folder, "/test");

        cleanup_db(&db_path);
    }

    #[test]
    fn test_get_notes_by_name_not_found() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!(
            "test_mdb_{}_{}.duckdb",
            std::process::id(),
            get_unique_id()
        ));
        let db = Database::new(&db_path).unwrap();

        let notes = db.get_notes_by_name("nonexistent").unwrap();
        assert!(notes.is_empty());

        cleanup_db(&db_path);
    }

    #[test]
    fn test_get_notes_by_name_multiple() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!(
            "test_mdb_{}_{}.duckdb",
            std::process::id(),
            get_unique_id()
        ));
        let db = Database::new(&db_path).unwrap();

        let mut note1 = create_test_note("same-name");
        note1.path = "/folder1/same-name.md".to_string();
        note1.folder = "/folder1".to_string();
        note1.tags = vec!["tag1".to_string()];

        let mut note2 = create_test_note("same-name");
        note2.path = "/folder2/same-name.md".to_string();
        note2.folder = "/folder2".to_string();
        note2.tags = vec!["tag2".to_string()];

        db.upsert_note(&note1).unwrap();
        db.upsert_note(&note2).unwrap();

        let notes = db.get_notes_by_name("same-name").unwrap();
        assert_eq!(notes.len(), 2);

        let folders: std::collections::HashSet<_> =
            notes.iter().map(|n| n.folder.as_str()).collect();
        assert!(folders.contains("/folder1"));
        assert!(folders.contains("/folder2"));

        cleanup_db(&db_path);
    }

    #[test]
    fn test_row_to_note_empty_arrays() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!(
            "test_mdb_{}_{}.duckdb",
            std::process::id(),
            get_unique_id()
        ));
        let db = Database::new(&db_path).unwrap();

        let mut note = create_test_note("empty-arrays");
        note.tags = vec![];
        note.links = vec![];
        note.backlinks = vec![];
        note.embeds = vec![];

        db.upsert_note(&note).unwrap();

        let notes = db.get_notes_by_name("empty-arrays").unwrap();
        assert_eq!(notes.len(), 1);

        let retrieved = &notes[0];
        assert!(retrieved.tags.is_empty());
        assert!(retrieved.links.is_empty());
        assert!(retrieved.backlinks.is_empty());
        assert!(retrieved.embeds.is_empty());

        cleanup_db(&db_path);
    }
}
