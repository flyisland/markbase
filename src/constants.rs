#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldType {
    String,
    Integer,
    Timestamp,
    Array,
}

pub fn get_reserved_field_type(field: &str) -> Option<FieldType> {
    match field {
        "path" | "folder" | "name" | "ext" => Some(FieldType::String),
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
    "tags",
    "links",
    "backlinks",
    "embeds",
];
