use std::error::Error;
use std::fmt;
use std::path::{Component, Path};

pub fn validate_note_name(name: &str) -> Result<(), ValidationError> {
    validate_path_free_name(name, "note name")
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
    use super::{validate_note_name, validate_path_free_name};

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
    fn allows_extension_without_directories() {
        assert!(validate_note_name("tasks.base").is_ok());
        assert!(validate_note_name("aaa.jpeg").is_ok());
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
