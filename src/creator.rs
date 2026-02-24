use std::fs;
use std::path::{Path, PathBuf};

pub fn create_note(
    base_dir: &Path,
    name: &str,
    template_name: Option<&str>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let template_dir = base_dir.join("templates");
    let target_path = base_dir.join(format!("{}.md", name));

    if target_path.exists() {
        return Err(format!("Note '{}' already exists", name).into());
    }

    let content = match template_name {
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
            replace_template_variables(&tmpl_content, name)
        }
        None => String::new(),
    };

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&target_path, content)?;

    Ok(target_path)
}

fn replace_template_variables(content: &str, name: &str) -> String {
    let now = chrono_lite_now();
    content
        .replace("{{name}}", name)
        .replace("{{date}}", &now.date)
        .replace("{{time}}", &now.time)
        .replace("{{datetime}}", &now.datetime)
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
    fn test_create_note_without_template() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("mdb_test_create");
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        let result = create_note(&test_dir, "test-note", None);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "test-note.md");

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
        let path = result.unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# today"));
        assert!(content.contains("Date: "));

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
}
