fn main() {
    use std::process::Command;

    let sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let timestamp = Command::new("git")
        .args(["log", "-1", "--format=%at"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let timestamp = if let Ok(ts) = timestamp.parse::<i64>() {
        chrono::DateTime::from_timestamp(ts, 0).map_or(timestamp.clone(), |dt| {
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        })
    } else {
        timestamp
    };

    println!("cargo:rustc-env=VERGEN_GIT_SHA={}", sha);
    println!("cargo:rustc-env=VERGEN_GIT_COMMIT_TIMESTAMP={}", timestamp);
}
