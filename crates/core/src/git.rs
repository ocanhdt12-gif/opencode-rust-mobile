//! Git helpers via **shell-out** (decision #4: call the `git` binary, not libgit2).
//!
//! Keeping the legacy behaviour means we invoke `git` exactly like the `.mjs`
//! scripts did (e.g. `git rev-parse --short HEAD`, worktree add/merge). This avoids
//! a libgit2 dependency and matches edge cases of the user's installed git.

use crate::Result;
use std::path::Path;
use std::process::Command;

/// Run `git` with `args` in `cwd`, returning trimmed stdout on success.
fn git_output(cwd: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git").current_dir(cwd).args(args).output()?;
    if !out.status.success() {
        anyhow::bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Return the short HEAD sha (`git rev-parse --short HEAD`) run in `cwd`.
pub fn short_head(cwd: &Path) -> Result<String> {
    git_output(cwd, &["rev-parse", "--short", "HEAD"])
}

/// Short HEAD sha, or `fallback` when not a git repo / git missing.
///
/// Mirrors the legacy script's `try { … } catch { return "no-git" }`.
pub fn short_head_or(cwd: &Path, fallback: &str) -> String {
    short_head(cwd).unwrap_or_else(|_| fallback.to_string())
}

/// List existing git worktrees (`git worktree list --porcelain`), one entry per
/// `worktree <path>` line.
#[allow(dead_code)]
pub fn worktrees(cwd: &Path) -> Result<Vec<String>> {
    let text = git_output(cwd, &["worktree", "list", "--porcelain"])?;
    Ok(text
        .lines()
        .filter_map(|l| l.strip_prefix("worktree "))
        .map(|s| s.to_string())
        .collect())
}
