use std::fs;
use std::path::Path;

pub fn describe_template(
    base_dir: &Path,
    name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let template_path = base_dir.join("templates").join(format!("{}.md", name));
    if !template_path.exists() {
        return Err(format!("Template '{}' not found", name).into());
    }
    Ok(fs::read_to_string(&template_path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_describe_template_success() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("mdb_describe_test");
        let _ = fs::remove_dir_all(&test_dir);

        let tmpl_dir = test_dir.join("templates");
        fs::create_dir_all(&tmpl_dir).unwrap();
        fs::write(tmpl_dir.join("daily.md"), "# Daily Note\nDate: {{date}}").unwrap();

        let result = describe_template(&test_dir, "daily");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "# Daily Note\nDate: {{date}}");

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_describe_template_not_found() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("mdb_describe_test2");
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        let result = describe_template(&test_dir, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        let _ = fs::remove_dir_all(&test_dir);
    }
}
