//! Source-note attachment archival.  Attachment records live in Markdown, not DuckDB.

use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::extractor::Extractor;

pub const START_MARKER: &str = "<!-- markbase:source-attachments:start -->";
pub const END_MARKER: &str = "<!-- markbase:source-attachments:end -->";
const RECORD_PREFIX: &str = "<!-- markbase:source-attachment ";
const RECORD_SUFFIX: &str = " -->";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AttachmentRecord {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
    pub mime_type: String,
    pub description: String,
    pub original_path: String,
}

#[derive(Serialize)]
pub struct AttachResult {
    pub source: String,
    pub status: String,
    pub attachment: AttachmentRecord,
}

#[derive(Serialize)]
pub struct VerifyResult {
    pub ok: bool,
    pub attachments: Vec<AttachmentRecord>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<AttachmentIssue>,
}

#[derive(Serialize)]
pub struct AttachmentIssue {
    pub path: String,
    pub code: String,
    pub message: String,
}

struct SourceDocument {
    path: PathBuf,
    relative_path: String,
    content: String,
    section: Section,
}
struct Section {
    start: usize,
    end: usize,
}

pub fn attach(
    base_dir: &Path,
    source_name: &str,
    input: &Path,
    description: &str,
) -> Result<AttachResult, Box<dyn std::error::Error>> {
    if description.trim().is_empty() {
        return Err("Attachment description cannot be empty".into());
    }
    let source = load_source(base_dir, source_name)?;
    let records = records_in_section(&source.content, &source.section)?;
    let input = validate_input(input)?;
    let (sha256, bytes) = hash_file(&input)?;

    if let Some(record) = records.iter().find(|record| {
        record.sha256 == sha256
            && archived_record_file(base_dir, record).is_ok_and(|path| path.is_file())
    }) {
        return Ok(AttachResult {
            source: source.relative_path,
            status: "existing".to_string(),
            attachment: record.clone(),
        });
    }

    let source_parent = source
        .path
        .parent()
        .ok_or("Source note has no parent directory")?;
    let stem = source
        .path
        .file_stem()
        .and_then(|v| v.to_str())
        .ok_or("Source note filename is not UTF-8")?;
    let directory = source_parent.join("attachments").join(stem);
    ensure_safe_directory(base_dir, &directory)?;
    let original_name = input
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or("Input filename is not UTF-8")?;
    let target = choose_target(&directory, original_name, &sha256)?;
    let rel_target = target
        .strip_prefix(base_dir)?
        .to_string_lossy()
        .replace('\\', "/");
    let record = AttachmentRecord {
        path: rel_target,
        sha256,
        bytes,
        mime_type: mime_type(&target).to_string(),
        description: description.trim().to_string(),
        original_path: input.to_string_lossy().to_string(),
    };

    let existed = target.exists();
    if !existed {
        copy_atomic(&input, &target)?;
    }
    let updated = append_record(
        &source.content,
        &source.section,
        source_parent,
        &target,
        &record,
    )?;
    if let Err(error) = write_atomic(&source.path, updated.as_bytes()) {
        if !existed {
            let _ = fs::remove_file(&target);
        }
        return Err(error);
    }
    Ok(AttachResult {
        source: source.relative_path,
        status: if existed { "existing" } else { "copied" }.to_string(),
        attachment: record,
    })
}

pub fn list(
    base_dir: &Path,
    source_name: &str,
) -> Result<Vec<AttachmentRecord>, Box<dyn std::error::Error>> {
    let source = load_source(base_dir, source_name)?;
    records_in_section(&source.content, &source.section)
}

pub fn verify(base_dir: &Path, source_name: &str) -> VerifyResult {
    let source = match load_source(base_dir, source_name) {
        Ok(source) => source,
        Err(error) => {
            return VerifyResult {
                ok: false,
                attachments: vec![],
                issues: vec![issue(source_name, "invalid_source", error.to_string())],
            };
        }
    };
    let records = match records_in_section(&source.content, &source.section) {
        Ok(records) => records,
        Err(error) => {
            return VerifyResult {
                ok: false,
                attachments: vec![],
                issues: vec![issue(
                    &source.relative_path,
                    "invalid_metadata",
                    error.to_string(),
                )],
            };
        }
    };
    let mut issues = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let path = match archived_record_file(base_dir, record) {
            Ok(path) => path,
            Err(error) => {
                issues.push(issue(&record.path, "path_escape", error.to_string()));
                continue;
            }
        };
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_file() => match hash_file(&path) {
                Ok((hash, bytes)) => {
                    if hash != record.sha256 {
                        issues.push(issue(
                            &record.path,
                            "sha256_mismatch",
                            "File SHA-256 does not match its record",
                        ));
                    }
                    if bytes != record.bytes {
                        issues.push(issue(
                            &record.path,
                            "bytes_mismatch",
                            "File byte count does not match its record",
                        ));
                    }
                    if mime_type(&path) != record.mime_type {
                        issues.push(issue(
                            &record.path,
                            "mime_type_mismatch",
                            "File MIME type does not match its record",
                        ));
                    }
                }
                Err(error) => issues.push(issue(&record.path, "unreadable", error.to_string())),
            },
            Ok(_) => issues.push(issue(
                &record.path,
                "not_regular_file",
                "Archived attachment is not a regular file",
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => issues.push(issue(
                &record.path,
                "missing",
                "Archived attachment does not exist",
            )),
            Err(error) => issues.push(issue(&record.path, "unreadable", error.to_string())),
        }
        for other in records.iter().skip(index + 1) {
            if other.path == record.path
                && (other.sha256 != record.sha256 || other.bytes != record.bytes)
            {
                issues.push(issue(
                    &record.path,
                    "duplicate_path_conflict",
                    "Two records point to this path with different content",
                ));
            }
        }
    }
    VerifyResult {
        ok: issues.is_empty(),
        attachments: records,
        issues,
    }
}

fn issue(path: impl AsRef<str>, code: &str, message: impl Into<String>) -> AttachmentIssue {
    AttachmentIssue {
        path: path.as_ref().to_string(),
        code: code.to_string(),
        message: message.into(),
    }
}

fn load_source(
    base_dir: &Path,
    source_name: &str,
) -> Result<SourceDocument, Box<dyn std::error::Error>> {
    let mut found = Vec::new();
    for entry in walkdir::WalkDir::new(base_dir).follow_links(false) {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type().is_file()
            && path.extension().and_then(|v| v.to_str()) == Some("md")
            && path.file_stem().and_then(|v| v.to_str()) == Some(source_name)
        {
            found.push(path.to_path_buf());
        }
    }
    if found.is_empty() {
        return Err(format!("Source note '{}' was not found", source_name).into());
    }
    if found.len() != 1 {
        return Err(format!("Source note '{}' is ambiguous", source_name).into());
    }
    let path = found.remove(0);
    let content = fs::read_to_string(&path)?;
    if Extractor::extract(&content)
        .frontmatter
        .get("type")
        .and_then(|v| v.as_str())
        != Some("source")
    {
        return Err(format!(
            "Note '{}' is not a source note (frontmatter type must be 'source')",
            source_name
        )
        .into());
    }
    let section = find_section(&content)?;
    let relative_path = path
        .strip_prefix(base_dir)?
        .to_string_lossy()
        .replace('\\', "/");
    Ok(SourceDocument {
        path,
        relative_path,
        content,
        section,
    })
}

fn find_section(content: &str) -> Result<Section, Box<dyn std::error::Error>> {
    let start = content
        .find(START_MARKER)
        .ok_or("Source note is missing the managed source-attachments start marker")?;
    if content[start + START_MARKER.len()..].contains(START_MARKER) {
        return Err("Source note has more than one managed source-attachments start marker".into());
    }
    let end = content[start + START_MARKER.len()..]
        .find(END_MARKER)
        .map(|offset| start + START_MARKER.len() + offset)
        .ok_or("Source note is missing the managed source-attachments end marker")?;
    if content[end + END_MARKER.len()..].contains(END_MARKER) {
        return Err("Source note has more than one managed source-attachments end marker".into());
    }
    Ok(Section {
        start: start + START_MARKER.len(),
        end,
    })
}

fn records_in_section(
    content: &str,
    section: &Section,
) -> Result<Vec<AttachmentRecord>, Box<dyn std::error::Error>> {
    let mut records = Vec::new();
    for line in content[section.start..section.end].lines() {
        let line = line.trim();
        if let Some(json) = line
            .strip_prefix(RECORD_PREFIX)
            .and_then(|v| v.strip_suffix(RECORD_SUFFIX))
        {
            records.push(
                serde_json::from_str(json)
                    .map_err(|error| format!("Invalid managed attachment metadata: {}", error))?,
            );
        }
    }
    Ok(records)
}

fn validate_input(input: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let meta = fs::symlink_metadata(input)
        .map_err(|e| format!("Cannot inspect input '{}': {}", input.display(), e))?;
    if meta.file_type().is_symlink() || !meta.file_type().is_file() {
        return Err(format!(
            "Input '{}' must be a readable regular file, not a directory or symlink",
            input.display()
        )
        .into());
    }
    let canonical = input.canonicalize()?;
    File::open(&canonical)
        .map_err(|e| format!("Input '{}' is not readable: {}", input.display(), e))?;
    Ok(canonical)
}

fn archived_record_file(
    base_dir: &Path,
    record: &AttachmentRecord,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let relative = Path::new(&record.path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("Attachment path must be a vault-relative normal path".into());
    }
    let mut path = base_dir.to_path_buf();
    for component in relative.components() {
        path.push(component.as_os_str());
        if fs::symlink_metadata(&path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err("Attachment path must not traverse a symlink".into());
        }
    }
    Ok(path)
}

fn ensure_safe_directory(
    base_dir: &Path,
    directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let relative = directory.strip_prefix(base_dir)?;
    let mut current = base_dir.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(format!(
                    "Attachment directory '{}' must not be a symlink",
                    current.display()
                )
                .into());
            }
            Ok(meta) if !meta.is_dir() => {
                return Err(format!(
                    "Attachment directory '{}' is not a directory",
                    current.display()
                )
                .into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => fs::create_dir(&current)?,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn choose_target(
    directory: &Path,
    name: &str,
    hash: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .and_then(|v| v.to_str())
        .ok_or("Input filename is invalid")?;
    let extension = path.extension().and_then(|v| v.to_str());
    for index in 1u32.. {
        let filename = if index == 1 {
            name.to_string()
        } else if let Some(extension) = extension {
            format!("{}_{:02}.{}", stem, index, extension)
        } else {
            format!("{}_{:02}", stem, index)
        };
        let candidate = directory.join(filename);
        match fs::symlink_metadata(&candidate) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(candidate),
            Ok(meta) if meta.file_type().is_file() => {
                if hash_file(&candidate)?.0 == hash {
                    return Ok(candidate);
                }
            }
            Ok(_) => {}
            Err(error) => return Err(error.into()),
        }
    }
    unreachable!()
}

fn append_record(
    content: &str,
    section: &Section,
    source_parent: &Path,
    target: &Path,
    record: &AttachmentRecord,
) -> Result<String, Box<dyn std::error::Error>> {
    let relative_link = target
        .strip_prefix(source_parent)?
        .to_string_lossy()
        .replace('\\', "/");
    let label = target
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or("Attachment filename is not UTF-8")?;
    let display = format!(
        "- [{}]({}) — description: {}; original path: {}; SHA-256: {}; bytes: {}; MIME type: {}\n  {}{}{}\n",
        label.replace(']', "\\]"),
        relative_link.replace(')', "\\)"),
        record.description.replace('\n', " "),
        record.original_path,
        record.sha256,
        record.bytes,
        record.mime_type,
        RECORD_PREFIX,
        serde_json::to_string(record)?,
        RECORD_SUFFIX
    );
    let mut result = String::with_capacity(content.len() + display.len() + 1);
    result.push_str(&content[..section.end]);
    if !content[..section.end].ends_with('\n') {
        result.push('\n');
    }
    result.push_str(&display);
    result.push_str(&content[section.end..]);
    Ok(result)
}

fn copy_atomic(input: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut from = File::open(input)?;
    let (mut temp, path) = create_temp(target)?;
    std::io::copy(&mut from, &mut temp)?;
    temp.sync_all()?;
    drop(temp);
    fs::rename(path, target)?;
    Ok(())
}
fn write_atomic(target: &Path, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let (mut temp, path) = create_temp(target)?;
    temp.write_all(bytes)?;
    temp.sync_all()?;
    drop(temp);
    fs::rename(path, target)?;
    Ok(())
}
fn create_temp(target: &Path) -> Result<(File, PathBuf), Box<dyn std::error::Error>> {
    for _ in 0..100 {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = target.with_file_name(format!(
            ".{}.markbase-{}-{}.tmp",
            target.file_name().unwrap().to_string_lossy(),
            std::process::id(),
            n
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((file, path)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err("Could not create unique temporary attachment file".into())
}

fn hash_file(path: &Path) -> Result<(String, u64), Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut bytes = 0;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
        bytes += n as u64;
    }
    Ok((hash.finish(), bytes))
}
fn mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "txt" | "md" | "log" => "text/plain",
        "csv" => "text/csv",
        "json" => "application/json",
        "yaml" | "yml" => "application/yaml",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

// A small streaming SHA-256 implementation keeps the command dependency-light.
struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    used: usize,
    length: u64,
}
impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buffer: [0; 64],
            used: 0,
            length: 0,
        }
    }
    fn update(&mut self, input: &[u8]) {
        self.length += input.len() as u64;
        for &byte in input {
            self.buffer[self.used] = byte;
            self.used += 1;
            if self.used == 64 {
                self.block();
                self.used = 0;
            }
        }
    }
    fn block(&mut self) {
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(self.buffer[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            w[i] = w[i - 16]
                .wrapping_add(
                    w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3),
                )
                .wrapping_add(w[i - 7])
                .wrapping_add(
                    w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10),
                );
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) = (
            self.state[0],
            self.state[1],
            self.state[2],
            self.state[3],
            self.state[4],
            self.state[5],
            self.state[6],
            self.state[7],
        );
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (state, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *state = state.wrapping_add(value);
        }
    }
    fn finish(mut self) -> String {
        let bits = self.length * 8;
        self.buffer[self.used] = 0x80;
        self.used += 1;
        if self.used > 56 {
            self.buffer[self.used..].fill(0);
            self.block();
            self.used = 0;
        }
        self.buffer[self.used..56].fill(0);
        self.buffer[56..].copy_from_slice(&bits.to_be_bytes());
        self.block();
        self.state
            .iter()
            .map(|word| format!("{:08x}", word))
            .collect()
    }
}
