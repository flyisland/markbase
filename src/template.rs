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
    instance_frontmatter: Map<String, Value>,
    body: String,
    location: Option<String>,
    name: Option<String>,
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

        let (frontmatter, instance_frontmatter, location) = normalize_frontmatter(frontmatter);

        Self {
            frontmatter,
            instance_frontmatter,
            body,
            location,
            name: None,
        }
    }

    pub fn load(base_dir: &Path, name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let template_path = base_dir.join("templates").join(format!("{}.md", name));
        if !template_path.exists() {
            return Err(format!("Template '{}' not found", name).into());
        }

        let content = fs::read_to_string(&template_path)?;
        let mut document = Self::from_content(&content);
        document.name = Some(name.to_string());
        Ok(document)
    }

    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }

    pub fn render_for_describe(&self) -> Result<String, Box<dyn std::error::Error>> {
        render_document(&self.frontmatter, &self.body)
    }

    pub fn render_for_instance(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut frontmatter = self.instance_frontmatter.clone();
        frontmatter.remove("_schema");
        frontmatter.remove("templates");

        if let Some(name) = &self.name {
            frontmatter.insert(
                "templates".to_string(),
                Value::Array(vec![Value::String(format!("[[{}]]", name))]),
            );
        }

        render_document(&frontmatter, &self.body)
    }
}

fn normalize_frontmatter(
    mut frontmatter: Map<String, Value>,
) -> (Map<String, Value>, Map<String, Value>, Option<String>) {
    let location = {
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

        location
    };

    let schema_snapshot = frontmatter
        .get("_schema")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let legacy_instance_keys = collect_legacy_instance_keys(&frontmatter, &schema_snapshot);
    let legacy_instance_frontmatter =
        extract_legacy_instance_frontmatter(&frontmatter, &legacy_instance_keys);

    let instance_frontmatter = {
        let schema = frontmatter
            .get_mut("_schema")
            .and_then(Value::as_object_mut)
            .expect("schema must be object");
        let instance_value = schema
            .entry("instance".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !instance_value.is_object() {
            *instance_value = Value::Object(Map::new());
        }
        let instance = instance_value
            .as_object_mut()
            .expect("instance schema must be object");
        for (key, value) in legacy_instance_frontmatter {
            instance.entry(key).or_insert(value);
        }
        if !matches!(instance.get("description"), Some(Value::String(_))) {
            instance.insert("description".to_string(), Value::String(String::new()));
        }

        instance.clone()
    };

    for key in legacy_instance_keys {
        frontmatter.remove(&key);
    }

    (frontmatter, instance_frontmatter, location)
}

fn collect_legacy_instance_keys(
    frontmatter: &Map<String, Value>,
    schema: &Map<String, Value>,
) -> Vec<String> {
    let property_keys: std::collections::HashSet<&str> = schema
        .iter()
        .filter(|(key, _)| key.as_str() == "properties")
        .flat_map(|(_, value)| {
            value
                .as_object()
                .into_iter()
                .flat_map(|properties| properties.keys().map(String::as_str))
        })
        .collect();
    let required_keys: std::collections::HashSet<&str> = schema
        .iter()
        .filter(|(key, _)| key.as_str() == "required")
        .flat_map(|(_, value)| {
            value
                .as_array()
                .into_iter()
                .flat_map(|values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        })
        .collect();

    frontmatter
        .keys()
        .filter(|key| {
            let key = key.as_str();
            key != "_schema"
                && key != "templates"
                && (matches!(key, "description" | "type" | "tags" | "aliases")
                    || property_keys.contains(key)
                    || required_keys.contains(key))
        })
        .cloned()
        .collect()
}

fn extract_legacy_instance_frontmatter(
    frontmatter: &Map<String, Value>,
    legacy_instance_keys: &[String],
) -> Map<String, Value> {
    legacy_instance_keys
        .iter()
        .filter_map(|key| {
            frontmatter
                .get(key)
                .map(|value| (key.clone(), value.clone()))
        })
        .collect()
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
    fn test_from_content_normalizes_instance_block() {
        let doc = TemplateDocument::from_content(
            r#"---
type: company
owner: Alice
_schema:
  location: customers/
---
# Body"#,
        );

        let described = doc.render_for_describe().unwrap();
        assert!(described.contains("required:"));
        assert!(described.contains("- description"));
        assert!(described.contains("type: text"));
        assert!(described.contains("instance:"));
        assert!(described.contains("type: company"));
        assert!(described.contains("description: ''") || described.contains("description: \"\""));
        assert!(described.contains("owner: Alice"));
        assert!(!described.contains("instance:\n    owner: Alice"));
        assert_eq!(doc.location(), Some("customers/"));
    }

    #[test]
    fn test_render_for_instance_uses_instance_block() {
        let doc = TemplateDocument::from_content(
            r#"---
type: company
_schema:
  location: customers/
  instance:
    type: person
---
# Body"#,
        );

        let instance = doc.render_for_instance().unwrap();
        assert!(instance.contains("type: person"));
        assert!(!instance.contains("type: company"));
        assert!(instance.contains("description: ''") || instance.contains("description: \"\""));
        assert!(!instance.contains("_schema"));
        assert!(!instance.contains("location:"));
    }

    #[test]
    fn test_render_for_instance_does_not_copy_legacy_outer_templates_field() {
        let doc = TemplateDocument::from_content(
            r#"---
templates:
  - "[[legacy]]"
_schema:
  instance:
    type: company
---
# Body"#,
        );

        let instance = doc.render_for_instance().unwrap();
        assert!(instance.contains("type: company"));
        assert!(!instance.contains("[[legacy]]"));
    }

    #[test]
    fn test_render_for_instance_does_not_copy_arbitrary_outer_frontmatter() {
        let doc = TemplateDocument::from_content(
            r#"---
owner: Alice
_schema:
  instance:
    type: company
---
# Body"#,
        );

        let instance = doc.render_for_instance().unwrap();
        assert!(instance.contains("type: company"));
        assert!(!instance.contains("owner: Alice"));
    }

    #[test]
    fn test_default_note_content_contains_description() {
        let content = default_note_content().unwrap();
        assert!(content.contains("description: 临时笔记"));
    }
}
