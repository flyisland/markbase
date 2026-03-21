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

pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 3000;
pub const DOCSIFY_INDEX_FILENAME: &str = "index.html";
pub const DEFAULT_CACHE_CONTROL: &str = "no-store, no-cache, must-revalidate";

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
    Markdown(String),
    Resource {
        body: Vec<u8>,
        content_type: &'static str,
    },
}

pub fn get(
    base_dir: &Path,
    db_path: &Path,
    compute_backlinks: bool,
    canonical_url: &str,
) -> Result<String, WebError> {
    match render_request(base_dir, db_path, compute_backlinks, canonical_url)? {
        WebResponse::Markdown(body) => Ok(body),
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
    cache_control: Option<&str>,
) -> Result<(), WebError> {
    ensure_docsify_shell_exists(base_dir)?;
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
                if let Err(err) =
                    handle_connection(stream, base_dir, db_path, compute_backlinks, cache_control)
                {
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

pub fn init_docsify(base_dir: &Path, homepage: &str, force: bool) -> Result<PathBuf, WebError> {
    decode_canonical_path(homepage)?;

    let index_path = docsify_index_path(base_dir);
    if index_path.exists() && !force {
        return Err(WebError::Io(format!(
            "ERROR: docsify shell already exists at '{}'. Re-run with --force to overwrite it.",
            index_path.display()
        )));
    }

    let shell = render_docsify_index(homepage);
    fs::write(&index_path, shell).map_err(|err| {
        WebError::Io(format!(
            "failed to write docsify shell '{}': {}",
            index_path.display(),
            err
        ))
    })?;

    Ok(index_path)
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
) -> Result<WebResponse, WebError> {
    with_request_context(
        base_dir,
        db_path,
        compute_backlinks,
        raw_path,
        |db, target| match target {
            RouteTarget::Note(note) => {
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
            RouteTarget::Resource(resource) => {
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
        },
    )
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
        return write_response(
            &mut stream,
            405,
            "Method Not Allowed",
            "text/plain; charset=utf-8",
            b"Method Not Allowed",
            cache_control,
        );
    }

    match render_request(base_dir, db_path, compute_backlinks, path) {
        Ok(WebResponse::Markdown(body)) => write_response(
            &mut stream,
            200,
            "OK",
            "text/markdown; charset=utf-8",
            body.as_bytes(),
            cache_control,
        ),
        Ok(WebResponse::Resource { body, content_type }) => {
            write_response(&mut stream, 200, "OK", content_type, &body, cache_control)
        }
        Err(WebError::BadPath(message)) => write_response(
            &mut stream,
            400,
            "Bad Request",
            "text/plain; charset=utf-8",
            message.as_bytes(),
            cache_control,
        ),
        Err(WebError::NotFound(message)) => write_response(
            &mut stream,
            404,
            "Not Found",
            "text/plain; charset=utf-8",
            message.as_bytes(),
            cache_control,
        ),
        Err(err) => write_response(
            &mut stream,
            500,
            "Internal Server Error",
            "text/plain; charset=utf-8",
            err.to_string().as_bytes(),
            cache_control,
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

fn ensure_docsify_shell_exists(base_dir: &Path) -> Result<(), WebError> {
    let index_path = docsify_index_path(base_dir);
    if index_path.is_file() {
        return Ok(());
    }

    Err(WebError::Io(format!(
        "ERROR: docsify shell not found at '{}'. Run `markbase web init-docsify --homepage <canonical-url>` first.",
        index_path.display()
    )))
}

fn render_docsify_index(homepage: &str) -> String {
    let homepage_json =
        serde_json::to_string(homepage).expect("serializing docsify homepage should not fail");
    format!(
        r##"<!doctype html>
<html>
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width,initial-scale=1" />
    <title>markbase</title>
    <link
      rel="stylesheet"
      href="//cdn.jsdelivr.net/npm/docsify@4/themes/vue.css"
    />
  </head>
  <body>
    <div id="app">Loading...</div>
    <script>
      window.$docsify = {{
        name: "markbase",
        homepage: {homepage_json},
        basePath: "/",
        ext: "",
        externalLinkTarget: "_self",
        noCompileLinks: ["/.*\\.md(?:[?#].*)?", "/.*\\.base(?:[?#].*)?"],
        auto2top: true,
        plugins: [
          function (hook) {{
            function normalizeDocsifyDom() {{
              document
                .querySelectorAll(".markdown-section a[href], .sidebar a[href], nav a[href]")
                .forEach(function (a) {{
                  const href = a.getAttribute("href");
                  if (!href) return;
                  if (!href.startsWith("/")) return;
                  if (href.startsWith("//")) return;
                  if (href.startsWith("/#")) return;

                  const path = href.split("#")[0].split("?")[0];
                  if (!(path.endsWith(".md") || path.endsWith(".base"))) return;

                  a.setAttribute("href", "#" + href);
                  a.removeAttribute("target");
                  a.removeAttribute("rel");
                }});

              document
                .querySelectorAll(".markdown-section img[data-origin], .sidebar img[data-origin]")
                .forEach(function (img) {{
                  const original = img.getAttribute("data-origin");
                  if (!original) return;
                  if (!original.startsWith("/")) return;

                  img.setAttribute("src", original);
                }});
            }}

            if (!window.__markbaseDocsifyObserverInstalled) {{
              window.__markbaseDocsifyObserverInstalled = true;
              const observer = new MutationObserver(function () {{
                normalizeDocsifyDom();
              }});

              observer.observe(document.body, {{
                childList: true,
                subtree: true,
                attributes: true,
                attributeFilter: ["href", "src", "data-origin"],
              }});
            }}

            hook.doneEach(function () {{
              normalizeDocsifyDom();
            }});
          }},
        ],
      }};
    </script>
    <script src="//cdn.jsdelivr.net/npm/docsify@4"></script>
  </body>
</html>
"##
    )
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
        assert!(
            html.contains(
                "noCompileLinks: [\"/.*\\\\.md(?:[?#].*)?\", \"/.*\\\\.base(?:[?#].*)?\"]"
            )
        );
        assert!(html.contains("function normalizeDocsifyDom() {"));
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
    fn test_render_docsify_index_escapes_homepage_for_javascript_string_literal() {
        let html = render_docsify_index("/docs/%22quoted%22.md");
        assert!(html.contains("homepage: \"/docs/%22quoted%22.md\""));

        let html = render_docsify_index("/docs/\"quoted\".md");
        assert!(html.contains("homepage: \"/docs/\\\"quoted\\\".md\""));
    }
}
