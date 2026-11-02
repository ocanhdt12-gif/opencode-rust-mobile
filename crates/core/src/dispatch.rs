//! Parallel task dispatch via git worktree-per-task (port of scripts/parallel-dispatch.mjs).
//!
//! Model: each independent task → its own git worktree + branch. A builder runs its
//! full loop inside the worktree; a merger folds the branches back into the layer's
//! integration branch. This module holds the pure, testable logic (unit resolution,
//! slug/branch naming, wave planning, task-file parsing). The git side-effects live
//! in the `tools` command (shell-out, decision #4).

/// A work unit: either an initial-build layer or a change phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// e.g. `layer-1` or `feature-x-phase-2`.
    pub unit: String,
    /// "build" or "change".
    pub kind: String,
    /// For change mode, the scope dir to scan (e.g. `feature-x`); None = scan all.
    pub dir_name: Option<String>,
    /// Task-file prefix, e.g. `layer-1-task-` or `phase-2-task-`.
    pub prefix: String,
}

/// Resolve the work unit from layer or scope+phase.
///
/// - `--layer N` → build, unit `layer-N`.
/// - `--scope <s> --phase N` → change, unit `<s>-phase-N`.
pub fn resolve_unit(
    layer: Option<&str>,
    scope: Option<&str>,
    phase: Option<&str>,
) -> crate::Result<Unit> {
    if let Some(scope) = scope {
        let phase = phase.ok_or_else(|| {
            anyhow::anyhow!("change mode cần cả --scope <feature|bug>-<slug> VÀ --phase <N>")
        })?;
        return Ok(Unit {
            unit: format!("{scope}-phase-{phase}"),
            kind: "change".into(),
            dir_name: Some(scope.to_string()),
            prefix: format!("phase-{phase}-task-"),
        });
    }
    let layer = layer.ok_or_else(|| {
        anyhow::anyhow!("cần --layer <N> (initial build) HOẶC --scope <...> --phase <N> (change)")
    })?;
    Ok(Unit {
        unit: format!("layer-{layer}"),
        kind: "build".into(),
        dir_name: None,
        prefix: format!("layer-{layer}-task-"),
    })
}

/// task id → slug safe for branch/worktree (`layer-1/task-03` → `layer-1__task-03`).
pub fn task_slug(task_id: &str) -> String {
    task_id.replace('/', "__")
}

/// Branch name for a task (`wt/<slug>`).
pub fn branch_name(task_id: &str) -> String {
    format!("wt/{}", task_slug(task_id))
}

/// Integration branch for a unit (`<unit>-integration`).
pub fn integration_branch(unit: &str) -> String {
    format!("{unit}-integration")
}

/// Branch/worktree prefix used to filter a unit's task branches (`wt/<unit>__`).
pub fn unit_branch_prefix(unit: &str) -> String {
    format!("wt/{}__", unit.replace('/', "__"))
}

/// Resolve the unit name from a task id.
/// `layer-1/task-03` → `layer-1`; `feature-x/phase-1/task-03` → `feature-x-phase-1`.
pub fn unit_of_task_id(task_id: &str) -> crate::Result<String> {
    if let Some(rest) = task_id.strip_prefix("layer-") {
        if let Some(slash) = rest.find('/') {
            return Ok(format!("layer-{}", &rest[..slash]));
        }
    }
    // <scope>/phase-<N>/...
    if let Some(phase_pos) = task_id.find("/phase-") {
        let scope = &task_id[..phase_pos];
        let after = &task_id[phase_pos + "/phase-".len()..];
        if let Some(slash) = after.find('/') {
            let n = &after[..slash];
            return Ok(format!("{scope}-phase-{n}"));
        }
    }
    anyhow::bail!(
        "taskId không hợp lệ: {task_id} (dạng \"layer-N/task-NN\" hoặc \"<scope>/phase-N/task-NN\")"
    )
}

/// A task discovered under `tasks/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub file: String,
    pub files: Vec<String>,
}

/// Extract paths under a task file's `## Files to Create/Modify` section.
pub fn files_of_task(text: &str) -> Vec<String> {
    let mut files = Vec::new();
    let mut in_section = false;
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with('#') {
            // A heading line: enter the section on the target heading, else leave it.
            in_section = t.to_lowercase().starts_with("## files to create/modify");
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some(p) = parse_bullet_path(line) {
            if !p.starts_with('#') {
                files.push(p);
            }
        }
    }
    files
}

/// Parse a `- `path`` or `* path` bullet into a path, stripping backticks.
fn parse_bullet_path(line: &str) -> Option<String> {
    let t = line.trim_start();
    let rest = t.strip_prefix("- ").or_else(|| t.strip_prefix("* "))?;
    let rest = rest.trim();
    let token: String = rest
        .trim_start_matches('`')
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != '`')
        .collect();
    let token = token.trim_matches('`').trim().to_string();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

/// Split tasks into waves: tasks sharing a file never land in the same wave.
/// Preserves input order; mirrors the greedy algorithm of the legacy script.
pub fn plan_waves(tasks: &[Task]) -> Vec<Vec<Task>> {
    let mut waves: Vec<Vec<Task>> = Vec::new();
    let mut remaining: Vec<Task> = tasks.to_vec();
    while !remaining.is_empty() {
        let mut wave: Vec<Task> = Vec::new();
        let mut claimed: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut i = 0;
        while i < remaining.len() {
            let conflict = remaining[i].files.iter().any(|f| claimed.contains(f));
            if !conflict {
                for f in &remaining[i].files {
                    claimed.insert(f.clone());
                }
                wave.push(remaining.remove(i));
            } else {
                i += 1;
            }
        }
        if wave.is_empty() {
            // all remaining mutually conflict → take one to avoid a stall
            wave.push(remaining.remove(0));
        }
        waves.push(wave);
    }
    waves
}

/// Worktree path for a task relative to root (`.worktrees/<slug>`).
pub fn worktree_rel(task_id: &str) -> String {
    format!(".worktrees/{}", task_slug(task_id))
}

/// True if a worktree-list porcelain path belongs to this unit.
pub fn worktree_belongs_to_unit(worktree_path: &str, unit: &str) -> bool {
    let needle = format!(".worktrees/{}__", unit.replace('/', "__"));
    worktree_path.contains(&needle)
}
