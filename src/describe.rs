use std::path::Path;

use crate::template::TemplateDocument;

pub fn describe_template(
    base_dir: &Path,
    name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    TemplateDocument::load(base_dir, name)?.render_for_describe()
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
        let content = result.unwrap();
        assert!(content.contains("description:"));
        assert!(content.contains("_schema:"));
        assert!(content.contains("# Daily Note"));

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
