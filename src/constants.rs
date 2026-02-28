#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum FieldType {
    String,
    Integer,
    Timestamp,
    Array,
    Json,
}

pub fn get_reserved_field_type(field: &str) -> Option<FieldType> {
    match field {
        "path" | "folder" | "name" | "ext" | "content" => Some(FieldType::String),
        "size" => Some(FieldType::Integer),
        "ctime" | "mtime" => Some(FieldType::Timestamp),
        "tags" | "links" | "backlinks" | "embeds" => Some(FieldType::Array),
        _ => None,
    }
}

pub const RESERVED_FIELDS: &[&str] = &[
    "path",
    "folder",
    "name",
    "ext",
    "size",
    "ctime",
    "mtime",
    "content",
    "tags",
    "links",
    "backlinks",
    "embeds",
];
