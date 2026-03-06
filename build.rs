fn main() {
    use std::process::Command;
    use std::time::SystemTime;

    let sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

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
        None => {
            // Fallback to build timestamp if git is unavailable
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .ok()
                .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
                .map_or("unknown".to_string(), |dt| {
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                })
        }
    };

    let sha_str = sha.as_deref().unwrap_or("unknown");
    println!("cargo:rustc-env=VERGEN_GIT_SHA={}", sha_str);
    println!("cargo:rustc-env=VERGEN_GIT_COMMIT_TIMESTAMP={}", timestamp);
}
