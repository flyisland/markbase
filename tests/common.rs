#![allow(dead_code)]

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tempfile::TempDir;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn get_unique_id() -> u64 {
    TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
}

pub struct TestVault {
    _temp_dir: TempDir,
    pub path: PathBuf,
}

impl TestVault {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let unique_id = get_unique_id();
        let vault_path =
            temp_dir
                .path()
                .join(format!("test_vault_{}_{}", std::process::id(), unique_id));
        std::fs::create_dir_all(&vault_path).unwrap();

        Self {
            _temp_dir: temp_dir,
            path: vault_path,
        }
    }

    pub fn create_note(&self, name: &str, content: &str) -> PathBuf {
        let note_path = self.path.join(format!("{}.md", name));
        std::fs::write(&note_path, content).unwrap();
        note_path
    }

    pub fn create_note_in_subdir(&self, subdir: &str, name: &str, content: &str) -> PathBuf {
        let dir = self.path.join(subdir);
        std::fs::create_dir_all(&dir).unwrap();
        let note_path = dir.join(format!("{}.md", name));
        std::fs::write(&note_path, content).unwrap();
        note_path
    }

    pub fn create_file(&self, name: &str, content: &str) -> PathBuf {
        let file_path = self.path.join(name);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file_path, content).unwrap();
        file_path
    }

    pub fn create_gitignore(&self, content: &str) {
        std::fs::write(self.path.join(".gitignore"), content).unwrap();
    }

    pub fn create_markbaseignore(&self, content: &str) {
        std::fs::write(self.path.join(".markbaseignore"), content).unwrap();
    }

    pub fn delete_file(&self, name: &str) {
        let file_path = self.path.join(name);
        std::fs::remove_file(&file_path).unwrap();
    }

    pub fn db_path(&self) -> PathBuf {
        self.path.join(".markbase/markbase.duckdb")
    }

    pub fn run_cli(&self, args: &[&str]) -> Output {
        let binary_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("markbase")))
            .unwrap_or_else(|| PathBuf::from("target/release/markbase"));

        let cmd_path = if binary_path.exists() {
            binary_path
        } else {
            PathBuf::from("target/debug/markbase")
        };

        let mut cmd = Command::new(&cmd_path);
        cmd.args(["--base-dir", &self.path.to_string_lossy()])
            .args(args)
            .env_remove("MARKBASE_BASE_DIR");

        let output = cmd
            .output()
            .expect(&format!("Failed to run CLI command: {:?}", cmd_path));
        output
    }

    pub fn run_cli_verbose(&self, args: &[&str]) -> (Output, String, String) {
        let output = self.run_cli(args);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        (output, stdout, stderr)
    }

    pub fn index(&self) -> Output {
        self.run_cli(&[
            "--index-log-level",
            "summary",
            "query",
            "file.name == '__markbase_missing__'",
        ])
    }

    pub fn index_verbose(&self) -> (Output, String, String) {
        self.run_cli_verbose(&["--index-log-level", "verbose", "query", ""])
    }

    pub fn index_force(&self) -> Output {
        self.run_cli(&[
            "--index-log-level",
            "summary",
            "query",
            "file.name == '__markbase_missing__'",
        ])
    }

    pub fn index_force_verbose(&self) -> (Output, String, String) {
        self.run_cli_verbose(&["--index-log-level", "verbose", "query", ""])
    }

    pub fn query(&self, sql: &str) -> Output {
        self.run_cli(&["query", sql])
    }

    pub fn query_format(&self, sql: &str, format: &str) -> Output {
        self.run_cli(&["query", sql, "-o", format])
    }

    pub fn query_abs_path(&self, sql: &str) -> Output {
        self.run_cli(&["query", sql, "--abs-path"])
    }

    pub fn query_dry_run(&self, sql: &str) -> Output {
        self.run_cli(&["query", sql, "--dry-run"])
    }

    pub fn query_with_backlinks(&self, sql: &str) -> Output {
        self.run_cli(&["--compute-backlinks", "query", sql])
    }

    pub fn note_new(&self, name: &str) -> Output {
        self.run_cli(&["note", "new", name])
    }

    pub fn note_new_with_template(&self, name: &str, template: &str) -> Output {
        self.run_cli(&["note", "new", name, "--template", template])
    }

    pub fn note_rename(&self, old_name: &str, new_name: &str) -> Output {
        self.run_cli(&["note", "rename", old_name, new_name])
    }

    pub fn template_list(&self) -> Output {
        self.run_cli(&["template", "list"])
    }

    pub fn template_describe(&self, name: &str) -> Output {
        self.run_cli(&["template", "describe", name])
    }

    pub fn note_verify(&self, name: &str) -> Output {
        self.run_cli(&["note", "verify", name])
    }

    pub fn note_resolve(&self, names: &[&str]) -> Output {
        let mut args = vec!["note", "resolve"];
        args.extend(names.iter().copied());
        self.run_cli(&args)
    }

    pub fn web_get(&self, canonical_url: &str) -> Output {
        self.run_cli(&["web", "get", canonical_url])
    }

    pub fn web_init_docsify(&self, homepage: &str) -> Output {
        self.run_cli(&["web", "init-docsify", "--homepage", homepage])
    }

    pub fn web_init_docsify_force(&self, homepage: &str) -> Output {
        self.run_cli(&["web", "init-docsify", "--homepage", homepage, "--force"])
    }

    pub fn spawn_web_server(&self, bind: &str, port: u16) -> TestServer {
        self.spawn_web_server_with_cache_control(bind, port, None)
    }

    pub fn spawn_web_server_with_cache_control(
        &self,
        bind: &str,
        port: u16,
        cache_control: Option<&str>,
    ) -> TestServer {
        let binary_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("markbase")))
            .unwrap_or_else(|| PathBuf::from("target/release/markbase"));

        let cmd_path = if binary_path.exists() {
            binary_path
        } else {
            PathBuf::from("target/debug/markbase")
        };

        let mut command = Command::new(&cmd_path);
        command.args([
            "--base-dir",
            &self.path.to_string_lossy(),
            "web",
            "serve",
            "--bind",
            bind,
            "--port",
            &port.to_string(),
        ]);
        if let Some(cache_control) = cache_control {
            command.args(["--cache-control", cache_control]);
        }

        let child = command
            .env_remove("MARKBASE_BASE_DIR")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn web server");

        wait_for_port(bind, port);

        TestServer { child, port }
    }
}

impl Default for TestVault {
    fn default() -> Self {
        Self::new()
    }
}

pub fn assert_cli_success(output: &Output) {
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "CLI command failed.\nStatus: {}\nstdout: {}\nstderr: {}",
            output.status, stdout, stderr
        );
    }
}

pub fn assert_cli_error(output: &Output) {
    assert!(
        !output.status.success(),
        "Expected CLI command to fail, but it succeeded"
    );
}

pub fn stdout_contains(output: &Output, text: &str) -> bool {
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.contains(text)
}

pub fn stderr_contains(output: &Output, text: &str) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr.contains(text)
}

pub fn pick_free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

pub struct TestServer {
    child: Child,
    pub port: u16,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

pub fn http_get(port: u16, path: &str) -> HttpResponse {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
        path, port
    );
    stream.write_all(request.as_bytes()).unwrap();

    let mut response = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => response.extend_from_slice(&buffer[..read]),
            Err(err) if err.kind() == std::io::ErrorKind::ConnectionReset => break,
            Err(err) => panic!("failed to read HTTP response: {}", err),
        }
    }
    parse_http_response(&response)
}

fn wait_for_port(bind: &str, port: u16) {
    for _ in 0..50 {
        if TcpStream::connect((bind, port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("web server did not start listening on {}:{}", bind, port);
}

fn parse_http_response(raw: &[u8]) -> HttpResponse {
    let header_end = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("missing HTTP header terminator");
    let (header_bytes, body_bytes) = raw.split_at(header_end + 4);
    let header_text = String::from_utf8_lossy(header_bytes);
    let mut lines = header_text.split("\r\n").filter(|line| !line.is_empty());
    let status_line = lines.next().unwrap();
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    HttpResponse {
        status_code,
        headers,
        body: body_bytes.to_vec(),
    }
}

pub fn parse_index_stats(output: &Output) -> IndexStats {
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let mut stats = IndexStats::default();

    if let Some(caps) = regex::Regex::new(
        r"Indexed: \d+ new, \d+ updated, \d+ deleted, \d+ errors, \d+ warnings — (\d+) total notes",
    )
    .unwrap()
    .captures(&stderr)
    {
        stats.total = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
    } else if let Some(caps) = regex::Regex::new(r"— (\d+) total notes")
        .unwrap()
        .captures(&stdout)
    {
        stats.total = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
    } else if let Some(caps) = regex::Regex::new(r"✓ (\d+) files indexed")
        .unwrap()
        .captures(&stdout)
    {
        stats.total = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
    }

    if let Some(caps) = regex::Regex::new(r"Indexed: (\d+) new")
        .unwrap()
        .captures(&stderr)
    {
        stats.new = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
    } else if let Some(caps) = regex::Regex::new(r"\((\d+) new").unwrap().captures(&stdout) {
        stats.new = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
    }

    if let Some(caps) = regex::Regex::new(r"new, (\d+) updated")
        .unwrap()
        .captures(&stderr)
    {
        stats.updated = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
    } else if let Some(caps) = regex::Regex::new(r", (\d+) updated")
        .unwrap()
        .captures(&stdout)
    {
        stats.updated = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
    }

    if let Some(caps) = regex::Regex::new(r"updated, (\d+) deleted")
        .unwrap()
        .captures(&stderr)
    {
        stats.deleted = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
    } else if let Some(caps) = regex::Regex::new(r", (\d+) deleted")
        .unwrap()
        .captures(&stdout)
    {
        stats.deleted = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
    }

    if let Some(caps) = regex::Regex::new(r"deleted, (\d+) errors")
        .unwrap()
        .captures(&stderr)
    {
        stats.errors = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
    } else if let Some(caps) = regex::Regex::new(r", (\d+) errors\)")
        .unwrap()
        .captures(&stdout)
    {
        stats.errors = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
    }

    for line in stderr.lines() {
        if line.contains("+ ") {
            stats
                .new_files
                .push(line.trim_start_matches("    + ").to_string());
        } else if line.contains("~ ") {
            stats
                .updated_files
                .push(line.trim_start_matches("    ~ ").to_string());
        } else if line.contains("⚠ Skipped:") {
            if let Some(caps) = regex::Regex::new(r"⚠ Skipped: (.+?) — (.+)")
                .unwrap()
                .captures(line)
            {
                stats.skipped.push((
                    caps.get(1).unwrap().as_str().to_string(),
                    caps.get(2).unwrap().as_str().to_string(),
                ));
            }
        }
    }

    stats
}

#[derive(Debug, Default)]
pub struct IndexStats {
    pub total: usize,
    pub new: usize,
    pub updated: usize,
    pub deleted: usize,
    pub errors: usize,
    pub new_files: Vec<String>,
    pub updated_files: Vec<String>,
    pub skipped: Vec<(String, String)>,
}
