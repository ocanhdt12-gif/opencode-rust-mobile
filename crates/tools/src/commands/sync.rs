//! `sync` — port of scripts/template-sync.mjs.
//!
//! Legacy CLI: `--project <path>`, `--dry-run`, `--prune`, `--template <git-url>`,
//! `--no-config`. Only writes managed paths. Shell-out to git + rsync (decision #4).

use clap::Args as ClapArgs;
use opencode_core::log;
use opencode_core::sync::{self, Change};
use std::env;
use std::path::PathBuf;
use std::process::Command;

#[derive(ClapArgs)]
pub struct Args {
    /// Target project to sync into (default: current dir).
    #[arg(long)]
    project: Option<String>,
    /// Preview only; do not write.
    #[arg(long = "dry-run")]
    dry_run: bool,
    /// Delete template files removed upstream (within managed paths).
    #[arg(long)]
    prune: bool,
    /// Override the canonical template source (git URL).
    #[arg(long)]
    template: Option<String>,
    /// Ignore `.template-sync.json`.
    #[arg(long = "no-config")]
    no_config: bool,
}

fn git(args: &[&str]) -> anyhow::Result<String> {
    let out = Command::new("git").args(args).output()?;
    if !out.status.success() {
        anyhow::bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let root: PathBuf = match &args.project {
        Some(p) => std::fs::canonicalize(p).unwrap_or_else(|_| PathBuf::from(p)),
        None => env::current_dir()?,
    };

    // sanity: must look like a template project
    if !root.exists() || !root.join("AGENTS.md").exists() {
        anyhow::bail!(
            "Không thấy \"{}\" — không giống project từ template. Dùng --project <dir>.",
            root.join("AGENTS.md").display()
        );
    }

    let cfg = sync::load_config(&root, args.no_config)?;
    let r = sync::resolve(&cfg, args.template.as_deref());

    // cache dir ~/.cache/opencode-template-sync/<name>
    let home = env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    let cache = home.join(".cache/opencode-template-sync").join(&r.name);
    std::fs::create_dir_all(cache.parent().unwrap())?;

    if !cache.join(".git").exists() {
        log::info(&format!(
            "⬇️  Clone template (lần đầu) → {}",
            cache.display()
        ));
        git(&[
            "clone",
            "--quiet",
            "--branch",
            sync::BRANCH,
            &r.repo_url,
            &cache.to_string_lossy(),
        ])?;
    } else {
        log::info("🔄 Fetch template…");
        let c = cache.to_string_lossy().to_string();
        git(&["-C", &c, "remote", "set-url", "origin", &r.repo_url])?;
        git(&["-C", &c, "fetch", "--quiet", "origin", sync::BRANCH])?;
        git(&[
            "-C",
            &c,
            "reset",
            "--hard",
            "--quiet",
            &format!("origin/{}", sync::BRANCH),
        ])?;
    }
    let c = cache.to_string_lossy().to_string();
    let sha = git(&["-C", &c, "rev-parse", "--short", "HEAD"])?;
    let when = git(&["-C", &c, "log", "-1", "--format=%cd", "--date=short"])?;
    log::info(&format!("📌 Template @ {sha} ({when}) · nguồn: {}", r.name));

    // guard: managed paths not dirty in project git
    if root.join(".git").exists() {
        let mut dirty = Vec::new();
        for p in &r.managed {
            if let Ok(out) = git(&[
                "-C",
                &root.to_string_lossy(),
                "status",
                "--porcelain",
                "--",
                p,
            ]) {
                if !out.trim().is_empty() {
                    dirty.push(p.clone());
                }
            }
        }
        if !dirty.is_empty() {
            log::info("❌ Các path template-managed đang có thay đổi CHƯA COMMIT:");
            for d in &dirty {
                log::info(&format!("   - {d}"));
            }
            anyhow::bail!("Commit hoặc stash trước rồi chạy lại (dùng --dry-run để xem trước).");
        }
    }

    // rsync each managed path
    let mut all: Vec<Change> = Vec::new();
    for p in &r.managed {
        let src_path = cache.join(p);
        if !src_path.exists() {
            log::info(&format!("⚠️  Template không có \"{p}\" — bỏ qua."));
            continue;
        }
        let is_dir = src_path.is_dir();
        let src = if is_dir {
            format!("{}/", src_path.to_string_lossy())
        } else {
            src_path.to_string_lossy().to_string()
        };
        let dst_path = root.join(p);
        let dst = if is_dir {
            format!("{}/", dst_path.to_string_lossy())
        } else {
            dst_path.to_string_lossy().to_string()
        };

        let mut a: Vec<String> = vec!["-a".into(), "--checksum".into(), "-i".into()];
        if is_dir && args.prune {
            a.push("--delete".into());
        }
        if args.dry_run {
            a.push("-n".into());
        }
        a.push(src);
        a.push(dst);

        let out = Command::new("rsync").args(&a).output()?;
        if !out.status.success() {
            anyhow::bail!(
                "rsync \"{p}\" lỗi: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        let changes = sync::parse_rsync_itemize(&String::from_utf8_lossy(&out.stdout));
        if changes.is_empty() {
            continue;
        }
        log::info(&format!("\n📦 {p}{}", if is_dir { "/" } else { "" }));
        for ch in &changes {
            match ch {
                Change::Delete(f) => log::info(&format!("   🗑  {f}")),
                Change::Add(f) => log::info(&format!("   ➕ {f}")),
                Change::Update(f) => log::info(&format!("   ✏️  {f}")),
            }
        }
        all.extend(changes);
    }

    // report-only drift
    for p in &r.report_only {
        let src = cache.join(p);
        let dst = root.join(p);
        if !src.exists() || !dst.exists() {
            continue;
        }
        let status = Command::new("diff")
            .args(["-q", &src.to_string_lossy(), &dst.to_string_lossy()])
            .status()?;
        if status.code() == Some(1) {
            log::info(&format!(
                "\n⚠️  \"{p}\" khác template — KHÔNG tự đổi. Review tay nếu cần."
            ));
        }
    }

    let s = sync::summarize(&all);
    let tag = if args.dry_run {
        "🧪 DRY-RUN —"
    } else {
        "✅"
    };
    log::info(&format!(
        "\n{tag} {} thêm · {} sửa · {} xoá",
        s.added, s.updated, s.deleted
    ));
    if !args.dry_run && root.join(".git").exists() {
        log::info("\nXem   :  git status --short && git diff");
        log::info("Undo  :  git checkout -- <path>");
    }
    Ok(())
}
