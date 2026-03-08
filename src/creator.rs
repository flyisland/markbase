use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::template::{TemplateDocument, default_note_content};

static RE_NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{\s*name\s*\}\}").unwrap());
static RE_DATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{\s*date\s*\}\}").unwrap());
static RE_TIME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{\s*time\s*\}\}").unwrap());
static RE_DATETIME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{\s*datetime\s*\}\}").unwrap());

#[derive(Debug)]
pub struct CreatedNote {
    pub path: PathBuf,
    pub content: String,
}

pub fn create_note(
    base_dir: &Path,
    name: &str,
    template_name: Option<&str>,
) -> Result<CreatedNote, Box<dyn std::error::Error>> {
    let (content, location) = match template_name {
        Some(tmpl) => process_template_document(&TemplateDocument::load(base_dir, tmpl)?, name)?,
        None => (default_note_content()?, None),
    };

    let target_path = if let Some(loc) = location {
        let dir = base_dir.join(&loc);
        let file_name = format!("{}.md", name);
        dir.join(&file_name)
    } else {
        base_dir.join(format!("{}.md", name))
    };

    if target_path.exists() {
        return Err(format!("Note '{}' already exists", target_path.display()).into());
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&target_path, content.clone())?;

    Ok(CreatedNote {
        path: target_path,
        content,
    })
}

fn replace_template_variables(content: &str, name: &str) -> String {
    let now = chrono_lite_now();

    let result = RE_NAME.replace_all(content, name);
    let result = RE_DATE.replace_all(&result, &now.date);
    let result = RE_TIME.replace_all(&result, &now.time);
    RE_DATETIME.replace_all(&result, &now.datetime).to_string()
}

fn process_template_document(
    template: &TemplateDocument,
    name: &str,
) -> Result<(String, Option<String>), Box<dyn std::error::Error>> {
    let content = template.render_for_instance()?;
    Ok((
        replace_template_variables(&content, name),
        template.location().map(ToString::to_string),
    ))
}

#[cfg(test)]
fn process_template(
    content: &str,
    name: &str,
) -> Result<(String, Option<String>), Box<dyn std::error::Error>> {
    process_template_document(&TemplateDocument::from_content(content), name)
}

fn chrono_lite_now() -> ChronoLite {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    let mut year = 1970;
    let mut remaining_days = days as i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months: [i64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for days_in_month in days_in_months.iter() {
        if remaining_days < *days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }
    let day = remaining_days + 1;

    let date = format!("{:04}-{:02}-{:02}", year, month, day);
    let time = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
    let datetime = format!("{} {}", date, time);

    ChronoLite {
        date,
        time,
        datetime,
    }
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

struct ChronoLite {
    date: String,
    time: String,
    datetime: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_template_variables() {
        let content = "# {{name}}\n\nDate: {{date}}\nTime: {{time}}\n";
        let result = replace_template_variables(content, "my-note");
        assert!(result.contains("# my-note"));
        assert!(result.contains("Date: "));
        assert!(result.contains("Time: "));
    }

    #[test]
    fn test_replace_partial_variables() {
        let content = "Title: {{name}}\nOnly date: {{date}}";
        let result = replace_template_variables(content, "test");
        assert!(result.contains("Title: test"));
        assert!(result.contains("Only date: "));
        assert!(!result.contains("{{time}}"));
    }

    #[test]
    fn test_replace_variables_with_spaces() {
        let content = "# {{ name }}\nDate: {{ date }}\nTime: {{ time }}";
        let result = replace_template_variables(content, "my-note");
        assert!(result.contains("# my-note"));
        assert!(result.contains("Date: "));
        assert!(result.contains("Time: "));
        assert!(!result.contains("{{"));
    }

    #[test]
    fn test_replace_variables_mixed_format() {
        let content = "{{name}} and {{ name }} and {{date}} and {{ date }}";
        let result = replace_template_variables(content, "test");
        assert!(result.contains("test and test and "));
    }

    #[test]
    fn test_replace_variables_multiple_spaces() {
        let content = "{{  name  }} {{date}}";
        let result = replace_template_variables(content, "x");
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
        assert_eq!(
            created.path.file_name().unwrap().to_str().unwrap(),
            "test-note.md"
        );
        assert!(created.content.contains("description: 临时笔记"));

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
        assert!(created.content.contains("# today"));
        assert!(created.content.contains("Date: "));
        assert!(created.content.contains("description:"));

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
    fn test_chrono_lite_now() {
        let now = chrono_lite_now();
        assert!(!now.date.is_empty());
        assert!(!now.time.is_empty());
        assert!(!now.datetime.is_empty());
        assert!(now.datetime.contains(' '));
    }

    #[test]
    fn test_process_template_removes_schema() {
        let content = r#"---
type: company
template: company_customer
_schema:
  description: Customer template
  required: [name]
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
type: company
template: company_customer
_schema:
  description: Customer template
  location: customers/
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
