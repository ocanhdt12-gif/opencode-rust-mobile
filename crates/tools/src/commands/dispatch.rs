//! `dispatch` — port of scripts/parallel-dispatch.mjs.
//!
//! Subcommands: plan | setup | merge | cleanup | status. Pure planning lives in
//! core::dispatch; git side-effects are shell-out here (decision #4).

use clap::{Args as ClapArgs, Subcommand};
use opencode_core::dispatch::{self, Task};
use serde_json::json;
use std::env;
use std::path::Path;
use std::process::Command;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Plan parallel waves for a layer (or change phase).
    Plan {
        #[arg(long)]
        layer: Option<String>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        phase: Option<String>,
    },
    /// Set up a worktree + branch for one task.
    Setup {
        #[arg(long)]
        task: String,
        #[arg(long)]
        base: Option<String>,
    },
    /// Merge task branches into the integration branch.
    Merge {
        #[arg(long)]
        layer: Option<String>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        phase: Option<String>,
        #[arg(long = "no-verify")]
        no_verify: bool,
    },
    /// Remove worktrees/branches for a finished unit.
    Cleanup {
        #[arg(long)]
        layer: Option<String>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        phase: Option<String>,
    },
    /// List worktrees + wt/ branches.
    Status,
}

fn git_try(root: &Path, args: &[&str]) -> (bool, String) {
    match Command::new("git").current_dir(root).args(args).output() {
        Ok(o) if o.status.success() => {
            (true, String::from_utf8_lossy(&o.stdout).trim().to_string())
        }
        Ok(o) => (false, String::from_utf8_lossy(&o.stderr).trim().to_string()),
        Err(e) => (false, e.to_string()),
    }
}

fn find_unit_tasks(root: &Path, u: &dispatch::Unit) -> Vec<Task> {
    let tasks_root = root.join("tasks");
    if !tasks_root.is_dir() {
        return Vec::new();
    }
    let slugs: Vec<String> = match &u.dir_name {
        Some(d) => vec![d.clone()],
        None => std::fs::read_dir(&tasks_root)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default(),
    };
    let mut results: Vec<Task> = Vec::new();
    for slug in slugs {
        let dir = tasks_root.join(&slug);
        if !dir.is_dir() {
            continue;
        }
        let mut entries: Vec<String> = std::fs::read_dir(&dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default();
        entries.sort();
        for f in entries {
            if f.starts_with(&u.prefix) && f.ends_with(".md") {
                let tnum = f
                    .split("task-")
                    .nth(1)
                    .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
                    .unwrap_or("??")
                    .to_string();
                let id = if u.kind == "change" {
                    // u.unit = "<scope>-phase-<N>"
                    let (scope, phase) = u
                        .unit
                        .rsplit_once("-phase-")
                        .unwrap_or((u.unit.as_str(), "?"));
                    format!("{scope}/phase-{phase}/task-{tnum}")
                } else {
                    let layer = u.unit.strip_prefix("layer-").unwrap_or(&u.unit);
                    format!("layer-{layer}/task-{tnum}")
                };
                let file = dir.join(&f);
                let text = std::fs::read_to_string(&file).unwrap_or_default();
                let files = dispatch::files_of_task(&text);
                results.push(Task {
                    id,
                    file: file
                        .strip_prefix(root)
                        .unwrap_or(&file)
                        .to_string_lossy()
                        .replace('\\', "/"),
                    files,
                });
            }
        }
    }
    results.sort_by(|a, b| a.file.cmp(&b.file));
    results
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    match args.action {
        Action::Plan {
            layer,
            scope,
            phase,
        } => {
            let u = dispatch::resolve_unit(layer.as_deref(), scope.as_deref(), phase.as_deref())?;
            let tasks = find_unit_tasks(&root, &u);
            let waves = dispatch::plan_waves(&tasks);
            let waves_json: Vec<Vec<_>> = waves
                .iter()
                .map(|w| {
                    w.iter()
                        .map(|t| json!({"id": t.id, "file": t.file, "files": t.files}))
                        .collect()
                })
                .collect();
            let out = json!({
                "unit": u.unit,
                "kind": u.kind,
                "integrationBranch": dispatch::integration_branch(&u.unit),
                "waves": waves_json,
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        Action::Setup { task, base } => {
            let unit = dispatch::unit_of_task_id(&task)?;
            let base = base.unwrap_or_else(|| {
                let (ok, out) = git_try(&root, &["rev-parse", "--abbrev-ref", "HEAD"]);
                if ok {
                    out
                } else {
                    "HEAD".into()
                }
            });
            let ib = dispatch::integration_branch(&unit);
            // ensure integration branch
            let exists = git_try(
                &root,
                &[
                    "show-ref",
                    "--verify",
                    "--quiet",
                    &format!("refs/heads/{ib}"),
                ],
            )
            .0;
            if !exists {
                let (ok, err) = git_try(&root, &["branch", &ib, &base]);
                if !ok {
                    anyhow::bail!("tạo integration branch lỗi: {err}");
                }
            }
            let wt = dispatch::worktree_rel(&task);
            let br = dispatch::branch_name(&task);
            if root.join(&wt).exists() {
                anyhow::bail!("worktree đã tồn tại: {wt} (chạy cleanup trước nếu muốn làm lại)");
            }
            std::fs::create_dir_all(root.join(".worktrees"))?;
            let (ok, err) = git_try(&root, &["worktree", "add", "-b", &br, &wt, &ib]);
            if !ok {
                anyhow::bail!("tạo worktree lỗi: {err}");
            }
            let out = json!({"task": task, "unit": unit, "worktree": wt, "branch": br, "base": ib});
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        Action::Merge {
            layer,
            scope,
            phase,
            no_verify,
        } => {
            let u = dispatch::resolve_unit(layer.as_deref(), scope.as_deref(), phase.as_deref())?;
            let ib = dispatch::integration_branch(&u.unit);
            if !git_try(
                &root,
                &[
                    "show-ref",
                    "--verify",
                    "--quiet",
                    &format!("refs/heads/{ib}"),
                ],
            )
            .0
            {
                anyhow::bail!("chưa có integration branch {ib} — chạy setup trước");
            }
            let (ok, err) = git_try(&root, &["checkout", &ib]);
            if !ok {
                anyhow::bail!("checkout {ib} lỗi: {err}");
            }
            let prefix = dispatch::unit_branch_prefix(&u.unit);
            let (_, listed) = git_try(
                &root,
                &[
                    "for-each-ref",
                    "--format=%(refname:short)",
                    "refs/heads/wt/",
                ],
            );
            let branches: Vec<String> = listed
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|b| b.starts_with(&prefix))
                .collect();
            let mut merged = Vec::new();
            for b in &branches {
                let msg = format!("merge({b}): {} parallel task", u.unit);
                let (ok, err) = git_try(&root, &["merge", "--no-ff", "-m", &msg, b]);
                if !ok {
                    git_try(&root, &["merge", "--abort"]);
                    anyhow::bail!("CONFLICT khi merge {b} — DỪNG. Xử lý thủ công.\n{err}");
                }
                merged.push(b.clone());
            }
            let mut out =
                json!({"unit": u.unit, "kind": u.kind, "integrationBranch": ib, "merged": merged});
            if !no_verify {
                out["verifyHint"] = json!(
                    "Chạy verify tổng thể (build/test/lint) theo .context/project-config.md."
                );
            }
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        Action::Cleanup {
            layer,
            scope,
            phase,
        } => {
            let u = dispatch::resolve_unit(layer.as_deref(), scope.as_deref(), phase.as_deref())?;
            let (_, list) = git_try(&root, &["worktree", "list", "--porcelain"]);
            let mut removed = Vec::new();
            for block in list.split("\n\n") {
                if let Some(line) = block.lines().find(|l| l.starts_with("worktree ")) {
                    let p = &line["worktree ".len()..];
                    if dispatch::worktree_belongs_to_unit(p, &u.unit) {
                        git_try(&root, &["worktree", "remove", "--force", p]);
                        removed.push(p.to_string());
                    }
                }
            }
            let prefix = dispatch::unit_branch_prefix(&u.unit);
            let (_, listed) = git_try(
                &root,
                &[
                    "for-each-ref",
                    "--format=%(refname:short)",
                    "refs/heads/wt/",
                ],
            );
            for b in listed
                .lines()
                .map(|s| s.trim())
                .filter(|b| b.starts_with(&prefix))
            {
                git_try(&root, &["branch", "-D", b]);
                removed.push(b.to_string());
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({"unit": u.unit, "removed": removed}))?
            );
        }
        Action::Status => {
            let (_, wt) = git_try(&root, &["worktree", "list"]);
            let (_, br) = git_try(
                &root,
                &[
                    "for-each-ref",
                    "--format=%(refname:short)",
                    "refs/heads/wt/",
                ],
            );
            println!(
                "=== worktrees ===\n{wt}\n\n=== wt/ branches ===\n{}",
                if br.is_empty() { "(none)".into() } else { br }
            );
        }
    }
    Ok(())
}
