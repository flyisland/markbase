use gray_matter::Matter;
use gray_matter::engine::YAML;
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

use crate::db::Database;
use crate::extractor::Extractor;
use crate::link_syntax::{LinkKind, LinkToken, ScanContext, scan_link_tokens};
use crate::name_validator::validate_note_name;

static DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d{4}-\d{2}-\d{2}$").expect("invalid regex: DATE_RE"));

static DATETIME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$").expect("invalid regex: DATETIME_RE")
});

#[derive(Debug)]
pub struct VerifyIssue {
    pub level: IssueLevel,
    pub message: String,
    pub field_definition: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum IssueLevel {
    Error,
    Warn,
    Info,
}

pub struct VerifyResult {
    pub note_path: Option<String>,
    pub template_names: Vec<String>,
    pub issues: Vec<VerifyIssue>,
}

impl VerifyResult {
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.level == IssueLevel::Error)
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.level == IssueLevel::Error)
            .count()
    }

    pub fn warn_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.level == IssueLevel::Warn)
            .count()
    }
}

fn check_global_description(name: &str, properties: &Value, issues: &mut Vec<VerifyIssue>) {
    let field_definition = Some(global_description_definition());

    match properties.get("description") {
        None => issues.push(VerifyIssue {
            level: IssueLevel::Error,
            message: format!("note '{}' is missing global field 'description'.", name),
            field_definition: field_definition.clone(),
        }),
        Some(Value::String(value)) if value.trim().is_empty() => issues.push(VerifyIssue {
            level: IssueLevel::Error,
            message: format!("note '{}' has empty global field 'description'.", name),
            field_definition: field_definition.clone(),
        }),
        Some(Value::String(_)) => {}
        Some(_) => issues.push(VerifyIssue {
            level: IssueLevel::Error,
            message: format!(
                "note '{}' has invalid global field 'description'. Expected non-empty text, got '{}'.",
                name,
                properties
                    .get("description")
                    .map(get_actual_type)
                    .unwrap_or("null")
            ),
            field_definition,
        }),
    }
}

pub fn verify_note(
    base_dir: &Path,
    db: &Database,
    name: &str,
) -> Result<VerifyResult, Box<dyn std::error::Error>> {
    validate_note_name(name)?;

    let mut issues = Vec::new();

    let notes = db.get_notes_by_name(name)?;
    if notes.is_empty() {
        issues.push(VerifyIssue {
            level: IssueLevel::Error,
            message: format!("note '{}' not found in the vault.", name),
            field_definition: None,
        });
        return Ok(VerifyResult {
            note_path: None,
            template_names: vec![],
            issues,
        });
    }

    if notes.len() > 1 {
        issues.push(VerifyIssue {
            level: IssueLevel::Error,
            message: format!("multiple notes found with name '{}'", name),
            field_definition: None,
        });
        return Ok(VerifyResult {
            note_path: None,
            template_names: vec![],
            issues,
        });
    }

    let note = &notes[0];
    let note_path_value = note.path.clone();
    let make_result = |template_names: Vec<String>, issues: Vec<VerifyIssue>| VerifyResult {
        note_path: Some(note_path_value.clone()),
        template_names,
        issues,
    };
    let folder = note.folder.clone();
    let properties = note.properties.clone();
    let note_path = base_dir.join(&note.path);
    let note_content = std::fs::read_to_string(&note_path)
        .map_err(|e| format!("failed to read note file '{}': {}", note_path.display(), e))?;
    let extracted = Extractor::extract(&note_content);
    let note_embeds: HashSet<String> = extracted.embeds.iter().cloned().collect();
    check_global_description(name, &properties, &mut issues);

    let templates_val = properties.get("templates");
    let templates_arr = match templates_val {
        Some(Value::Array(arr)) if !arr.is_empty() => arr,
        _ => {
            issues.push(VerifyIssue {
                level: IssueLevel::Error,
                message: format!(
                    "note '{}' has no 'templates' field. Cannot determine schema.",
                    name
                ),
                field_definition: None,
            });
            return Ok(make_result(vec![], issues));
        }
    };

    let mut template_file_names = Vec::new();
    for (i, item) in templates_arr.iter().enumerate() {
        let link_str = match item {
            Value::String(s) => s,
            _ => {
                issues.push(VerifyIssue {
                    level: IssueLevel::Error,
                    message: format!(
                        "'templates' contains invalid link: '{}'. Each element must be an Obsidian wiki-link, e.g. \"[[template-name]]\".",
                        item
                    ),
                    field_definition: None,
                });
                return Ok(make_result(vec![], issues));
            }
        };

        let template_name =
            parse_pure_wikilink(link_str).map(|token| token.parsed.normalized_target);

        if let Some(tn) = template_name {
            template_file_names.push(tn);
        } else if i == 0 {
            issues.push(VerifyIssue {
                level: IssueLevel::Error,
                message: format!(
                    "'templates' contains invalid link: '{}'. Each element must be an Obsidian wiki-link, e.g. \"[[template-name]]\".",
                    link_str
                ),
                field_definition: None,
            });
            return Ok(make_result(vec![], issues));
        }
    }

    if template_file_names.is_empty() {
        issues.push(VerifyIssue {
            level: IssueLevel::Error,
            message: format!(
                "note '{}' has no 'templates' field. Cannot determine schema.",
                name
            ),
            field_definition: None,
        });
        return Ok(make_result(vec![], issues));
    }

    let mut all_schema_fields: std::collections::HashMap<String, SchemaFieldInfo> =
        std::collections::HashMap::new();
    let mut templates_with_schema: Vec<String> = Vec::new();

    for template_name in &template_file_names {
        let tmpl_path = base_dir
            .join("templates")
            .join(format!("{}.md", template_name));
        if !tmpl_path.exists() {
            issues.push(VerifyIssue {
                level: IssueLevel::Error,
                message: format!("template file 'templates/{}.md' not found.", template_name),
                field_definition: None,
            });
            return Ok(make_result(templates_with_schema, issues));
        }

        let content = match std::fs::read_to_string(&tmpl_path) {
            Ok(c) => c,
            Err(e) => {
                issues.push(VerifyIssue {
                    level: IssueLevel::Error,
                    message: format!(
                        "failed to read template file '{}': {}",
                        tmpl_path.display(),
                        e
                    ),
                    field_definition: None,
                });
                return Ok(make_result(templates_with_schema, issues));
            }
        };

        let matter = Matter::<YAML>::new();
        let parsed = match matter.parse::<Value>(&content) {
            Ok(p) => p,
            Err(e) => {
                issues.push(VerifyIssue {
                    level: IssueLevel::Error,
                    message: format!(
                        "failed to parse template frontmatter '{}': {}",
                        tmpl_path.display(),
                        e
                    ),
                    field_definition: None,
                });
                return Ok(make_result(templates_with_schema, issues));
            }
        };

        let fm = parsed
            .data
            .map(|v| serde_json::to_value(v).unwrap_or(Value::Null))
            .unwrap_or(Value::Null);
        let template_extracted = Extractor::extract(&content);

        verify_template_embedded_bases(
            template_name,
            &note_embeds,
            &template_extracted.embeds,
            &mut issues,
        );

        if let Some(schema_obj) = fm.get("_schema") {
            if let Some(props) = schema_obj.get("properties").and_then(|v| v.as_object()) {
                for (field_name, field_def) in props {
                    let field_type = field_def
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("text");

                    if let Some(existing) = all_schema_fields.get(field_name) {
                        if existing.field_type != field_type {
                            issues.push(VerifyIssue {
                                level: IssueLevel::Error,
                                message: format!(
                                    "field '{}' has conflicting type definitions across templates ('{}': '{}', '{}': '{}'). Using '{}' definition.",
                                    field_name,
                                    existing.template_name,
                                    existing.field_type,
                                    template_name,
                                    field_type,
                                    existing.template_name
                                ),
                                field_definition: None,
                            });
                        }
                    } else {
                        let format = field_def
                            .get("format")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        let enum_values =
                            field_def.get("enum").and_then(|v| v.as_array()).map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .map(String::from)
                                    .collect()
                            });
                        let description = field_def
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        let target = field_def
                            .get("target")
                            .and_then(|v| v.as_str())
                            .map(String::from);

                        all_schema_fields.insert(
                            field_name.to_string(),
                            SchemaFieldInfo {
                                field_type: field_type.to_string(),
                                template_name: template_name.clone(),
                                format,
                                enum_values,
                                description,
                                target,
                            },
                        );
                    }
                }
            }
            templates_with_schema.push(template_name.clone());

            let location = schema_obj.get("location").and_then(|v| v.as_str());

            if let Some(loc) = location {
                let normalized_folder = folder.trim_end_matches('/').to_string();
                let normalized_location = loc.trim_end_matches('/').to_string();
                if normalized_folder != normalized_location {
                    issues.push(VerifyIssue {
                        level: IssueLevel::Error,
                        message: format!(
                            "note '{}' is located at '{}/', but template '{}' requires location '{}'.",
                            name,
                            folder,
                            template_name,
                            loc
                        ),
                        field_definition: None,
                    });
                }
            }
        }

        if let Some(obj) = fm.as_object() {
            for (key, tmpl_val) in obj {
                if key == "_schema" {
                    continue;
                }

                let note_val = properties.get(key);

                if note_val.is_none() {
                    issues.push(VerifyIssue {
                        level: IssueLevel::Error,
                        message: format!(
                            "missing field '{}' (defined in template '{}').",
                            key, template_name
                        ),
                        field_definition: None,
                    });
                    continue;
                }

                let note_val = note_val.unwrap();

                if let Value::Array(tmpl_arr) = tmpl_val {
                    if let Value::Array(note_arr) = note_val {
                        let missing: Vec<String> = tmpl_arr
                            .iter()
                            .filter_map(|v| v.as_str())
                            .filter(|v| !note_arr.iter().any(|nv| nv.as_str() == Some(*v)))
                            .map(String::from)
                            .collect();
                        if !missing.is_empty() {
                            issues.push(VerifyIssue {
                                level: IssueLevel::Error,
                                message: format!(
                                    "list field '{}' is missing values required by template '{}'. Missing: [{}]",
                                    key,
                                    template_name,
                                    missing.join(", ")
                                ),
                                field_definition: None,
                            });
                        }
                    }
                } else if let Some(tmpl_str) = tmpl_val.as_str()
                    && !tmpl_str.is_empty()
                    && let Some(note_str) = note_val.as_str()
                    && note_str != tmpl_str
                {
                    issues.push(VerifyIssue {
                        level: IssueLevel::Error,
                        message: format!(
                            "field '{}' value mismatch. Expected: '{}' (from template '{}'), got: '{}'.",
                            key, tmpl_str, template_name, note_str
                        ),
                        field_definition: None,
                    });
                }
            }
        }

        if let Some(schema_obj) = fm.get("_schema") {
            let required: Vec<&str> = schema_obj
                .get("required")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            for field_name in required {
                let val = properties.get(field_name);
                let is_empty = match val {
                    None => true,
                    Some(Value::Null) => true,
                    Some(Value::String(s)) => s.is_empty(),
                    Some(Value::Array(arr)) => arr.is_empty(),
                    _ => false,
                };
                if is_empty {
                    let field_def = all_schema_fields.get(field_name);
                    let field_definition = field_def.map(format_field_definition);
                    issues.push(VerifyIssue {
                        level: IssueLevel::Error,
                        message: format!(
                            "required field '{}' is missing or empty (defined in _schema.required of '{}').",
                            field_name, template_name
                        ),
                        field_definition,
                    });
                }
            }

            if let Some(props) = schema_obj.get("properties").and_then(|v| v.as_object()) {
                for (field_name, field_def) in props {
                    let note_val = match properties.get(field_name) {
                        Some(v) => v,
                        None => continue,
                    };

                    if note_val.is_null() || is_value_empty(note_val) {
                        continue;
                    }

                    let field_type = field_def
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("text");

                    if !check_type(note_val, field_type) {
                        let actual_type = get_actual_type(note_val);
                        let field_def = all_schema_fields.get(field_name);
                        let field_definition = field_def.map(format_field_definition);
                        issues.push(VerifyIssue {
                            level: IssueLevel::Error,
                            message: format!(
                                "field '{}' type mismatch. Expected '{}' (from template '{}'), got '{}'.",
                                field_name, field_type, template_name, actual_type
                            ),
                            field_definition,
                        });
                    }

                    if let Some(enum_arr) = field_def.get("enum").and_then(|v| v.as_array()) {
                        let allowed: Vec<String> = enum_arr
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                        if field_type == "list" {
                            if let Some(arr) = note_val.as_array() {
                                for item in arr {
                                    if let Some(s) = item.as_str()
                                        && !allowed.contains(&s.to_string())
                                    {
                                        let field_def = all_schema_fields.get(field_name);
                                        let field_definition =
                                            field_def.map(format_field_definition);
                                        issues.push(VerifyIssue {
                                            level: IssueLevel::Error,
                                            message: format!(
                                                "field '{}' has invalid value '{}'. Allowed values (from template '{}'): [{}]",
                                                field_name, s, template_name, allowed.join(", ")
                                            ),
                                            field_definition,
                                        });
                                    }
                                }
                            }
                        } else if let Some(s) = note_val.as_str()
                            && !allowed.contains(&s.to_string())
                        {
                            let field_def = all_schema_fields.get(field_name);
                            let field_definition = field_def.map(format_field_definition);
                            issues.push(VerifyIssue {
                                level: IssueLevel::Error,
                                message: format!(
                                    "field '{}' has invalid value '{}'. Allowed values (from template '{}'): [{}]",
                                    field_name, s, template_name, allowed.join(", ")
                                ),
                                field_definition,
                            });
                        }
                    }

                    let field_format = field_def.get("format").and_then(|v| v.as_str());
                    if field_format == Some("link") {
                        let field_target = field_def.get("target").and_then(|v| v.as_str());

                        if let Value::Array(arr) = note_val {
                            for item in arr {
                                if let Some(link_val) = item.as_str() {
                                    verify_link_field(
                                        field_name,
                                        link_val,
                                        field_target,
                                        template_name,
                                        db,
                                        &all_schema_fields,
                                        &mut issues,
                                    );
                                }
                            }
                        } else if let Some(link_val) = note_val.as_str() {
                            verify_link_field(
                                field_name,
                                link_val,
                                field_target,
                                template_name,
                                db,
                                &all_schema_fields,
                                &mut issues,
                            );
                        }
                    }
                }
            }
        }
    }

    verify_embedded_bases(db, &extracted.embeds, &mut issues);

    Ok(make_result(templates_with_schema, issues))
}

#[derive(Clone)]
struct SchemaFieldInfo {
    field_type: String,
    template_name: String,
    format: Option<String>,
    enum_values: Option<Vec<String>>,
    description: Option<String>,
    target: Option<String>,
}

fn format_field_definition(field: &SchemaFieldInfo) -> String {
    let mut parts = vec![format!("type={}", field.field_type)];

    if let Some(ref fmt) = field.format {
        parts.push(format!("format={}", fmt));
    }

    if let Some(ref target) = field.target {
        parts.push(format!("target={}", target));
    }

    if let Some(ref enum_vals) = field.enum_values {
        parts.push(format!("enum=[{}]", enum_vals.join(", ")));
    }

    if let Some(ref desc) = field.description {
        parts.push(format!("description=\"{}\"", desc));
    }

    parts.join(", ")
}

fn global_description_definition() -> String {
    "scope=global, required=true, type=text, nonempty=true, description=\"一句话说明这个 note 是什么\"".to_string()
}

fn check_type(val: &Value, expected_type: &str) -> bool {
    match expected_type {
        "text" => val.is_string(),
        "number" => {
            val.is_number()
                || val
                    .as_str()
                    .map(|s| s.parse::<f64>().is_ok())
                    .unwrap_or(false)
        }
        "boolean" => val.is_boolean(),
        "date" => val.as_str().map(|s| DATE_RE.is_match(s)).unwrap_or(false),
        "datetime" => val
            .as_str()
            .map(|s| DATETIME_RE.is_match(s))
            .unwrap_or(false),
        "list" => val.is_array(),
        _ => true,
    }
}

fn get_actual_type(val: &Value) -> &'static str {
    if val.is_string() {
        "text"
    } else if val.is_number() {
        "number"
    } else if val.is_boolean() {
        "boolean"
    } else if val.is_array() {
        "list"
    } else if val.is_null() {
        "null"
    } else {
        "unknown"
    }
}

fn is_value_empty(val: &Value) -> bool {
    match val {
        Value::String(s) => s.is_empty(),
        Value::Array(arr) => arr.is_empty(),
        _ => false,
    }
}

fn verify_link_field(
    field_name: &str,
    link_val: &str,
    target_type: Option<&str>,
    template_name: &str,
    db: &Database,
    all_schema_fields: &std::collections::HashMap<String, SchemaFieldInfo>,
    issues: &mut Vec<VerifyIssue>,
) {
    let trimmed = link_val.trim();
    if trimmed.is_empty() {
        return;
    }

    if trimmed.starts_with("[?") {
        issues.push(VerifyIssue {
            level: IssueLevel::Info,
            message: format!(
                "field '{}' has dangling reference: '{}'.",
                field_name, link_val
            ),
            field_definition: None,
        });
        return;
    }

    let target_name = match parse_pure_wikilink(trimmed) {
        Some(token) => token.parsed.normalized_target,
        None => {
            issues.push(VerifyIssue {
                level: IssueLevel::Error,
                message: format!(
                    "field '{}' has invalid link format: '{}'. Expected Obsidian wiki-link, e.g. [[note-name]].",
                    field_name, link_val
                ),
                field_definition: None,
            });
            return;
        }
    };

    let notes = match db.get_notes_by_name(&target_name) {
        Ok(n) => n,
        Err(_) => {
            let field_def = all_schema_fields.get(field_name);
            let field_definition = field_def.map(format_field_definition);
            issues.push(VerifyIssue {
                level: IssueLevel::Error,
                message: format!(
                    "field '{}' links to '{}' which is not found in the vault.",
                    field_name, target_name
                ),
                field_definition,
            });
            return;
        }
    };

    if notes.is_empty() {
        issues.push(VerifyIssue {
            level: IssueLevel::Error,
            message: format!(
                "field '{}' links to '{}' which is not found in the vault.",
                field_name, target_name
            ),
            field_definition: all_schema_fields
                .get(field_name)
                .map(format_field_definition),
        });
        return;
    }

    if let Some(expected_type) = target_type {
        let target_type_val = notes[0].properties.get("type");
        let actual_type = target_type_val.and_then(|v| v.as_str()).unwrap_or("");

        if actual_type != expected_type {
            let field_def = all_schema_fields.get(field_name);
            let field_definition = field_def.map(format_field_definition);
            issues.push(VerifyIssue {
                level: IssueLevel::Error,
                message: format!(
                    "field '{}' links to '{}' (type: '{}'), but template '{}' requires target type '{}'.",
                    field_name, target_name, actual_type, template_name, expected_type
                ),
                field_definition,
            });
        }
    }
}

fn parse_pure_wikilink(input: &str) -> Option<LinkToken> {
    let trimmed = input.trim();
    let leading_ws = input.len().saturating_sub(input.trim_start().len());
    let trailing_ws = input.len().saturating_sub(input.trim_end().len());
    let expected_start = leading_ws;
    let expected_end = input.len().saturating_sub(trailing_ws);

    let mut tokens = scan_link_tokens(input, ScanContext::FrontmatterString).into_iter();
    let token = tokens.next()?;
    if tokens.next().is_some() {
        return None;
    }
    if token.kind != LinkKind::WikiLink {
        return None;
    }
    if token.full_span.start != expected_start || token.full_span.end != expected_end {
        return None;
    }
    if trimmed.is_empty() {
        return None;
    }
    Some(token)
}

fn verify_embedded_bases(db: &Database, embeds: &[String], issues: &mut Vec<VerifyIssue>) {
    for embed in embeds {
        if !embed.ends_with(".base") {
            continue;
        }

        match db.get_notes_by_name(embed) {
            Ok(notes) if notes.is_empty() => {
                issues.push(VerifyIssue {
                    level: IssueLevel::Error,
                    message: format!("embedded base file '{}' is not found in the vault", embed),
                    field_definition: None,
                });
            }
            Err(e) => {
                issues.push(VerifyIssue {
                    level: IssueLevel::Error,
                    message: format!("failed to check embedded base file '{}': {}", embed, e),
                    field_definition: None,
                });
            }
            _ => {}
        }
    }
}

fn verify_template_embedded_bases(
    template_name: &str,
    note_embeds: &HashSet<String>,
    template_embeds: &[String],
    issues: &mut Vec<VerifyIssue>,
) {
    for embed in template_embeds {
        if !embed.ends_with(".base") {
            continue;
        }

        if !note_embeds.contains(embed) {
            issues.push(VerifyIssue {
                level: IssueLevel::Error,
                message: format!(
                    "note is missing embedded base file '![[{}]]' required by template '{}'.",
                    embed, template_name
                ),
                field_definition: None,
            });
        }
    }
}
