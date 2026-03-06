fn main() {
    use std::process::Command;

    // Check if we're in a git repository
    let in_git_repo = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .ok()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let version = if in_git_repo {
        // In git repo - include SHA and timestamp
        let sha = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();

        let git_timestamp = Command::new("git")
            .args(["log", "-1", "--format=%at"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        let timestamp = match git_timestamp {
            Some(ts_str) => {
                if let Ok(ts) = ts_str.parse::<i64>() {
                    chrono::DateTime::from_timestamp(ts, 0).map_or(ts_str.clone(), |dt| {
                        dt.format("%Y-%m-%d %H:%M:%S").to_string()
                    })
                } else {
                    ts_str
                }
            }
            None => String::new(),
        };

        if sha.is_empty() {
            env!("CARGO_PKG_VERSION").to_string()
        } else {
            format!("{} ({} {})", env!("CARGO_PKG_VERSION"), sha, timestamp)
        }
    } else {
        // Not in git repo (e.g., crates.io install) - just version number
        env!("CARGO_PKG_VERSION").to_string()
    };

    println!("cargo:rustc-env=MARKBASE_VERSION={}", version);
}
