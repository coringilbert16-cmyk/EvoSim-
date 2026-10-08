//! Background checkpointing for the persistent Bob/chemistry knowledge stores.
//!
//! Library persistence stays independent of Git. This module only observes the
//! durable library directories and publishes a read-only mirror to the current
//! Git branch. It uses a temporary Git index so the background process never
//! stages or commits the user's normal working-tree changes.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const DEFAULT_INTERVAL: Duration = Duration::from_secs(120);
const GEOMETRY_DATA: &str = "geometry_library/data";
const CHEMISTRY_DATA: &str = "chemistry_library/data";

fn interval() -> Duration {
    std::env::var("EVOSIM_LIBRARY_SYNC_INTERVAL_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_INTERVAL)
}

fn run_git(repo: &Path, args: &[&str], index: Option<&Path>) -> std::io::Result<std::process::Output> {
    let mut command = Command::new("git");
    command.current_dir(repo).args(args);
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    command.output()
}

fn repo_root() -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let root = String::from_utf8(output.stdout).ok()?;
    let root = root.trim();
    if root.is_empty() {
        None
    } else {
        Some(PathBuf::from(root))
    }
}

fn data_paths(repo: &Path) -> Vec<&'static str> {
    [GEOMETRY_DATA, CHEMISTRY_DATA]
        .into_iter()
        .filter(|path| repo.join(path).exists())
        .collect()
}

fn temporary_index_path(repo: &Path) -> PathBuf {
    let pid = std::process::id();
    repo.join(".evosim-runner")
        .join(format!("library-sync-index-{pid}"))
}

fn remove_temp_index(path: &Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(path.with_extension("lock"));
}

fn publish_once() -> std::io::Result<bool> {
    let Some(repo) = repo_root() else {
        return Ok(false);
    };
    let paths = data_paths(&repo);
    if paths.is_empty() {
        return Ok(false);
    }

    fs::create_dir_all(repo.join(".evosim-runner"))?;
    let index = temporary_index_path(&repo);
    remove_temp_index(&index);

    let result = (|| -> std::io::Result<bool> {
        let output = run_git(&repo, &["rev-parse", "HEAD"], None)?;
        if !output.status.success() {
            return Ok(false);
        }
        let parent = String::from_utf8_lossy(&output.stdout).trim().to_owned();

        let output = run_git(&repo, &["read-tree", &parent], Some(&index))?;
        if !output.status.success() {
            return Ok(false);
        }

        let mut add_args = vec!["add", "-f", "--"];
        add_args.extend(paths.iter().copied());
        let output = run_git(&repo, &add_args, Some(&index))?;
        if !output.status.success() {
            return Ok(false);
        }

        let output = run_git(
            &repo,
            &["diff", "--cached", "--quiet", "--", GEOMETRY_DATA, CHEMISTRY_DATA],
            Some(&index),
        )?;

        if output.status.success() {
            return Ok(false);
        }
        if output.status.code() != Some(1) {
            return Ok(false);
        }

        let tree = run_git(&repo, &["write-tree"], Some(&index))?;
        if !tree.status.success() {
            return Ok(false);
        }
        let tree_sha = String::from_utf8_lossy(&tree.stdout).trim().to_owned();

        let commit = Command::new("git")
            .current_dir(&repo)
            .env("GIT_INDEX_FILE", &index)
            .args([
                "commit-tree",
                &tree_sha,
                "-p",
                &parent,
                "-m",
                "Checkpoint persistent geometry and chemistry libraries",
            ])
            .output()?;

        if !commit.status.success() {
            return Ok(false);
        }
        let commit_sha = String::from_utf8_lossy(&commit.stdout).trim().to_owned();

        let branch = run_git(&repo, &["symbolic-ref", "--short", "HEAD"], None)?;
        if !branch.status.success() {
            return Ok(false);
        }
        let branch = String::from_utf8_lossy(&branch.stdout).trim().to_owned();
        if branch.is_empty() {
            return Ok(false);
        }

        // Only advance the local branch if HEAD is still the commit we read
        // before staging. If the user committed concurrently, leave the
        // checkpoint commit unreachable and retry against the new HEAD later.
        let update = run_git(
            &repo,
            &["update-ref", &format!("refs/heads/{branch}"), &commit_sha, &parent],
            None,
        )?;
        if !update.status.success() {
            return Ok(false);
        }

        let push = run_git(&repo, &["push", "origin", &branch], None)?;
        if !push.status.success() {
            eprintln!(
                "library sync: checkpoint {commit_sha} created locally; push will be retried"
            );
        } else {
            eprintln!("library sync: published checkpoint {commit_sha}");
        }

        Ok(true)
    })();

    remove_temp_index(&index);
    result
}

/// Publish one checkpoint if either persistent library has changed.
pub fn checkpoint_once() -> std::io::Result<bool> {
    publish_once()
}

/// Start the non-blocking background publisher used by the normal EvoSim
/// processes. Git/network failures never stop the simulation or worker.
pub fn spawn_background() {
    if std::env::var("EVOSIM_LIBRARY_SYNC_DISABLE").as_deref() == Ok("1") {
        return;
    }

    thread::spawn(|| {
        loop {
            if let Err(error) = publish_once() {
                eprintln!("library sync: {error}");
            }
            thread::sleep(interval());
        }
    });
}

/// Run the publisher as a foreground daemon.
pub fn run_foreground() {
    loop {
        if let Err(error) = publish_once() {
            eprintln!("library sync: {error}");
        }
        thread::sleep(interval());
    }
}
