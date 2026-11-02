//! Pull latest managed paths from the canonical template into a project
//! (port of scripts/template-sync.mjs).
//!
//! Only overwrites TEMPLATE-owned paths (managedPaths): AGENTS.md, .agent/,
//! .opencode/, scripts/, skills/. Never touches code, SPECIFICATIONS.md, BRIEF.md,
//! docs/, spec/, tasks/, .context/, .devops/, .env*, package.json, .gitignore,
//! README.md.
//!
//! Safety: if managedPaths have UNCOMMITTED changes in the project, sync aborts.
//!
//! The actual clone/fetch/rsync is performed by the `tools` command (shell-out,
//! decision #4). This module holds the pure, testable pieces: config resolution,
//! repo-name derivation, and rsync itemize-output parsing.

use serde::Deserialize;
use std::path::Path;

pub const TEMPLATE_REPO: &str = "git@github.com:ocanhdt12-gif/opencode-rust-mobile.git";
pub const BRANCH: &str = "main";

/// Default managed/report-only paths (match the legacy DEFAULTS).
pub fn default_managed() -> Vec<String> {
    ["AGENTS.md", ".agent", ".opencode", "scripts", "skills"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}
pub fn default_report_only() -> Vec<String> {
    vec!["opencode.jsonc".to_string()]
}

/// Optional `<project>/.template-sync.json`.
#[derive(Debug, Default, Deserialize)]
pub struct SyncConfig {
    #[serde(rename = "managedPaths")]
    pub managed_paths: Option<Vec<String>>,
    #[serde(rename = "reportOnlyPaths")]
    pub report_only_paths: Option<Vec<String>>,
    #[serde(rename = "templateRepo")]
    pub template_repo: Option<String>,
}

/// Load `.template-sync.json` from `root` unless `no_config`.
pub fn load_config(root: &Path, no_config: bool) -> crate::Result<SyncConfig> {
    if no_config {
        return Ok(SyncConfig::default());
    }
    let p = root.join(".template-sync.json");
    if !p.exists() {
        return Ok(SyncConfig::default());
    }
    let text = std::fs::read_to_string(&p)?;
    let cfg: SyncConfig = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!(".template-sync.json không hợp lệ: {e}"))?;
    Ok(cfg)
}

/// Resolved effective settings after merging CLI flag, config, and defaults.
pub struct Resolved {
    pub managed: Vec<String>,
    pub report_only: Vec<String>,
    pub repo_url: String,
    pub name: String,
}

/// Merge precedence: CLI `--template` > config > default; name derived from url.
pub fn resolve(cfg: &SyncConfig, template_flag: Option<&str>) -> Resolved {
    let managed = cfg.managed_paths.clone().unwrap_or_else(default_managed);
    let report_only = cfg
        .report_only_paths
        .clone()
        .unwrap_or_else(default_report_only);
    let repo_url = template_flag
        .map(|s| s.to_string())
        .or_else(|| cfg.template_repo.clone())
        .unwrap_or_else(|| TEMPLATE_REPO.to_string());
    let name = repo_name(&repo_url);
    Resolved {
        managed,
        report_only,
        repo_url,
        name,
    }
}

/// Basename of a repo url, stripping trailing `.git` and `/`.
pub fn repo_name(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    let trimmed = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    trimmed
        .rsplit(['/', ':'])
        .next()
        .unwrap_or(trimmed)
        .to_string()
}

/// One change detected from rsync `-i` itemized output.
#[derive(Debug, PartialEq, Eq)]
pub enum Change {
    Add(String),
    Update(String),
    Delete(String),
}

/// Parse rsync itemize-changes stdout into structured changes.
/// Lines starting with `>` or `*` are relevant (file transfers / deletions).
pub fn parse_rsync_itemize(stdout: &str) -> Vec<Change> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        if !(line.starts_with('>') || line.starts_with('*')) {
            continue;
        }
        let sp = match line.find(char::is_whitespace) {
            Some(i) => i,
            None => continue,
        };
        let flags = &line[..sp];
        let file = line[sp..].trim().to_string();
        if flags.starts_with("*deleting") {
            out.push(Change::Delete(file));
        } else if flags.contains("+++++++++") {
            out.push(Change::Add(file));
        } else {
            out.push(Change::Update(file));
        }
    }
    out
}

/// Summary counts.
pub struct Summary {
    pub added: usize,
    pub updated: usize,
    pub deleted: usize,
}

/// Tally a set of changes.
pub fn summarize(changes: &[Change]) -> Summary {
    let mut s = Summary {
        added: 0,
        updated: 0,
        deleted: 0,
    };
    for c in changes {
        match c {
            Change::Add(_) => s.added += 1,
            Change::Update(_) => s.updated += 1,
            Change::Delete(_) => s.deleted += 1,
        }
    }
    s
}
