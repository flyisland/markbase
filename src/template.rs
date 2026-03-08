use gray_matter::Matter;
use gray_matter::engine::YAML;
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;

const DESCRIPTION_PROMPT: &str = "一句话说明这个 note 是什么";
pub const DEFAULT_NOTE_DESCRIPTION: &str = "临时笔记";

#[derive(Debug, Clone)]
pub struct TemplateDocument {
    frontmatter: Map<String, Value>,
    body: String,
    location: Option<String>,
}

impl TemplateDocument {
    pub fn from_content(content: &str) -> Self {
        let matter = Matter::<YAML>::new();

        let (frontmatter, body) = match matter.parse::<Value>(content) {
            Ok(parsed) => {
                let frontmatter = parsed
                    .data
                    .and_then(|value| value.as_object().cloned())
                    .unwrap_or_default();
                (frontmatter, parsed.content.to_string())
            }
            Err(_) => (Map::new(), content.to_string()),
        };

        let (frontmatter, location) = normalize_frontmatter(frontmatter);

        Self {
            frontmatter,
            body,
            location,
        }
    }

    pub fn load(base_dir: &Path, name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let template_path = base_dir.join("templates").join(format!("{}.md", name));
        if !template_path.exists() {
            return Err(format!("Template '{}' not found", name).into());
        }

        let content = fs::read_to_string(&template_path)?;
        Ok(Self::from_content(&content))
    }

    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }

    pub fn render_for_describe(&self) -> Result<String, Box<dyn std::error::Error>> {
        render_document(&self.frontmatter, &self.body)
    }

    pub fn render_for_instance(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut frontmatter = self.frontmatter.clone();
        frontmatter.remove("_schema");
        render_document(&frontmatter, &self.body)
    }
}

fn normalize_frontmatter(
    mut frontmatter: Map<String, Value>,
) -> (Map<String, Value>, Option<String>) {
    if !matches!(frontmatter.get("description"), Some(Value::String(_))) {
        frontmatter.insert("description".to_string(), Value::String(String::new()));
    }

    let schema_value = frontmatter
        .entry("_schema".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !schema_value.is_object() {
        *schema_value = Value::Object(Map::new());
    }

    let schema = schema_value.as_object_mut().expect("schema must be object");

    let location = schema
        .get("location")
        .and_then(Value::as_str)
        .map(String::from);

    let required_value = schema
        .entry("required".to_string())
        .or_insert_with(|| Value::Array(vec![]));
    if !required_value.is_array() {
        *required_value = Value::Array(vec![]);
    }
    let required = required_value
        .as_array_mut()
        .expect("required must be array");
    if !required
        .iter()
        .any(|value| value.as_str() == Some("description"))
    {
        required.push(Value::String("description".to_string()));
    }

    let properties_value = schema
        .entry("properties".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !properties_value.is_object() {
        *properties_value = Value::Object(Map::new());
    }
    let properties = properties_value
        .as_object_mut()
        .expect("properties must be object");

    let description_value = properties
        .entry("description".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !description_value.is_object() {
        *description_value = Value::Object(Map::new());
    }
    let description = description_value
        .as_object_mut()
        .expect("description schema must be object");
    description
        .entry("type".to_string())
        .or_insert_with(|| Value::String("text".to_string()));
    description
        .entry("description".to_string())
        .or_insert_with(|| Value::String(DESCRIPTION_PROMPT.to_string()));

    (frontmatter, location)
}

fn render_document(
    frontmatter: &Map<String, Value>,
    body: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    if frontmatter.is_empty() {
        return Ok(body.to_string());
    }

    let yaml = serde_yaml::to_string(&Value::Object(frontmatter.clone()))?;
    let yaml = yaml.trim_start_matches("---\n");
    let yaml = yaml.trim_end();

    if body.is_empty() {
        Ok(format!("---\n{}\n---\n", yaml))
    } else {
        Ok(format!("---\n{}\n---\n\n{}", yaml, body))
    }
}

pub fn default_note_content() -> Result<String, Box<dyn std::error::Error>> {
    let frontmatter =
        Map::from_iter([("description".to_string(), json!(DEFAULT_NOTE_DESCRIPTION))]);
    render_document(&frontmatter, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_content_normalizes_description_fields() {
        let doc = TemplateDocument::from_content(
            r#"---
type: company
_schema:
  location: customers/
---
# Body"#,
        );

        let described = doc.render_for_describe().unwrap();
        assert!(described.contains("description: ''") || described.contains("description: \"\""));
        assert!(described.contains("required:"));
        assert!(described.contains("- description"));
        assert!(described.contains("type: text"));
        assert_eq!(doc.location(), Some("customers/"));
    }

    #[test]
    fn test_render_for_instance_strips_schema() {
        let doc = TemplateDocument::from_content(
            r#"---
type: company
_schema:
  location: customers/
---
# Body"#,
        );

        let instance = doc.render_for_instance().unwrap();
        assert!(instance.contains("type: company"));
        assert!(instance.contains("description: ''") || instance.contains("description: \"\""));
        assert!(!instance.contains("_schema"));
        assert!(!instance.contains("location:"));
    }

    #[test]
    fn test_default_note_content_contains_description() {
        let content = default_note_content().unwrap();
        assert!(content.contains("description: 临时笔记"));
    }
}
