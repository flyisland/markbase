#![allow(dead_code)]

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
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
        self.run_cli(&["index"])
    }

    pub fn index_verbose(&self) -> (Output, String, String) {
        self.run_cli_verbose(&["index", "--verbose"])
    }

    pub fn index_force(&self) -> Output {
        self.run_cli(&["index", "--force"])
    }

    pub fn index_force_verbose(&self) -> (Output, String, String) {
        self.run_cli_verbose(&["index", "--force"])
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

pub fn parse_index_stats(output: &Output) -> IndexStats {
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let mut stats = IndexStats::default();

    if let Some(caps) = regex::Regex::new(r"— (\d+) total notes")
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

    if let Some(caps) = regex::Regex::new(r"\((\d+) new").unwrap().captures(&stdout) {
        stats.new = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
    }

    if let Some(caps) = regex::Regex::new(r", (\d+) updated")
        .unwrap()
        .captures(&stdout)
    {
        stats.updated = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
    }

    if let Some(caps) = regex::Regex::new(r", (\d+) deleted")
        .unwrap()
        .captures(&stdout)
    {
        stats.deleted = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
    }

    if let Some(caps) = regex::Regex::new(r", (\d+) errors\)")
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
