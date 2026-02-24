use notify::RecommendedWatcher;
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

#[allow(dead_code)]
pub struct FileWatcher {
    debouncer: notify_debouncer_mini::Debouncer<RecommendedWatcher>,
    rx: mpsc::Receiver<Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>>,
}

impl FileWatcher {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel();

        let mut debouncer = new_debouncer(Duration::from_secs(2), tx)?;
        debouncer
            .watcher()
            .watch(path.as_ref(), RecursiveMode::Recursive)?;

        Ok(Self { debouncer, rx })
    }

    #[allow(dead_code)]
    pub fn wait_for_changes(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let events = self.rx.recv()??;

        let mut paths: Vec<PathBuf> = Vec::new();
        for event in events {
            if let DebouncedEventKind::Any = event.kind {
                if let Some(path) = event.path.extension() {
                    if path == "md" {
                        paths.push(event.path);
                    }
                }
            }
        }

        Ok(paths)
    }

    pub fn wait_for_changes_with_kind(
        &self,
    ) -> Result<Vec<(PathBuf, notify_debouncer_mini::DebouncedEventKind)>, Box<dyn std::error::Error>>
    {
        let events = self.rx.recv()??;

        let mut results: Vec<(PathBuf, notify_debouncer_mini::DebouncedEventKind)> = Vec::new();
        let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

        for event in events {
            if let Some(path) = event.path.extension() {
                if path == "md" && !seen.contains(&event.path) {
                    seen.insert(event.path.clone());
                    results.push((event.path, event.kind));
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::Duration;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn get_unique_id() -> u64 {
        TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    fn create_test_directory() -> PathBuf {
        let temp_dir = std::env::temp_dir();
        let unique_id = get_unique_id();
        let test_dir = temp_dir.join(format!("test_watcher_{}_{}", std::process::id(), unique_id));
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();
        test_dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_watcher_new() {
        let test_dir = create_test_directory();
        let result = FileWatcher::new(&test_dir);
        assert!(result.is_ok());
        cleanup(&test_dir);
    }

    #[test]
    fn test_watcher_detect_create() {
        let test_dir = create_test_directory();
        let mut watcher = FileWatcher::new(&test_dir).unwrap();

        let test_file = test_dir.join("test.md");
        let mut file = fs::File::create(&test_file).unwrap();
        file.write_all(b"# Test").unwrap();
        drop(file);

        thread::sleep(Duration::from_secs(3));

        let result = watcher.wait_for_changes();
        assert!(result.is_ok());
        let paths = result.unwrap();
        assert!(!paths.is_empty());
        assert!(paths
            .iter()
            .any(|p| p.file_name().map_or(false, |n| n == "test.md")));

        cleanup(&test_dir);
    }

    #[test]
    fn test_watcher_detect_modify() {
        let test_dir = create_test_directory();

        let test_file = test_dir.join("test.md");
        let mut file = fs::File::create(&test_file).unwrap();
        file.write_all(b"# Original").unwrap();
        drop(file);

        let mut watcher = FileWatcher::new(&test_dir).unwrap();

        thread::sleep(Duration::from_secs(1));
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&test_file)
            .unwrap();
        file.write_all(b" Modified").unwrap();
        drop(file);

        thread::sleep(Duration::from_secs(3));

        let result = watcher.wait_for_changes();
        assert!(result.is_ok());
        let paths = result.unwrap();
        assert!(!paths.is_empty());

        cleanup(&test_dir);
    }

    #[test]
    fn test_watcher_detect_delete() {
        let test_dir = create_test_directory();

        let test_file = test_dir.join("test.md");
        let mut file = fs::File::create(&test_file).unwrap();
        file.write_all(b"# Test").unwrap();
        drop(file);

        let mut watcher = FileWatcher::new(&test_dir).unwrap();

        thread::sleep(Duration::from_secs(1));
        fs::remove_file(&test_file).unwrap();

        thread::sleep(Duration::from_secs(3));

        let result = watcher.wait_for_changes();
        assert!(result.is_ok());

        cleanup(&test_dir);
    }

    #[test]
    fn test_watcher_ignores_non_md() {
        let test_dir = create_test_directory();
        let mut watcher = FileWatcher::new(&test_dir).unwrap();

        let test_file = test_dir.join("test.txt");
        let mut file = fs::File::create(&test_file).unwrap();
        file.write_all(b"Content").unwrap();
        drop(file);

        thread::sleep(Duration::from_secs(3));

        let result = watcher.wait_for_changes();
        assert!(result.is_ok());
        let paths = result.unwrap();
        assert!(paths.is_empty());

        cleanup(&test_dir);
    }
}
