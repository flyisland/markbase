use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use gray_matter::Matter;
use gray_matter::engine::YAML;
use regex::Regex;
use serde_json::Value;

static RE_NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{\s*name\s*\}\}").unwrap());
static RE_DATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{\s*date\s*\}\}").unwrap());
static RE_TIME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{\s*time\s*\}\}").unwrap());
static RE_DATETIME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{\s*datetime\s*\}\}").unwrap());
static RE_DIRECTIVES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<!--\s*\[[^\]]+\].*?-->").unwrap());

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
    let template_dir = base_dir.join("templates");

    let (content, location) = match template_name {
        Some(tmpl) => {
            let template_path = template_dir.join(format!("{}.md", tmpl));
            if !template_path.exists() {
                return Err(format!(
                    "Template '{}' not found at '{}'",
                    tmpl,
                    template_path.display()
                )
                .into());
            }
            let tmpl_content = fs::read_to_string(&template_path)?;
            process_template(&tmpl_content, name)?
        }
        None => (String::new(), None),
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

fn process_template(
    content: &str,
    name: &str,
) -> Result<(String, Option<String>), Box<dyn std::error::Error>> {
    let matter = Matter::<YAML>::new();

    match matter.parse::<Value>(content) {
        Ok(parsed) => {
            let frontmatter = parsed.data;
            let body = parsed.content;

            let mut outer_fields: HashMap<String, String> = HashMap::new();
            let mut location: Option<String> = None;

            if let Some(fm) = frontmatter
                && let Some(obj) = fm.as_object()
            {
                for (key, value) in obj.iter() {
                    if key == "_schema" {
                        if let Some(schema_obj) = value.as_object()
                            && let Some(loc_val) = schema_obj.get("location")
                            && let Value::String(loc) = loc_val
                        {
                            location = Some(loc.clone());
                        }
                        continue;
                    }
                    let val_str = match value {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    outer_fields.insert(key.clone(), val_str);
                }
            }

            let clean_body = clean_body_directives(&body);
            let skeleton_frontmatter = build_skeleton_frontmatter(&outer_fields);

            let final_content = if skeleton_frontmatter.is_empty() {
                clean_body
            } else {
                format!("---\n{}---\n\n{}", skeleton_frontmatter, clean_body)
            };

            Ok((replace_template_variables(&final_content, name), location))
        }
        Err(_) => Ok((replace_template_variables(content, name), None)),
    }
}

fn build_skeleton_frontmatter(outer_fields: &HashMap<String, String>) -> String {
    if outer_fields.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();
    for (key, value) in outer_fields.iter() {
        if value.contains('\n') {
            lines.push(format!("{}: |", key));
            for line in value.lines() {
                lines.push(format!("  {}", line));
            }
        } else if value.contains(':')
            || value.contains('#')
            || value.starts_with('"')
            || value.starts_with('\'')
        {
            lines.push(format!("{}: '{}'", key, value.replace('\'', "''")));
        } else {
            lines.push(format!("{}: {}", key, value));
        }
    }

    let mut result = lines.join("\n");
    result.push('\n');
    result
}

fn clean_body_directives(body: &str) -> String {
    RE_DIRECTIVES.replace_all(body, "").to_string()
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
        assert!(created.content.is_empty());

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
    }

    #[test]
    fn test_process_template_removes_directives() {
        let content = r#"---
type: test
---
# Section 1
<!-- [Fill]: Write something here -->

## Section 2
<!-- [Update]: Overwrite
     Some policy content -->
Content here"#;
        let (result, _) = process_template(content, "test").unwrap();
        assert!(!result.contains("[Fill]"));
        assert!(!result.contains("[Update]"));
        assert!(result.contains("# Section 1"));
        assert!(result.contains("## Section 2"));
        assert!(result.contains("Content here"));
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
