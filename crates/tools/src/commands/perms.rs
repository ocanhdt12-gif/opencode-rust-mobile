//! `perms` — port of scripts/apply-verify-permissions.mjs.
//!
//! Legacy CLI: default is **dry-run**; `--write` required to actually write;
//! `--dry-run` explicit. Syncs reviewer/spec-validator bash allow-rules with the
//! verify commands in `.context/project-config.md`, only between the
//! `# verify-commands:start/end` markers. Idempotent; refuses unsafe commands.

use clap::Args as ClapArgs;
use opencode_core::log;
use opencode_core::perms::{self, PatchStatus, Verdict};
use std::env;
use std::path::Path;

#[derive(ClapArgs)]
pub struct Args {
    /// Actually write files (default is dry-run).
    #[arg(long)]
    write: bool,

    /// Explicit dry-run (preview only). This is also the default.
    #[arg(long = "dry-run")]
    dry_run: bool,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let dry_run = !args.write;
    let root = env::current_dir()?;
    let profile = root.join(".context/project-config.md");
    if !profile.exists() {
        anyhow::bail!("Không tìm thấy {}", profile.display());
    }

    let text = std::fs::read_to_string(&profile)?;
    let commands = perms::parse_commands(&text);

    let mut patterns: Vec<String> = Vec::new();
    let mut skipped: Vec<(String, String)> = Vec::new();
    for cmd in &commands {
        match perms::classify(cmd) {
            Verdict::Ok(p) => patterns.push(p),
            Verdict::Skip(reason) => skipped.push((cmd.clone(), reason)),
        }
    }

    if patterns.is_empty() {
        log::info("Không có verify command nào đủ an toàn để auto-allow.");
    }

    let files = perms::agent_files(&root);
    let missing: Vec<&std::path::PathBuf> = files.iter().filter(|f| !f.exists()).collect();
    if !missing.is_empty() {
        log::info("Thiếu agent file — không ghi file nào:");
        for f in &missing {
            log::info(&format!("- {}", rel(&root, f)));
        }
        std::process::exit(1);
    }

    // marker check
    let mut missing_markers = Vec::new();
    for f in &files {
        let c = std::fs::read_to_string(f)?;
        let (_, st) = perms::patch_content(&c, &patterns, "\n");
        if st == PatchStatus::NoMarkers {
            missing_markers.push(f.clone());
        }
    }
    if !missing_markers.is_empty() {
        log::info("Thiếu marker — không ghi file nào:");
        for f in &missing_markers {
            log::info(&format!(
                "- {}: cần `{}`/`{}`.",
                rel(&root, f),
                perms::START,
                perms::END
            ));
        }
        std::process::exit(1);
    }

    let sample = std::fs::read_to_string(&files[0])?;
    let eol = perms::detect_eol(&sample);

    for f in &files {
        let content = std::fs::read_to_string(f)?;
        let (updated, st) = perms::patch_content(&content, &patterns, eol);
        let status_str = match st {
            PatchStatus::Unchanged => "unchanged",
            PatchStatus::WouldUpdate if dry_run => "would-update",
            PatchStatus::WouldUpdate => {
                std::fs::write(f, &updated)?;
                "updated"
            }
            PatchStatus::Updated => "updated",
            PatchStatus::Missing => "missing",
            PatchStatus::NoMarkers => "no-markers",
        };
        log::info(&format!(
            "- {}: {} ({} rule).",
            rel(&root, f),
            status_str,
            patterns.len()
        ));
    }

    if !patterns.is_empty() {
        let list = patterns
            .iter()
            .map(|p| format!("`{p}`"))
            .collect::<Vec<_>>()
            .join(", ");
        log::info(&format!("\nAllow patterns: {list}"));
    }
    if !skipped.is_empty() {
        log::info("\nBỏ qua (cần user tự thêm rule nếu muốn):");
        for (cmd, reason) in &skipped {
            log::info(&format!("- `{cmd}` — {reason}"));
        }
    }
    if dry_run {
        log::dry_run_notice();
    }
    Ok(())
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}
