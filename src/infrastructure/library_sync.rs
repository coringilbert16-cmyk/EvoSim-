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

fn run_git(
    repo: &Path,
    args: &[&str],
    index: Option<&Path>,
) -> std::io::Result<std::process::Output> {
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

fn data_paths(repo: &Path) -> Vec<String> {
    const GEOMETRY_FILES: &[&str] = &[
        "formations.jsonl",
        "manifest.json",
        "frontier.json",
        "contact_families.jsonl",
        "fluid_boundary_families.jsonl",
        "rigid_contact_families.jsonl",
        "rigid_point_contact_families.jsonl",
        "rigid_vertex_contact_families.jsonl",
    ];
    const CHEMISTRY_FILES: &[&str] = &["chemistry.jsonl", "manifest.json"];

    GEOMETRY_FILES
        .iter()
        .map(|file| format!("{GEOMETRY_DATA}/{file}"))
        .chain(
            CHEMISTRY_FILES
                .iter()
                .map(|file| format!("{CHEMISTRY_DATA}/{file}")),
        )
        .filter(|path| repo.join(path).is_file())
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

fn current_branch(repo: &Path) -> Option<String> {
    let output = run_git(repo, &["symbolic-ref", "--short", "HEAD"], None).ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8(output.stdout).ok()?;
    let branch = branch.trim();
    if branch.is_empty() {
        None
    } else {
        Some(branch.to_owned())
    }
}

fn remote_is_ancestor(repo: &Path, branch: &str) -> bool {
    let remote_ref = format!("origin/{branch}");
    let remote = match run_git(repo, &["rev-parse", &remote_ref], None) {
        Ok(output) if output.status.success() => remote_ref,
        // A branch that does not exist on origin yet has no remote history to
        // protect. The first checkpoint may publish it.
        _ => return true,
    };

    match run_git(
        repo,
        &["merge-base", "--is-ancestor", &remote, "HEAD"],
        None,
    ) {
        Ok(output) => output.status.success(),
        // A failed ancestry check is fail-closed: do not manufacture a
        // checkpoint on a branch whose remote relationship is unknown.
        Err(_) => false,
    }
}

fn push_if_ahead(repo: &Path, branch: &str) {
    let comparison = run_git(
        repo,
        &[
            "rev-list",
            "--left-right",
            "--count",
            &format!("origin/{branch}...{branch}"),
        ],
        None,
    );

    let should_push = match comparison {
        Ok(output) if output.status.success() => {
            let counts = String::from_utf8_lossy(&output.stdout);
            let mut values = counts.split_whitespace();
            let _behind = values.next().and_then(|value| value.parse::<u64>().ok());
            values
                .next()
                .map(|ahead| ahead.parse::<u64>().unwrap_or(0) > 0)
                .unwrap_or(false)
        }
        // A missing remote branch is treated as needing the initial push.
        _ => true,
    };

    if !should_push {
        return;
    }

    match run_git(repo, &["push", "origin", branch], None) {
        Ok(output) if output.status.success() => {
            eprintln!("library sync: pushed {branch}");
        }
        Ok(output) => {
            eprintln!(
                "library sync: push deferred: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Err(error) => {
            eprintln!("library sync: push deferred: {error}");
        }
    }
}

fn publish_once() -> std::io::Result<bool> {
    let Some(repo) = repo_root() else {
        return Ok(false);
    };
    let Some(branch) = current_branch(&repo) else {
        return Ok(false);
    };
    let paths = data_paths(&repo);

    if !remote_is_ancestor(&repo, &branch) {
        eprintln!(
            "library sync: deferred because origin/{branch} is ahead of the local branch"
        );
        return Ok(false);
    }

    if paths.is_empty() {
        push_if_ahead(&repo, &branch);
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
        add_args.extend(paths.iter().map(String::as_str));
        let output = run_git(&repo, &add_args, Some(&index))?;
        if !output.status.success() {
            return Ok(false);
        }

        let output = run_git(&repo, &["diff", "--cached", "--quiet"], Some(&index))?;

        if output.status.success() {
            push_if_ahead(&repo, &branch);
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
            eprintln!(
                "library sync: commit deferred: {}",
                String::from_utf8_lossy(&commit.stderr).trim()
            );
            return Ok(false);
        }
        let commit_sha = String::from_utf8_lossy(&commit.stdout).trim().to_owned();

        // Only advance the local branch if HEAD is still the commit we read
        // before staging. If the user committed concurrently, leave the
        // checkpoint commit unreachable and retry against the new HEAD later.
        let update = run_git(
            &repo,
            &[
                "update-ref",
                &format!("refs/heads/{branch}"),
                &commit_sha,
                &parent,
            ],
            None,
        )?;
        if !update.status.success() {
            return Ok(false);
        }

        eprintln!("library sync: created checkpoint {commit_sha}");
        push_if_ahead(&repo, &branch);
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
