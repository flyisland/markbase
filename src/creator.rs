use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use chrono::{DateTime, Local};
use regex::Regex;

use crate::name_validator::validate_note_name;
use crate::template::{TemplateDocument, default_note_content};

static RE_NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{\s*name\s*\}\}").unwrap());
static RE_DATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{\s*date\s*\}\}").unwrap());
static RE_TIME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{\s*time\s*\}\}").unwrap());
static RE_DATETIME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{\s*datetime\s*\}\}").unwrap());
static RE_TIMESTAMP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{\s*timestamp\s*\}\}").unwrap());

#[derive(Debug)]
pub struct CreatedNote {
    pub path: PathBuf,
}

pub fn create_note(
    base_dir: &Path,
    name: &str,
    template_name: Option<&str>,
) -> Result<CreatedNote, Box<dyn std::error::Error>> {
    validate_note_name(name)?;
    create_note_at(base_dir, name, template_name, Local::now())
}

fn create_note_at(
    base_dir: &Path,
    name: &str,
    template_name: Option<&str>,
    now: DateTime<Local>,
) -> Result<CreatedNote, Box<dyn std::error::Error>> {
    let timestamp = CreationTimestamp::from(now);

    let (content, location, filename_pattern) = match template_name {
        Some(tmpl) => process_template_document_at(
            &TemplateDocument::load(base_dir, tmpl)?,
            name,
            &timestamp,
        )?,
        None => (default_note_content()?, None, None),
    };
    let has_filename_pattern = filename_pattern.is_some();
    let requested_note_name = filename_pattern.unwrap_or_else(|| name.to_string());
    validate_note_name(&requested_note_name)?;
    let note_name = if has_filename_pattern {
        find_available_note_name(base_dir, &requested_note_name)?
    } else {
        requested_note_name
    };

    let target_path = if let Some(loc) = location {
        let dir = base_dir.join(&loc);
        let file_name = format!("{}.md", note_name);
        dir.join(&file_name)
    } else {
        base_dir.join("inbox").join(format!("{}.md", note_name))
    };

    if let Some(existing_path) = find_existing_note_path(base_dir, &note_name)? {
        return Err(format!(
            "Note '{}' already exists at '{}'",
            note_name,
            existing_path.display()
        )
        .into());
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&target_path, content)?;

    Ok(CreatedNote { path: target_path })
}

fn find_available_note_name(
    base_dir: &Path,
    requested_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    if find_existing_note_path(base_dir, requested_name)?.is_none() {
        return Ok(requested_name.to_string());
    }

    for suffix in 2.. {
        let candidate = format!("{}_{:02}", requested_name, suffix);
        if find_existing_note_path(base_dir, &candidate)?.is_none() {
            return Ok(candidate);
        }
    }

    unreachable!("the suffix search is unbounded")
}

fn find_existing_note_path(
    base_dir: &Path,
    name: &str,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    for entry in walkdir::WalkDir::new(base_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");

        if file_stem == name || file_name == name {
            return Ok(Some(path.to_path_buf()));
        }
    }

    Ok(None)
}

fn replace_template_variables(content: &str, name: &str, now: &CreationTimestamp) -> String {
    let result = RE_NAME.replace_all(content, name);
    let result = RE_DATE.replace_all(&result, &now.date);
    let result = RE_TIME.replace_all(&result, &now.time);
    let result = RE_DATETIME.replace_all(&result, &now.datetime);
    RE_TIMESTAMP
        .replace_all(&result, &now.timestamp)
        .to_string()
}

fn process_template_document_at(
    template: &TemplateDocument,
    name: &str,
    now: &CreationTimestamp,
) -> Result<(String, Option<String>, Option<String>), Box<dyn std::error::Error>> {
    let content = template.render_for_create()?;
    Ok((
        replace_template_variables(&content, name, now),
        template.location().map(ToString::to_string),
        template
            .filename_pattern()
            .map(|pattern| replace_template_variables(pattern, name, now)),
    ))
}

#[cfg(test)]
fn process_template(
    content: &str,
    name: &str,
) -> Result<(String, Option<String>), Box<dyn std::error::Error>> {
    let (content, location, _) = process_template_document_at(
        &TemplateDocument::from_content(content),
        name,
        &CreationTimestamp::from(Local::now()),
    )?;
    Ok((content, location))
}

struct CreationTimestamp {
    date: String,
    time: String,
    datetime: String,
    timestamp: String,
}

impl CreationTimestamp {
    fn from(now: DateTime<Local>) -> Self {
        Self {
            date: now.format("%Y-%m-%d").to_string(),
            time: now.format("%H:%M:%S").to_string(),
            datetime: now.to_rfc3339(),
            timestamp: now.format("%Y-%m-%d-%H-%M-%S").to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_template_variables() {
        let content = "# {{name}}\n\nDate: {{date}}\nTime: {{time}}\n";
        let result =
            replace_template_variables(content, "my-note", &CreationTimestamp::from(Local::now()));
        assert!(result.contains("# my-note"));
        assert!(result.contains("Date: "));
        assert!(result.contains("Time: "));
    }

    #[test]
    fn test_replace_partial_variables() {
        let content = "Title: {{name}}\nOnly date: {{date}}";
        let result =
            replace_template_variables(content, "test", &CreationTimestamp::from(Local::now()));
        assert!(result.contains("Title: test"));
        assert!(result.contains("Only date: "));
        assert!(!result.contains("{{time}}"));
    }

    #[test]
    fn test_replace_variables_with_spaces() {
        let content = "# {{ name }}\nDate: {{ date }}\nTime: {{ time }}";
        let result =
            replace_template_variables(content, "my-note", &CreationTimestamp::from(Local::now()));
        assert!(result.contains("# my-note"));
        assert!(result.contains("Date: "));
        assert!(result.contains("Time: "));
        assert!(!result.contains("{{"));
    }

    #[test]
    fn test_replace_variables_mixed_format() {
        let content = "{{name}} and {{ name }} and {{date}} and {{ date }}";
        let result =
            replace_template_variables(content, "test", &CreationTimestamp::from(Local::now()));
        assert!(result.contains("test and test and "));
    }

    #[test]
    fn test_replace_variables_multiple_spaces() {
        let content = "{{  name  }} {{date}}";
        let result =
            replace_template_variables(content, "x", &CreationTimestamp::from(Local::now()));
        let re = Regex::new(r"\d{4}-\d{2}-\d{2}").unwrap();
        assert!(
            re.is_match(&result),
            "date should match YYYY-MM-DD format, got: {}",
            result
        );
        assert!(!result.contains("{{"));
    }

    #[test]
    fn test_create_note_without_template() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("mdb_test_create");
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        let result = create_note(&test_dir, "test-note", None);
        assert!(result.is_ok());
        let created = result.unwrap();
        assert!(created.path.exists());
        assert_eq!(created.path.parent().unwrap(), test_dir.join("inbox"));
        assert_eq!(
            created.path.file_name().unwrap().to_str().unwrap(),
            "test-note.md"
        );
        let content = fs::read_to_string(&created.path).unwrap();
        assert!(content.contains("description: 临时笔记"));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_create_note_duplicate() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("mdb_test_dup");
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        fs::write(test_dir.join("existing.md"), "").unwrap();

        let result = create_note(&test_dir, "existing", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_create_note_with_template() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("mdb_test_tmpl");
        let _ = fs::remove_dir_all(&test_dir);

        let tmpl_dir = test_dir.join("templates");
        fs::create_dir_all(&tmpl_dir).unwrap();
        fs::write(tmpl_dir.join("daily.md"), "# {{name}}\nDate: {{date}}").unwrap();

        let result = create_note(&test_dir, "today", Some("daily"));
        assert!(result.is_ok());
        let created = result.unwrap();
        assert!(created.path.exists());
        assert_eq!(created.path.parent().unwrap(), test_dir.join("inbox"));
        let content = fs::read_to_string(&created.path).unwrap();
        assert!(content.contains("# today"));
        assert!(content.contains("Date: "));
        assert!(content.contains("description:"));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_create_note_with_template_uses_schema_create() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("mdb_test_schema_create");
        let _ = fs::remove_dir_all(&test_dir);

        let tmpl_dir = test_dir.join("templates");
        fs::create_dir_all(&tmpl_dir).unwrap();
        fs::write(
            tmpl_dir.join("customer.md"),
            r#"---
type: ignored
_schema:
  create:
    type: company
    tags: []
---
# {{name}}"#,
        )
        .unwrap();

        let result = create_note(&test_dir, "acme", Some("customer"));
        assert!(result.is_ok());
        let created = result.unwrap();
        let content = fs::read_to_string(&created.path).unwrap();
        assert!(content.contains("type: company"));
        assert!(content.contains("tags: []"));
        assert!(!content.contains("type: ignored"));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_create_note_with_template_auto_injects_templates_field() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("mdb_test_template_auto_link");
        let _ = fs::remove_dir_all(&test_dir);

        let tmpl_dir = test_dir.join("templates");
        fs::create_dir_all(&tmpl_dir).unwrap();
        fs::write(
            tmpl_dir.join("customer.md"),
            r#"---
_schema:
  create:
    type: company
---
# {{name}}"#,
        )
        .unwrap();

        let result = create_note(&test_dir, "acme", Some("customer"));
        assert!(result.is_ok());
        let created = result.unwrap();
        let content = fs::read_to_string(&created.path).unwrap();
        assert!(content.contains("templates:"));
        assert!(content.contains("- '[[customer]]'") || content.contains("- \"[[customer]]\""));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_create_note_with_template_ignores_legacy_outer_templates_field() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("mdb_test_legacy_outer_templates");
        let _ = fs::remove_dir_all(&test_dir);

        let tmpl_dir = test_dir.join("templates");
        fs::create_dir_all(&tmpl_dir).unwrap();
        fs::write(
            tmpl_dir.join("customer.md"),
            r#"---
templates:
  - "[[legacy-template]]"
_schema:
  create:
    type: company
---
# {{name}}"#,
        )
        .unwrap();

        let result = create_note(&test_dir, "acme", Some("customer"));
        assert!(result.is_ok());
        let created = result.unwrap();
        let content = fs::read_to_string(&created.path).unwrap();
        assert!(!content.contains("[[legacy-template]]"));
        assert!(content.contains("- '[[customer]]'") || content.contains("- \"[[customer]]\""));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_create_note_template_not_found() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("mdb_test_notfound");
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        let result = create_note(&test_dir, "test", Some("nonexistent"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_create_note_rejects_path_like_name() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("mdb_test_invalid_name");
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        let result = create_note(&test_dir, "notes/test-note", None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must not include directories")
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_create_note_with_template_location_uses_template_directory() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("mdb_test_template_location");
        let _ = fs::remove_dir_all(&test_dir);

        let tmpl_dir = test_dir.join("templates");
        fs::create_dir_all(&tmpl_dir).unwrap();
        fs::write(
            tmpl_dir.join("customer.md"),
            "---\n_schema:\n  location: customers/\n---\n# {{name}}",
        )
        .unwrap();

        let result = create_note(&test_dir, "acme", Some("customer"));
        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.path.parent().unwrap(), test_dir.join("customers"));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_creation_timestamp_uses_a_timezone_aware_datetime() {
        let now = CreationTimestamp::from(Local::now());
        assert!(!now.date.is_empty());
        assert!(!now.time.is_empty());
        assert!(!now.datetime.is_empty());
        assert!(now.datetime.contains('T'));
        assert!(now.datetime.contains('+') || now.datetime.ends_with('Z'));
    }

    #[test]
    fn test_template_filename_pattern_uses_the_same_creation_timestamp() {
        let template = TemplateDocument::from_content(
            r#"---
_schema:
  filename:
    pattern: "{{timestamp}}_{{name}}"
  create:
    captured_at: "{{datetime}}"
---
# {{name}}"#,
        );
        let now = CreationTimestamp {
            date: "2026-07-23".to_string(),
            time: "14:32:08".to_string(),
            datetime: "2026-07-23T14:32:08+08:00".to_string(),
            timestamp: "2026-07-23-14-32-08".to_string(),
        };

        let (content, _, pattern) =
            process_template_document_at(&template, "富途拜访", &now).unwrap();

        assert_eq!(pattern.as_deref(), Some("2026-07-23-14-32-08_富途拜访"));
        assert!(content.contains("2026-07-23T14:32:08+08:00"));
    }

    #[test]
    fn test_template_filename_pattern_uses_a_stable_suffix_on_collision() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("2026-07-23-14-32-08_富途拜访.md"), "").unwrap();

        let available =
            find_available_note_name(temp_dir.path(), "2026-07-23-14-32-08_富途拜访").unwrap();

        assert_eq!(available, "2026-07-23-14-32-08_富途拜访_02");
    }

    #[test]
    fn test_process_template_removes_schema() {
        let content = r#"---
_schema:
  description: Customer template
  required: [name]
  create:
    type: company
    template: company_customer
---
# Content"#;
        let (result, _) = process_template(content, "test").unwrap();
        assert!(!result.contains("_schema"));
        assert!(result.contains("type: company"));
        assert!(result.contains("template: company_customer"));
        assert!(result.contains("description:"));
    }

    #[test]
    fn test_process_template_preserves_frontmatter_fields() {
        let content = r#"---
_schema:
  create:
    type: person
    template: person_work
    name: John
    age: 30
---
# Body"#;
        let (result, _) = process_template(content, "test").unwrap();
        assert!(result.contains("type: person"));
        assert!(result.contains("template: person_work"));
        assert!(result.contains("name: John"));
        assert!(result.contains("age: 30"));
    }

    #[test]
    fn test_process_template_without_frontmatter() {
        let content = "# Hello {{name}}\n\nSome content";
        let (result, _) = process_template(content, "world").unwrap();
        assert!(result.contains("# Hello world"));
        assert!(result.contains("Some content"));
        assert!(result.contains("description:"));
    }

    #[test]
    fn test_process_template_with_name_variable() {
        let content = r#"---
_schema:
  create:
    type: test
---
# {{name}}
Content"#;
        let (result, _) = process_template(content, "my-note").unwrap();
        assert!(result.contains("# my-note"));
    }

    #[test]
    fn test_process_template_frontmatter_format() {
        let content = r#"---
_schema:
  create:
    type: person
    template: person_work
    aliases: []
---
# Body"#;
        let (result, _) = process_template(content, "test").unwrap();
        assert!(result.starts_with("---\n"));
        assert!(result.contains("type: person\n"));
        assert!(result.contains("template: person_work\n"));
        assert!(result.contains("description:"));
        assert!(result.contains("---\n\n# Body"));
    }

    #[test]
    fn test_process_template_location() {
        let content = r#"---
_schema:
  description: Customer template
  location: customers/
  create:
    type: company
    template: company_customer
---
# Body"#;
        let (result, location) = process_template(content, "test").unwrap();
        assert!(result.contains("type: company"));
        assert!(result.contains("template: company_customer"));
        assert!(!result.contains("_schema"));
        assert!(!result.contains("location"));
        assert_eq!(location, Some("customers/".to_string()));
    }
}
