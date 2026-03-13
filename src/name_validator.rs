use std::error::Error;
use std::fmt;
use std::path::{Component, Path};

pub fn validate_note_name(name: &str) -> Result<(), ValidationError> {
    validate_path_free_name(name, "note name")?;

    if Path::new(name).extension().is_some() {
        return Err(ValidationError(format!(
            "Invalid note name '{}': note name must not include a file extension",
            name
        )));
    }

    Ok(())
}

pub fn validate_render_target_name(name: &str) -> Result<(), ValidationError> {
    validate_path_free_name(name, "render target")?;

    match Path::new(name).extension().and_then(|ext| ext.to_str()) {
        None => Ok(()),
        Some("base") => Ok(()),
        Some(_) => Err(ValidationError(format!(
            "Invalid render target '{}': render target must be a note name or .base filename",
            name
        ))),
    }
}

pub fn validate_resolve_input(value: &str) -> Result<(), ValidationError> {
    validate_path_free_name(value, "resolve input")?;

    if Path::new(value).extension().is_some() {
        return Err(ValidationError(format!(
            "Invalid resolve input '{}': resolve input must not include a file extension",
            value
        )));
    }

    Ok(())
}

pub fn validate_path_free_name(value: &str, label: &str) -> Result<(), ValidationError> {
    let path = Path::new(value);

    if value.is_empty() {
        return Err(ValidationError(format!(
            "{} cannot be empty",
            uppercase_first(label)
        )));
    }

    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
        || path.components().count() != 1
    {
        return Err(ValidationError(format!(
            "Invalid {} '{}': {} must not include directories",
            label, value, label
        )));
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError(String);

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for ValidationError {}

fn uppercase_first(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        validate_note_name, validate_path_free_name, validate_render_target_name,
        validate_resolve_input,
    };

    #[test]
    fn rejects_path_like_note_name() {
        let err = validate_note_name("logs/opportunities/acme").unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid note name 'logs/opportunities/acme': note name must not include directories"
        );
    }

    #[test]
    fn rejects_non_normal_segments() {
        let err = validate_note_name("../acme").unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid note name '../acme': note name must not include directories"
        );
    }

    #[test]
    fn rejects_note_name_with_extension() {
        let err = validate_note_name("acme.md").unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid note name 'acme.md': note name must not include a file extension"
        );
    }

    #[test]
    fn allows_base_render_target() {
        assert!(validate_render_target_name("tasks.base").is_ok());
    }

    #[test]
    fn rejects_non_base_render_target_extension() {
        let err = validate_render_target_name("diagram.png").unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid render target 'diagram.png': render target must be a note name or .base filename"
        );
    }

    #[test]
    fn rejects_resolve_input_with_extension() {
        let err = validate_resolve_input("customer.md").unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid resolve input 'customer.md': resolve input must not include a file extension"
        );
    }

    #[test]
    fn custom_label_uses_same_rule() {
        let err = validate_path_free_name("nested/new-name", "old_name").unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid old_name 'nested/new-name': old_name must not include directories"
        );
    }
}
