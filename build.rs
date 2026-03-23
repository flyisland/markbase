fn main() {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    // Check if we're in a git repository
    let git_dir = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let in_git_repo = git_dir.is_some();

    if let Some(git_dir) = git_dir.as_ref() {
        let git_dir_path = PathBuf::from(git_dir);
        let head_path = git_dir_path.join("HEAD");
        println!("cargo:rerun-if-changed={}", head_path.display());

        if let Ok(head_contents) = fs::read_to_string(&head_path)
            && let Some(reference) = head_contents.trim().strip_prefix("ref: ")
        {
            let reference_path = git_dir_path.join(reference);
            println!("cargo:rerun-if-changed={}", reference_path.display());
        }
    }

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
