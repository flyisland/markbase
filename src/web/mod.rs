use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::db::{Database, Note};
use crate::link_syntax::{LinkKind, LinkToken, ScanContext, scan_link_tokens};
use crate::renderer::{RenderFormat, RenderMode, RenderOptions, render_note_to_string};
use crate::scanner::{self, IndexOptions};
use crate::template::TemplateDocument;
use serde_json::{Map as JsonMap, Value, json};

pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 3000;
pub const DOCSIFY_INDEX_FILENAME: &str = "index.html";
pub const DEFAULT_CACHE_CONTROL: &str = "no-store, no-cache, must-revalidate";
const DOCSIFY_INDEX_TEMPLATE: &str = include_str!("templates/docsify_index.html");
const DOCSIFY_SHELL_STYLE: &str = include_str!("templates/docsify_shell.css");
const DOCSIFY_SHELL_SCRIPT: &str = include_str!("templates/docsify_shell.js");
const MARKBASE_BUILD_VERSION: &str = env!("MARKBASE_BUILD_VERSION");
const MARKBASE_GIT_COMMIT: &str = env!("MARKBASE_GIT_COMMIT");
const MARKBASE_GIT_COMMIT_TIME: &str = env!("MARKBASE_GIT_COMMIT_TIME");
const DOCSIFY_SHELL_VERSION_MARKER_PREFIX: &str = "<!-- markbase-shell-version: ";
const DOCSIFY_SHELL_VERSION_MARKER_SUFFIX: &str = " -->";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedNoteRoute {
    pub file_path: String,
    pub render_target_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedResourceRoute {
    pub file_path: String,
    pub file_name: String,
    pub absolute_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteTarget {
    Note(ResolvedNoteRoute),
    Resource(ResolvedResourceRoute),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebError {
    BadPath(String),
    NotFound(String),
    BinaryResource(String),
    Internal(String),
    Io(String),
}

impl Display for WebError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WebError::BadPath(message)
            | WebError::NotFound(message)
            | WebError::BinaryResource(message)
            | WebError::Internal(message)
            | WebError::Io(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for WebError {}

impl From<std::io::Error> for WebError {
    fn from(err: std::io::Error) -> Self {
        WebError::Io(err.to_string())
    }
}

enum WebResponse {
    EntryHtml(String),
    Markdown(String),
    Json(String),
    Resource {
        body: Vec<u8>,
        content_type: &'static str,
    },
}

enum DocsifyEntryHtmlMode {
    Exported,
    Dynamic {
        ignored_exported_entry_html: bool,
        homepage_canonical_url: String,
    },
}

struct SelectedDocsifyEntryHtml {
    html: String,
    mode: DocsifyEntryHtmlMode,
}

struct ExportedDocsifyEntryHtml {
    html: String,
    version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetadataField {
    Properties,
    Links,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RequestMode {
    Markdown,
    Metadata(Vec<MetadataField>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedRequestTarget {
    canonical_path: String,
    mode: RequestMode,
}

#[derive(Debug, Clone)]
struct PropertySchemaInfo {
    template_name: String,
    required: bool,
    field_type: Option<String>,
    format: Option<String>,
    target: Option<String>,
    enum_values: Option<Vec<String>>,
    description: Option<String>,
}

pub fn get(
    base_dir: &Path,
    db_path: &Path,
    compute_backlinks: bool,
    canonical_url: &str,
) -> Result<String, WebError> {
    match render_request(base_dir, db_path, compute_backlinks, canonical_url, None)? {
        WebResponse::EntryHtml(_) => Err(WebError::BinaryResource(
            "ERROR: `markbase web get` does not emit docsify entry HTML.".to_string(),
        )),
        WebResponse::Markdown(body) | WebResponse::Json(body) => Ok(body),
        WebResponse::Resource { .. } => Err(WebError::BinaryResource(format!(
            "ERROR: canonical URL '{}' resolves to a binary resource; `markbase web get` does not stream resource bytes.",
            canonical_url
        ))),
    }
}

pub fn serve(
    base_dir: &Path,
    db_path: &Path,
    compute_backlinks: bool,
    bind: &str,
    port: u16,
    homepage: Option<&str>,
    cache_control: Option<&str>,
) -> Result<(), WebError> {
    let entry_html = select_docsify_entry_html(base_dir, db_path, compute_backlinks, homepage)?;
    log_docsify_entry_html_mode(base_dir, &entry_html.mode);
    let cache_control = cache_control.unwrap_or(DEFAULT_CACHE_CONTROL);
    let listener = TcpListener::bind((bind, port))
        .map_err(|err| WebError::Io(format!("failed to bind {}:{}: {}", bind, port, err)))?;
    let local_addr = listener
        .local_addr()
        .map_err(|err| WebError::Io(format!("failed to read local address: {}", err)))?;
    eprintln!("Serving markbase web on http://{}", local_addr);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(err) = handle_connection(
                    stream,
                    base_dir,
                    db_path,
                    compute_backlinks,
                    &entry_html.html,
                    cache_control,
                ) {
                    eprintln!("WARN: web request failed: {}", err);
                }
            }
            Err(err) => {
                eprintln!("WARN: failed to accept connection: {}", err);
            }
        }
    }

    Ok(())
}

pub fn init_docsify(
    base_dir: &Path,
    db_path: &Path,
    compute_backlinks: bool,
    homepage: &str,
    force: bool,
) -> Result<PathBuf, WebError> {
    let homepage = resolve_homepage_reference(base_dir, db_path, compute_backlinks, homepage)?;

    let index_path = docsify_index_path(base_dir);
    if index_path.exists() && !force {
        return Err(WebError::Io(format!(
            "ERROR: docsify entry HTML already exists at '{}'. Re-run with --force to overwrite it.",
            index_path.display()
        )));
    }

    let shell = render_docsify_index(&homepage);
    fs::write(&index_path, shell).map_err(|err| {
        WebError::Io(format!(
            "failed to write docsify entry HTML '{}': {}",
            index_path.display(),
            err
        ))
    })?;

    Ok(index_path)
}

fn select_docsify_entry_html(
    base_dir: &Path,
    db_path: &Path,
    compute_backlinks: bool,
    homepage_override: Option<&str>,
) -> Result<SelectedDocsifyEntryHtml, WebError> {
    if let Some(homepage) = homepage_override {
        let homepage_canonical =
            resolve_homepage_reference(base_dir, db_path, compute_backlinks, homepage)?;
        return Ok(SelectedDocsifyEntryHtml {
            html: render_docsify_index(&homepage_canonical),
            mode: DocsifyEntryHtmlMode::Dynamic {
                ignored_exported_entry_html: docsify_index_path(base_dir).is_file(),
                homepage_canonical_url: homepage_canonical,
            },
        });
    }

    let exported = read_exported_docsify_entry_html(base_dir)?;
    match exported {
        Some(exported) if exported.version.as_deref() == Some(MARKBASE_BUILD_VERSION) => {
            return Ok(SelectedDocsifyEntryHtml {
                html: exported.html,
                mode: DocsifyEntryHtmlMode::Exported,
            });
        }
        Some(exported) => {
            let index_path = docsify_index_path(base_dir);
            let exported_version = exported
                .version
                .as_deref()
                .unwrap_or("missing markbase version metadata");
            return Err(WebError::Io(format!(
                "ERROR: `markbase web serve` was started without `--homepage`, so it can only reuse the exported docsify entry HTML '{}'. That file is not usable because its embedded markbase version is '{}', not '{}'. Re-run with `--homepage <homepage-ref>` for dynamic mode or refresh the exported file with `markbase web init-docsify --homepage <homepage-ref> --force`.",
                index_path.display(),
                exported_version,
                MARKBASE_BUILD_VERSION
            )));
        }
        None => {}
    }

    Err(WebError::Io(format!(
        "ERROR: `markbase web serve` was started without `--homepage`, so it can only reuse the exported docsify entry HTML '{}'. That file does not exist. Pass `--homepage <homepage-ref>` for dynamic mode or run `markbase web init-docsify --homepage <homepage-ref>` first.",
        docsify_index_path(base_dir).display()
    )))
}

fn read_exported_docsify_entry_html(
    base_dir: &Path,
) -> Result<Option<ExportedDocsifyEntryHtml>, WebError> {
    let index_path = docsify_index_path(base_dir);
    if !index_path.is_file() {
        return Ok(None);
    }

    let html = fs::read_to_string(&index_path).map_err(|err| {
        WebError::Io(format!(
            "failed to read docsify entry HTML '{}': {}",
            index_path.display(),
            err
        ))
    })?;

    Ok(Some(ExportedDocsifyEntryHtml {
        version: read_docsify_shell_version(&html).map(str::to_owned),
        html,
    }))
}

fn log_docsify_entry_html_mode(base_dir: &Path, mode: &DocsifyEntryHtmlMode) {
    let index_path = docsify_index_path(base_dir);
    match mode {
        DocsifyEntryHtmlMode::Exported => eprintln!(
            "INFO: using exported docsify entry HTML '{}'.",
            index_path.display()
        ),
        DocsifyEntryHtmlMode::Dynamic {
            ignored_exported_entry_html,
            homepage_canonical_url,
        } => {
            eprintln!(
                "INFO: serving dynamic docsify entry HTML for homepage '{}'.",
                homepage_canonical_url
            );
            if *ignored_exported_entry_html {
                eprintln!(
                    "WARN: exported docsify entry HTML '{}' exists but will not be used because `--homepage` requested dynamic mode.",
                    index_path.display()
                );
            }
        }
    }
}

pub fn with_request_context<T, F>(
    base_dir: &Path,
    db_path: &Path,
    compute_backlinks: bool,
    raw_path: &str,
    f: F,
) -> Result<T, WebError>
where
    F: FnOnce(&Database, RouteTarget) -> Result<T, WebError>,
{
    let db = Database::new(db_path)
        .map_err(|err| WebError::Internal(format!("failed to open database: {}", err)))?;
    scanner::index_directory_with_options(base_dir, &db, false, IndexOptions { compute_backlinks })
        .map_err(|err| WebError::Internal(format!("failed to refresh index: {}", err)))?;

    let decoded_path = decode_canonical_path(raw_path)?;
    let target = resolve_decoded_path(base_dir, &db, &decoded_path)?;
    f(&db, target)
}

fn with_indexed_database<T, F>(
    base_dir: &Path,
    db_path: &Path,
    compute_backlinks: bool,
    f: F,
) -> Result<T, WebError>
where
    F: FnOnce(&Database) -> Result<T, WebError>,
{
    let db = Database::new(db_path)
        .map_err(|err| WebError::Internal(format!("failed to open database: {}", err)))?;
    scanner::index_directory_with_options(base_dir, &db, false, IndexOptions { compute_backlinks })
        .map_err(|err| WebError::Internal(format!("failed to refresh index: {}", err)))?;
    f(&db)
}

fn resolve_homepage_reference(
    base_dir: &Path,
    db_path: &Path,
    compute_backlinks: bool,
    homepage_ref: &str,
) -> Result<String, WebError> {
    with_indexed_database(base_dir, db_path, compute_backlinks, |db| {
        let note = if homepage_ref.starts_with('/') {
            let decoded = decode_canonical_path(homepage_ref)?;
            db.get_note_by_path(&decoded)
                .map_err(|err| WebError::Internal(format!("failed to resolve route: {}", err)))?
                .ok_or_else(|| {
                    WebError::NotFound(format!(
                        "ERROR: homepage '{}' does not resolve to an indexed document.",
                        homepage_ref
                    ))
                })?
        } else if let Some(note) = db
            .get_note_by_path(homepage_ref)
            .map_err(|err| WebError::Internal(format!("failed to resolve route: {}", err)))?
        {
            note
        } else {
            resolve_homepage_note_name(db, homepage_ref)?
        };

        homepage_canonical_url_for_note(&note, homepage_ref)
    })
}

fn resolve_homepage_note_name(db: &Database, homepage_ref: &str) -> Result<Note, WebError> {
    let matches = db.get_notes_by_name(homepage_ref).map_err(|err| {
        WebError::Internal(format!("failed to resolve '{}': {}", homepage_ref, err))
    })?;

    if matches.is_empty() {
        return Err(WebError::NotFound(format!(
            "ERROR: homepage '{}' does not resolve to an indexed document.",
            homepage_ref
        )));
    }

    if matches.len() > 1 {
        return Err(WebError::Internal(format!(
            "multiple indexed entries found for '{}'",
            homepage_ref
        )));
    }

    Ok(matches.into_iter().next().expect("matches is not empty"))
}

fn homepage_canonical_url_for_note(note: &Note, homepage_ref: &str) -> Result<String, WebError> {
    if note.ext == "md" || note.ext == "base" {
        return Ok(encode_canonical_path(&note.path));
    }

    Err(WebError::BinaryResource(format!(
        "ERROR: homepage '{}' resolves to '{}', which is a binary resource. Homepage only supports `.md` and `.base` targets.",
        homepage_ref, note.path
    )))
}

pub fn decode_canonical_path(raw_path: &str) -> Result<String, WebError> {
    let path_only = raw_path
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(raw_path);

    if !path_only.starts_with('/') {
        return Err(WebError::BadPath(format!(
            "ERROR: canonical URL '{}' must start with '/'.",
            raw_path
        )));
    }

    let mut bytes = Vec::with_capacity(path_only.len());
    let input = path_only.as_bytes();
    let mut idx = 1;

    while idx < input.len() {
        match input[idx] {
            b'%' => {
                if idx + 2 >= input.len() {
                    return Err(WebError::BadPath(format!(
                        "ERROR: canonical URL '{}' contains an incomplete percent-encoding.",
                        raw_path
                    )));
                }
                let hi = decode_hex(input[idx + 1]).ok_or_else(|| {
                    WebError::BadPath(format!(
                        "ERROR: canonical URL '{}' contains an invalid percent-encoding.",
                        raw_path
                    ))
                })?;
                let lo = decode_hex(input[idx + 2]).ok_or_else(|| {
                    WebError::BadPath(format!(
                        "ERROR: canonical URL '{}' contains an invalid percent-encoding.",
                        raw_path
                    ))
                })?;
                bytes.push((hi << 4) | lo);
                idx += 3;
            }
            byte => {
                bytes.push(byte);
                idx += 1;
            }
        }
    }

    String::from_utf8(bytes).map_err(|_| {
        WebError::BadPath(format!(
            "ERROR: canonical URL '{}' is not valid UTF-8 after decoding.",
            raw_path
        ))
    })
}

pub fn encode_canonical_path(file_path: &str) -> String {
    let mut out = String::with_capacity(file_path.len() + 1);
    out.push('/');

    for byte in file_path.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(*byte as char)
            }
            _ => {
                out.push('%');
                out.push(to_hex(byte >> 4));
                out.push(to_hex(byte & 0x0f));
            }
        }
    }

    out
}

fn render_request(
    base_dir: &Path,
    db_path: &Path,
    compute_backlinks: bool,
    raw_path: &str,
    docsify_entry_html: Option<&str>,
) -> Result<WebResponse, WebError> {
    if let Some(entry_html) = docsify_entry_html
        && is_docsify_entry_html_route(raw_path)
    {
        return Ok(WebResponse::EntryHtml(entry_html.to_string()));
    }

    let request = parse_request_target(raw_path)?;
    with_request_context(
        base_dir,
        db_path,
        compute_backlinks,
        &request.canonical_path,
        |db, target| match (&request.mode, target) {
            (RequestMode::Markdown, RouteTarget::Note(note)) => {
                let rendered = render_note_to_string(
                    base_dir,
                    db,
                    &note.render_target_name,
                    &RenderOptions {
                        format: RenderFormat::Table,
                        dry_run: false,
                        mode: RenderMode::Web,
                    },
                )
                .map_err(|err| WebError::Internal(err.to_string()))?;
                let normalized = normalize_markdown_for_web(&rendered, db)?;
                Ok(WebResponse::Markdown(normalized))
            }
            (RequestMode::Markdown, RouteTarget::Resource(resource)) => {
                let body = fs::read(&resource.absolute_path).map_err(|err| {
                    WebError::Io(format!(
                        "failed to read resource '{}': {}",
                        resource.file_path, err
                    ))
                })?;
                Ok(WebResponse::Resource {
                    body,
                    content_type: content_type_for_path(&resource.file_path),
                })
            }
            (RequestMode::Metadata(fields), RouteTarget::Note(note)) => {
                let note_row = db
                    .get_note_by_path(&note.file_path)
                    .map_err(|err| {
                        WebError::Internal(format!(
                            "failed to load note metadata for '{}': {}",
                            note.file_path, err
                        ))
                    })?
                    .ok_or_else(|| {
                        WebError::NotFound(format!(
                            "ERROR: canonical URL '/{}' was not found in the indexed vault.",
                            note.file_path
                        ))
                    })?;

                if note_row.ext != "md" {
                    return Err(WebError::BadPath(format!(
                        "ERROR: metadata mode only supports canonical Markdown note routes; '/{}' resolves to '.{}'.",
                        note_row.path, note_row.ext
                    )));
                }

                let metadata = render_note_metadata(base_dir, db, &note_row, fields)?;
                Ok(WebResponse::Json(metadata))
            }
            (RequestMode::Metadata(_), RouteTarget::Resource(resource)) => {
                Err(WebError::BadPath(format!(
                    "ERROR: metadata mode only supports canonical Markdown note routes; '/{}' resolves to a binary resource.",
                    resource.file_path
                )))
            }
        },
    )
}

fn is_docsify_entry_html_route(raw_path: &str) -> bool {
    let path_only = raw_path
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(raw_path);
    path_only == "/" || path_only == format!("/{}", DOCSIFY_INDEX_FILENAME)
}

fn parse_request_target(raw_path: &str) -> Result<ParsedRequestTarget, WebError> {
    let Some((path, query)) = raw_path.split_once('?') else {
        return Ok(ParsedRequestTarget {
            canonical_path: raw_path.to_string(),
            mode: RequestMode::Markdown,
        });
    };

    if query.is_empty() {
        return Err(WebError::BadPath(format!(
            "ERROR: canonical URL '{}' contains an empty query string.",
            raw_path
        )));
    }

    let fields = parse_query_fields(raw_path, query)?;
    Ok(ParsedRequestTarget {
        canonical_path: path.to_string(),
        mode: RequestMode::Metadata(fields),
    })
}

fn parse_query_fields(raw_path: &str, query: &str) -> Result<Vec<MetadataField>, WebError> {
    let mut fields_value = None;

    for segment in query.split('&') {
        if segment.is_empty() {
            return Err(WebError::BadPath(format!(
                "ERROR: canonical URL '{}' contains an empty query parameter.",
                raw_path
            )));
        }

        let Some((key, value)) = segment.split_once('=') else {
            return Err(WebError::BadPath(format!(
                "ERROR: canonical URL '{}' contains an unsupported query parameter '{}'.",
                raw_path, segment
            )));
        };

        if key != "fields" {
            return Err(WebError::BadPath(format!(
                "ERROR: canonical URL '{}' contains an unsupported query parameter '{}'.",
                raw_path, key
            )));
        }

        if fields_value.is_some() {
            return Err(WebError::BadPath(format!(
                "ERROR: canonical URL '{}' repeats the 'fields' query parameter.",
                raw_path
            )));
        }

        fields_value = Some(value);
    }

    let Some(fields_value) = fields_value else {
        return Err(WebError::BadPath(format!(
            "ERROR: canonical URL '{}' is missing the required 'fields' query parameter value.",
            raw_path
        )));
    };

    parse_fields_value(raw_path, fields_value)
}

fn parse_fields_value(raw_path: &str, value: &str) -> Result<Vec<MetadataField>, WebError> {
    if value.is_empty() {
        return Err(WebError::BadPath(format!(
            "ERROR: canonical URL '{}' contains an empty 'fields' query parameter.",
            raw_path
        )));
    }

    let mut fields = Vec::new();
    for token in value.split(',') {
        if token.is_empty() || token.trim() != token {
            return Err(WebError::BadPath(format!(
                "ERROR: canonical URL '{}' contains malformed 'fields' syntax.",
                raw_path
            )));
        }

        let field = match token {
            "properties" => MetadataField::Properties,
            "links" => MetadataField::Links,
            _ => {
                return Err(WebError::BadPath(format!(
                    "ERROR: canonical URL '{}' requests unsupported metadata field '{}'.",
                    raw_path, token
                )));
            }
        };

        if !fields.contains(&field) {
            fields.push(field);
        }
    }

    Ok(fields)
}

fn resolve_decoded_path(
    base_dir: &Path,
    db: &Database,
    decoded_path: &str,
) -> Result<RouteTarget, WebError> {
    let decoded_path = if decoded_path.is_empty() {
        DOCSIFY_INDEX_FILENAME
    } else {
        decoded_path
    };

    let note = db
        .get_note_by_path(decoded_path)
        .map_err(|err| WebError::Internal(format!("failed to resolve route: {}", err)))?
        .ok_or_else(|| {
            WebError::NotFound(format!(
                "ERROR: canonical URL '/{}' was not found in the indexed vault.",
                decoded_path
            ))
        })?;

    if note.ext == "md" || note.ext == "base" {
        return Ok(RouteTarget::Note(ResolvedNoteRoute {
            file_path: note.path,
            render_target_name: note.name,
        }));
    }

    Ok(RouteTarget::Resource(ResolvedResourceRoute {
        file_name: note.name.clone(),
        file_path: note.path.clone(),
        absolute_path: base_dir.join(note.path),
    }))
}

fn normalize_markdown_for_web(markdown: &str, db: &Database) -> Result<String, WebError> {
    transform_markdown_preserving_code(markdown, |body| {
        let stripped = strip_comments(body);
        rewrite_body_links_and_embeds(&stripped, db)
    })
}

fn rewrite_body_links_and_embeds(body: &str, db: &Database) -> Result<String, WebError> {
    let tokens = scan_link_tokens(body, ScanContext::MarkdownBody);
    let mut output = String::with_capacity(body.len());
    let mut cursor = 0;

    for token in tokens {
        let Some(replacement) = replacement_for_token(&token, db)? else {
            continue;
        };

        output.push_str(&body[cursor..token.full_span.start]);
        output.push_str(&replacement);
        cursor = token.full_span.end;
    }

    output.push_str(&body[cursor..]);
    Ok(output)
}

fn replacement_for_token(token: &LinkToken, db: &Database) -> Result<Option<String>, WebError> {
    match token.kind {
        LinkKind::WikiLink => rewrite_wikilink(token, db).map(Some),
        LinkKind::Embed => rewrite_resource_embed(token, db),
    }
}

fn rewrite_wikilink(token: &LinkToken, db: &Database) -> Result<String, WebError> {
    let Some(note) = lookup_unique_name(db, &token.parsed.normalized_target)? else {
        return Ok(token_text(token));
    };
    if note.ext != "md" && note.ext != "base" {
        return Ok(token_text(token));
    }

    let href = encode_canonical_path(&note.path);
    let text = if let Some(alias) = token.parsed.alias_or_size.as_deref() {
        alias.to_string()
    } else if let Some(anchor) = token.parsed.anchor.as_deref() {
        if anchor.starts_with('^') {
            token.parsed.normalized_target.clone()
        } else {
            format!("{} > {}", token.parsed.normalized_target, anchor)
        }
    } else {
        token.parsed.normalized_target.clone()
    };

    Ok(format!("[{}]({})", text, href))
}

fn rewrite_resource_embed(token: &LinkToken, db: &Database) -> Result<Option<String>, WebError> {
    if token.parsed.is_markdown_note || token.parsed.normalized_target.ends_with(".base") {
        return Ok(None);
    }

    let Some(resource) = lookup_unique_name(db, &token.parsed.normalized_target)? else {
        return Ok(None);
    };
    if resource.ext == "md" || resource.ext == "base" {
        return Ok(None);
    }

    let href = encode_canonical_path(&resource.path);
    if is_image_extension(&resource.ext) {
        return Ok(Some(format!("![]({})", href)));
    }

    Ok(Some(format!("[{}]({})", resource.name, href)))
}

fn lookup_unique_name(db: &Database, name: &str) -> Result<Option<Note>, WebError> {
    let matches = db
        .get_notes_by_name(name)
        .map_err(|err| WebError::Internal(format!("failed to resolve '{}': {}", name, err)))?;

    if matches.is_empty() {
        return Ok(None);
    }

    if matches.len() > 1 {
        return Err(WebError::Internal(format!(
            "multiple indexed entries found for '{}'",
            name
        )));
    }

    Ok(matches.into_iter().next())
}

fn render_note_metadata(
    base_dir: &Path,
    db: &Database,
    note: &Note,
    fields: &[MetadataField],
) -> Result<String, WebError> {
    let template_names = parse_note_template_names(&note.properties);
    let property_schemas = collect_property_schemas(base_dir, &template_names);
    let property_order = load_note_frontmatter_order(base_dir, note).unwrap_or_default();

    let mut response = JsonMap::new();
    response.insert(
        "file".to_string(),
        build_file_metadata(note, &template_names),
    );

    for field in fields {
        match field {
            MetadataField::Properties => {
                response.insert(
                    "properties".to_string(),
                    build_properties_metadata(db, note, &property_schemas, &property_order)?,
                );
            }
            MetadataField::Links => {
                response.insert("links".to_string(), build_links_metadata(db, note)?);
            }
        }
    }

    serde_json::to_string(&Value::Object(response))
        .map_err(|err| WebError::Internal(format!("failed to serialize note metadata: {}", err)))
}

fn build_file_metadata(note: &Note, template_names: &[String]) -> Value {
    json!({
        "path": note.path,
        "name": note.name,
        "folder": note.folder,
        "templates": template_names,
    })
}

fn build_properties_metadata(
    db: &Database,
    note: &Note,
    property_schemas: &std::collections::HashMap<String, PropertySchemaInfo>,
    property_order: &[String],
) -> Result<Value, WebError> {
    let mut fields = Vec::new();

    if let Some(properties) = note.properties.as_object() {
        for key in property_order {
            if let Some(value) = properties.get(key) {
                fields.push(build_property_field(
                    db,
                    key,
                    value,
                    property_schemas.get(key),
                )?);
            }
        }

        let mut remaining_keys: Vec<&String> = properties
            .keys()
            .filter(|key| !property_order.contains(*key))
            .collect();
        remaining_keys.sort();

        for key in remaining_keys {
            if let Some(value) = properties.get(key) {
                fields.push(build_property_field(
                    db,
                    key,
                    value,
                    property_schemas.get(key),
                )?);
            }
        }
    }

    Ok(json!({ "fields": fields }))
}

fn build_property_field(
    db: &Database,
    key: &str,
    raw_value: &Value,
    schema: Option<&PropertySchemaInfo>,
) -> Result<Value, WebError> {
    let mut field = JsonMap::new();
    field.insert("key".to_string(), Value::String(key.to_string()));
    field.insert("raw".to_string(), raw_value.clone());
    field.insert("value".to_string(), semantic_value_node(db, raw_value)?);
    if let Some(schema) = schema {
        field.insert("schema".to_string(), property_schema_value(schema));
    }
    Ok(Value::Object(field))
}

fn semantic_value_node(db: &Database, value: &Value) -> Result<Value, WebError> {
    match value {
        Value::Null => Ok(json!({ "kind": "null" })),
        Value::Bool(_) | Value::Number(_) => Ok(json!({ "kind": "scalar", "value": value })),
        Value::String(text) => rich_text_value(db, text),
        Value::Array(items) => Ok(json!({
            "kind": "list",
            "items": items
                .iter()
                .map(|item| semantic_value_node(db, item))
                .collect::<Result<Vec<_>, _>>()?,
        })),
        Value::Object(object) => {
            let fields = object
                .iter()
                .map(|(key, nested)| {
                    Ok(json!({
                        "key": key,
                        "value": semantic_value_node(db, nested)?,
                    }))
                })
                .collect::<Result<Vec<_>, WebError>>()?;
            Ok(json!({
                "kind": "object",
                "fields": fields,
            }))
        }
    }
}

fn rich_text_value(db: &Database, input: &str) -> Result<Value, WebError> {
    let tokens = scan_link_tokens(input, ScanContext::FrontmatterString);
    let mut segments = Vec::new();
    let mut cursor = 0;

    for token in tokens
        .into_iter()
        .filter(|token| token.kind == LinkKind::WikiLink)
    {
        if cursor < token.full_span.start {
            segments.push(json!({
                "type": "text",
                "text": &input[cursor..token.full_span.start],
            }));
        }

        segments.push(frontmatter_wikilink_segment(db, &token)?);
        cursor = token.full_span.end;
    }

    if cursor < input.len() || segments.is_empty() {
        segments.push(json!({
            "type": "text",
            "text": &input[cursor..],
        }));
    }

    Ok(json!({
        "kind": "rich_text",
        "segments": segments,
    }))
}

fn frontmatter_wikilink_segment(db: &Database, token: &LinkToken) -> Result<Value, WebError> {
    let mut segment = JsonMap::new();
    segment.insert("type".to_string(), Value::String("wikilink".to_string()));
    segment.insert(
        "target".to_string(),
        Value::String(token.parsed.normalized_target.clone()),
    );
    segment.insert(
        "text".to_string(),
        Value::String(wikilink_display_text(token)),
    );

    if let Some(target) = lookup_unique_name(db, &token.parsed.normalized_target)? {
        segment.insert("exists".to_string(), Value::Bool(true));
        segment.insert(
            "href".to_string(),
            Value::String(encode_canonical_path(&target.path)),
        );
    } else {
        segment.insert("exists".to_string(), Value::Bool(false));
    }

    Ok(Value::Object(segment))
}

fn wikilink_display_text(token: &LinkToken) -> String {
    if let Some(alias) = token.parsed.alias_or_size.as_deref() {
        alias.to_string()
    } else if let Some(anchor) = token.parsed.anchor.as_deref() {
        if anchor.starts_with('^') {
            token.parsed.normalized_target.clone()
        } else {
            format!("{} > {}", token.parsed.normalized_target, anchor)
        }
    } else {
        token.parsed.normalized_target.clone()
    }
}

fn build_links_metadata(db: &Database, note: &Note) -> Result<Value, WebError> {
    let links = note
        .links
        .iter()
        .map(|target| build_link_entry(db, target))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Value::Array(links))
}

fn build_link_entry(db: &Database, target: &str) -> Result<Value, WebError> {
    let mut entry = JsonMap::new();
    entry.insert("target".to_string(), Value::String(target.to_string()));

    if let Some(resolved) = lookup_unique_name(db, target)? {
        entry.insert(
            "kind".to_string(),
            Value::String(resolved_link_kind(&resolved).to_string()),
        );
        entry.insert("exists".to_string(), Value::Bool(true));
        entry.insert(
            "href".to_string(),
            Value::String(encode_canonical_path(&resolved.path)),
        );
    } else {
        entry.insert(
            "kind".to_string(),
            Value::String(unresolved_link_kind(target).to_string()),
        );
        entry.insert("exists".to_string(), Value::Bool(false));
    }

    Ok(Value::Object(entry))
}

fn resolved_link_kind(note: &Note) -> &'static str {
    match note.ext.as_str() {
        "md" => "note",
        "base" => "base",
        _ => "resource",
    }
}

fn unresolved_link_kind(target: &str) -> &'static str {
    if target.ends_with(".base") {
        "base"
    } else if target.contains('.') {
        "resource"
    } else {
        "note"
    }
}

fn parse_note_template_names(properties: &Value) -> Vec<String> {
    properties
        .get("templates")
        .and_then(Value::as_array)
        .map(|templates| {
            templates
                .iter()
                .filter_map(Value::as_str)
                .filter_map(parse_pure_wikilink_name)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_pure_wikilink_name(input: &str) -> Option<String> {
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

    Some(token.parsed.normalized_target)
}

fn collect_property_schemas(
    base_dir: &Path,
    template_names: &[String],
) -> std::collections::HashMap<String, PropertySchemaInfo> {
    let mut schemas = std::collections::HashMap::new();

    for template_name in template_names {
        let Ok(template) = TemplateDocument::load(base_dir, template_name) else {
            continue;
        };

        let required_fields: std::collections::HashSet<String> =
            template.required_fields().into_iter().collect();
        let property_definitions = template.schema_properties();

        let mut field_names = Vec::new();
        if let Some(property_definitions) = property_definitions {
            field_names.extend(property_definitions.keys().cloned());
        }
        for required in &required_fields {
            if !field_names.contains(required) {
                field_names.push(required.clone());
            }
        }

        for field_name in field_names {
            if schemas.contains_key(&field_name) {
                continue;
            }

            let is_required = required_fields.contains(&field_name);
            let definition = property_definitions.and_then(|props| props.get(&field_name));
            let field_type = definition
                .and_then(Value::as_object)
                .and_then(|field| field.get("type"))
                .and_then(Value::as_str)
                .map(String::from);
            let format = definition
                .and_then(Value::as_object)
                .and_then(|field| field.get("format"))
                .and_then(Value::as_str)
                .map(String::from);
            let target = definition
                .and_then(Value::as_object)
                .and_then(|field| field.get("target"))
                .and_then(Value::as_str)
                .map(String::from);
            let enum_values = definition
                .and_then(Value::as_object)
                .and_then(|field| field.get("enum"))
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect::<Vec<_>>()
                });
            let description = definition
                .and_then(Value::as_object)
                .and_then(|field| field.get("description"))
                .and_then(Value::as_str)
                .map(String::from);

            schemas.insert(
                field_name,
                PropertySchemaInfo {
                    template_name: template.name().unwrap_or(template_name).to_string(),
                    required: is_required,
                    field_type,
                    format,
                    target,
                    enum_values,
                    description,
                },
            );
        }
    }

    schemas
}

fn load_note_frontmatter_order(base_dir: &Path, note: &Note) -> Result<Vec<String>, WebError> {
    let note_path = base_dir.join(&note.path);
    let content = fs::read_to_string(&note_path).map_err(|err| {
        WebError::Io(format!(
            "failed to read note '{}' for frontmatter order: {}",
            note.path, err
        ))
    })?;

    Ok(extract_top_level_frontmatter_keys(&content))
}

fn extract_top_level_frontmatter_keys(content: &str) -> Vec<String> {
    let mut lines = content.lines();
    let Some(first_line) = lines.next() else {
        return Vec::new();
    };
    if first_line.trim() != "---" {
        return Vec::new();
    }

    let mut keys = Vec::new();
    for line in lines {
        if line.is_empty()
            || line.starts_with(' ')
            || line.starts_with('\t')
            || line.starts_with('#')
            || line.starts_with('-')
        {
            continue;
        }

        if line.trim() == "---" {
            break;
        }

        let Some((key, _)) = line.split_once(':') else {
            continue;
        };

        let key = key.trim();
        if key.is_empty() {
            continue;
        }

        let normalized = key
            .strip_prefix('"')
            .and_then(|key| key.strip_suffix('"'))
            .or_else(|| {
                key.strip_prefix('\'')
                    .and_then(|key| key.strip_suffix('\''))
            })
            .unwrap_or(key);

        if !keys.iter().any(|existing| existing == normalized) {
            keys.push(normalized.to_string());
        }
    }

    keys
}

fn property_schema_value(schema: &PropertySchemaInfo) -> Value {
    let mut value = JsonMap::new();
    value.insert(
        "template".to_string(),
        Value::String(schema.template_name.clone()),
    );
    value.insert("required".to_string(), Value::Bool(schema.required));
    if let Some(field_type) = &schema.field_type {
        value.insert("type".to_string(), Value::String(field_type.clone()));
    }
    if let Some(format) = &schema.format {
        value.insert("format".to_string(), Value::String(format.clone()));
    }
    if let Some(target) = &schema.target {
        value.insert("target".to_string(), Value::String(target.clone()));
    }
    if let Some(enum_values) = &schema.enum_values {
        value.insert(
            "enum".to_string(),
            Value::Array(enum_values.iter().cloned().map(Value::String).collect()),
        );
    }
    if let Some(description) = &schema.description {
        value.insert(
            "description".to_string(),
            Value::String(description.clone()),
        );
    }
    Value::Object(value)
}

fn transform_markdown_preserving_code<F>(
    input: &str,
    mut transform_body: F,
) -> Result<String, WebError>
where
    F: FnMut(&str) -> Result<String, WebError>,
{
    let bytes = input.as_bytes();
    let mut output = String::with_capacity(input.len());
    let mut body_start = 0;
    let mut idx = 0;
    let mut line_start = true;

    while idx < bytes.len() {
        if line_start && let Some((marker, count, after)) = detect_fence_marker(bytes, idx) {
            output.push_str(&transform_body(&input[body_start..idx])?);

            let mut end = skip_to_line_end(bytes, after);
            let mut next_line_start = end;
            while next_line_start < bytes.len() {
                if let Some((found_marker, found_count, found_after)) =
                    detect_fence_marker(bytes, next_line_start)
                    && found_marker == marker
                    && found_count >= count
                {
                    end = skip_to_line_end(bytes, found_after);
                    break;
                }

                while end < bytes.len() && bytes[end] != b'\n' {
                    end += 1;
                }
                if end < bytes.len() {
                    end += 1;
                }
                next_line_start = end;
            }

            output.push_str(&input[idx..end]);
            idx = end;
            body_start = idx;
            line_start = idx == 0 || bytes[idx - 1] == b'\n';
            continue;
        }

        if bytes[idx] == b'`' {
            let tick_count = count_run(bytes, idx, b'`');
            let mut end = idx + tick_count;
            let mut found = None;

            while end < bytes.len() {
                if bytes[end] == b'`' && count_run(bytes, end, b'`') == tick_count {
                    found = Some(end + tick_count);
                    break;
                }
                end += 1;
            }

            if let Some(code_end) = found {
                output.push_str(&transform_body(&input[body_start..idx])?);
                output.push_str(&input[idx..code_end]);
                idx = code_end;
                body_start = idx;
                line_start = false;
                continue;
            }
        }

        line_start = bytes[idx] == b'\n';
        idx += 1;
    }

    output.push_str(&transform_body(&input[body_start..])?);
    Ok(output)
}

fn strip_comments(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;

    while let Some(start_rel) = input[cursor..].find("%%") {
        let start = cursor + start_rel;
        output.push_str(&input[cursor..start]);
        let Some(end_rel) = input[start + 2..].find("%%") else {
            output.push_str(&input[start..]);
            return output;
        };
        cursor = start + 2 + end_rel + 2;
    }

    output.push_str(&input[cursor..]);
    output
}

fn handle_connection(
    mut stream: TcpStream,
    base_dir: &Path,
    db_path: &Path,
    compute_backlinks: bool,
    docsify_entry_html: &str,
    cache_control: &str,
) -> Result<(), WebError> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|err| WebError::Io(format!("failed to set read timeout: {}", err)))?;

    let mut buffer = [0_u8; 8192];
    let read = stream
        .read(&mut buffer)
        .map_err(|err| WebError::Io(format!("failed to read request: {}", err)))?;
    if read == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..read]);
    let mut parts = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or("/");

    if method != "GET" {
        write_response(
            &mut stream,
            405,
            "Method Not Allowed",
            "text/plain; charset=utf-8",
            b"Method Not Allowed",
            cache_control,
        )?;
        eprintln!("ACCESS: {} {} {}", method, path, 405);
        return Ok(());
    }

    let mut respond = |status_code: u16,
                       status_text: &str,
                       content_type: &str,
                       body: &[u8]|
     -> Result<(), WebError> {
        write_response(
            &mut stream,
            status_code,
            status_text,
            content_type,
            body,
            cache_control,
        )?;
        eprintln!("ACCESS: {} {} {}", method, path, status_code);
        Ok(())
    };

    match render_request(
        base_dir,
        db_path,
        compute_backlinks,
        path,
        Some(docsify_entry_html),
    ) {
        Ok(WebResponse::EntryHtml(body)) => {
            respond(200, "OK", "text/html; charset=utf-8", body.as_bytes())
        }
        Ok(WebResponse::Markdown(body)) => {
            respond(200, "OK", "text/markdown; charset=utf-8", body.as_bytes())
        }
        Ok(WebResponse::Json(body)) => respond(
            200,
            "OK",
            "application/json; charset=utf-8",
            body.as_bytes(),
        ),
        Ok(WebResponse::Resource { body, content_type }) => respond(200, "OK", content_type, &body),
        Err(WebError::BadPath(message)) => respond(
            400,
            "Bad Request",
            "text/plain; charset=utf-8",
            message.as_bytes(),
        ),
        Err(WebError::NotFound(message)) => respond(
            404,
            "Not Found",
            "text/plain; charset=utf-8",
            message.as_bytes(),
        ),
        Err(err) => respond(
            500,
            "Internal Server Error",
            "text/plain; charset=utf-8",
            err.to_string().as_bytes(),
        ),
    }
}

fn write_response(
    stream: &mut TcpStream,
    status_code: u16,
    status_text: &str,
    content_type: &str,
    body: &[u8],
    cache_control: &str,
) -> Result<(), WebError> {
    let cache_headers = if cache_control == DEFAULT_CACHE_CONTROL {
        format!(
            "Cache-Control: {}\r\nPragma: no-cache\r\nExpires: 0\r\n",
            cache_control
        )
    } else {
        format!("Cache-Control: {}\r\n", cache_control)
    };
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: {}\r\n{}Connection: close\r\n\r\n",
        status_code,
        status_text,
        body.len(),
        content_type,
        cache_headers
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}

fn content_type_for_path(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match ext.as_str() {
        "apng" => "image/apng",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "pdf" => "application/pdf",
        "json" => "application/json; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "md" => "text/markdown; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn docsify_index_path(base_dir: &Path) -> PathBuf {
    base_dir.join(DOCSIFY_INDEX_FILENAME)
}

fn render_docsify_index(homepage: &str) -> String {
    let homepage_json =
        serde_json::to_string(homepage).expect("serializing docsify homepage should not fail");
    let homepage_html = html_escape(homepage);
    let version_html = html_escape(MARKBASE_BUILD_VERSION);
    let commit_html = html_escape(MARKBASE_GIT_COMMIT);
    let commit_time_html = html_escape(MARKBASE_GIT_COMMIT_TIME);
    DOCSIFY_INDEX_TEMPLATE
        .replace("__MARKBASE_DOCSIFY_STYLE__", DOCSIFY_SHELL_STYLE)
        .replace("__MARKBASE_DOCSIFY_SCRIPT__", DOCSIFY_SHELL_SCRIPT)
        .replace("__MARKBASE_DOCSIFY_HOMEPAGE__", &homepage_json)
        .replace("__MARKBASE_DOCSIFY_HOMEPAGE_JSON__", &homepage_json)
        .replace("__MARKBASE_DOCSIFY_HOMEPAGE_HTML__", &homepage_html)
        .replace("__MARKBASE_BUILD_VERSION__", &version_html)
        .replace("__MARKBASE_GIT_COMMIT__", &commit_html)
        .replace("__MARKBASE_GIT_COMMIT_TIME__", &commit_time_html)
}

fn read_docsify_shell_version(html: &str) -> Option<&str> {
    let start = html.find(DOCSIFY_SHELL_VERSION_MARKER_PREFIX)?;
    let version_start = start + DOCSIFY_SHELL_VERSION_MARKER_PREFIX.len();
    let suffix_start = html[version_start..].find(DOCSIFY_SHELL_VERSION_MARKER_SUFFIX)?;
    Some(&html[version_start..version_start + suffix_start])
}

fn html_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn is_image_extension(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg"
    )
}

fn token_text(token: &LinkToken) -> String {
    match token.kind {
        LinkKind::WikiLink => format!("[[{}]]", token.raw_inner),
        LinkKind::Embed => format!("![[{}]]", token.raw_inner),
    }
}

fn decode_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn to_hex(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'A' + (value - 10)) as char,
        _ => unreachable!(),
    }
}

fn detect_fence_marker(bytes: &[u8], i: usize) -> Option<(u8, usize, usize)> {
    let mut cursor = i;
    while cursor < bytes.len() && bytes[cursor] == b' ' {
        cursor += 1;
    }

    let marker = *bytes.get(cursor)?;
    if marker != b'`' && marker != b'~' {
        return None;
    }

    let count = count_run(bytes, cursor, marker);
    if count < 3 {
        return None;
    }

    Some((marker, count, cursor + count))
}

fn count_run(bytes: &[u8], start: usize, needle: u8) -> usize {
    let mut count = 0;
    while start + count < bytes.len() && bytes[start + count] == needle {
        count += 1;
    }
    count
}

fn skip_to_line_end(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() {
        let byte = bytes[i];
        i += 1;
        if byte == b'\n' {
            break;
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_canonical_path_decodes_percent_encoded_input() {
        let decoded = decode_canonical_path("/entities/person/%E5%BC%A0%E4%B8%89.md").unwrap();
        assert_eq!(decoded, "entities/person/张三.md");
    }

    #[test]
    fn test_decode_canonical_path_rejects_invalid_percent_encoding() {
        let err = decode_canonical_path("/broken/%ZZ.md").unwrap_err();
        assert!(matches!(err, WebError::BadPath(_)));
    }

    #[test]
    fn test_encode_canonical_path_percent_encodes_reserved_chars() {
        let encoded = encode_canonical_path("entities/with space/#hash?.md");
        assert_eq!(encoded, "/entities/with%20space/%23hash%3F.md");
    }

    #[test]
    fn test_strip_comments_removes_balanced_comments() {
        assert_eq!(strip_comments("A %%hidden%% B"), "A  B");
    }

    #[test]
    fn test_render_docsify_index_rewrites_internal_document_links_only() {
        let html = render_docsify_index("/HOME.md");
        assert!(html.contains("homepage: \"/HOME.md\""));
        assert!(html.contains("ext: \"\""));
        assert!(html.contains("externalLinkTarget: \"_self\""));
        assert!(html.contains(&format!(
            "<meta name=\"markbase-version\" content=\"{}\" />",
            MARKBASE_BUILD_VERSION
        )));
        assert!(html.contains("Generated by markbase"));
        assert!(
            html.contains(
                "noCompileLinks: [\"/.*\\\\.md(?:[?#].*)?\", \"/.*\\\\.base(?:[?#].*)?\"]"
            )
        );
        assert!(html.contains("function normalizeDocsifyDom() {"));
        assert!(html.contains("function attachDocsifyFooter() {"));
        assert!(html.contains("document.querySelector(\"section.content\")"));
        assert!(html.contains("new MutationObserver(function () {"));
        assert!(html.contains(
            ".querySelectorAll(\".markdown-section a[href], .sidebar a[href], nav a[href]\")"
        ));
        assert!(html.contains(
            ".querySelectorAll(\".markdown-section img[data-origin], .sidebar img[data-origin]\")"
        ));
        assert!(html.contains("path.endsWith(\".md\") || path.endsWith(\".base\")"));
        assert!(html.contains("a.removeAttribute(\"target\")"));
        assert!(html.contains("img.setAttribute(\"src\", original)"));
        assert!(!html.contains("path.endsWith(\".png\")"));
    }

    #[test]
    fn test_read_docsify_shell_version_extracts_embedded_shell_version_marker() {
        let html = render_docsify_index("/HOME.md");
        assert_eq!(
            read_docsify_shell_version(&html),
            Some(MARKBASE_BUILD_VERSION)
        );
    }

    #[test]
    fn test_render_docsify_index_includes_callout_upgrade_assets() {
        let html = render_docsify_index("/HOME.md");
        assert!(html.contains(".mb-callout {"));
        assert!(html.contains("section.content {"));
        assert!(html.contains("min-height: 100vh;"));
        assert!(html.contains("flex-direction: column;"));
        assert!(html.contains("function upgradeCalloutsDom() {"));
        assert!(html.contains("function parseCalloutMetadata(firstParagraph) {"));
        assert!(html.contains("function defaultTitleForCallout(calloutType) {"));
        assert!(html.contains("function calloutDepth(blockquote) {"));
        assert!(html.contains("const foldMarkerSvg ="));
        assert!(html.contains("document.createElement(metadata.foldable ? \"details\" : \"div\")"));
        assert!(html.contains("document.createElement(metadata.foldable ? \"summary\" : \"div\")"));
    }

    #[test]
    fn test_render_docsify_index_escapes_homepage_for_javascript_string_literal() {
        let html = render_docsify_index("/docs/%22quoted%22.md");
        assert!(html.contains("homepage: \"/docs/%22quoted%22.md\""));

        let html = render_docsify_index("/docs/\"quoted\".md");
        assert!(html.contains("homepage: \"/docs/\\\"quoted\\\".md\""));
    }
}
