fn main() {
    use std::process::Command;

    // Check if we're in a git repository
    let in_git_repo = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .ok()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let version = env!("CARGO_PKG_VERSION").to_string();
    let git_commit = if in_git_repo {
        Command::new("git")
            .args(["rev-parse", "--short=12", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        "unknown".to_string()
    };

    let git_commit_time = if in_git_repo {
        Command::new("git")
            .args(["show", "-s", "--format=%cI", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        "unknown".to_string()
    };

    let decorated_version = if git_commit == "unknown" || git_commit_time == "unknown" {
        version.clone()
    } else {
        format!("{} ({} {})", version, git_commit, git_commit_time)
    };

    println!("cargo:rustc-env=MARKBASE_VERSION={}", decorated_version);
    println!("cargo:rustc-env=MARKBASE_BUILD_VERSION={}", version);
    println!("cargo:rustc-env=MARKBASE_GIT_COMMIT={}", git_commit);
    println!(
        "cargo:rustc-env=MARKBASE_GIT_COMMIT_TIME={}",
        git_commit_time
    );
}
