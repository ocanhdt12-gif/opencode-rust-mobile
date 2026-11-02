//! Golden tests for `dispatch` pure logic: unit resolve, naming, task parse, waves.

use opencode_core::dispatch::{self, Task};

#[test]
fn resolve_unit_build_and_change() {
    let b = dispatch::resolve_unit(Some("1"), None, None).unwrap();
    assert_eq!(b.unit, "layer-1");
    assert_eq!(b.kind, "build");
    assert_eq!(b.prefix, "layer-1-task-");

    let c = dispatch::resolve_unit(None, Some("feature-x"), Some("2")).unwrap();
    assert_eq!(c.unit, "feature-x-phase-2");
    assert_eq!(c.kind, "change");
    assert_eq!(c.dir_name.as_deref(), Some("feature-x"));
    assert_eq!(c.prefix, "phase-2-task-");

    // change missing phase → error
    assert!(dispatch::resolve_unit(None, Some("bug-y"), None).is_err());
    // nothing → error
    assert!(dispatch::resolve_unit(None, None, None).is_err());
}

#[test]
fn naming_helpers() {
    assert_eq!(dispatch::task_slug("layer-1/task-03"), "layer-1__task-03");
    assert_eq!(
        dispatch::branch_name("layer-1/task-03"),
        "wt/layer-1__task-03"
    );
    assert_eq!(
        dispatch::integration_branch("layer-1"),
        "layer-1-integration"
    );
    assert_eq!(dispatch::unit_branch_prefix("layer-1"), "wt/layer-1__");
    assert_eq!(
        dispatch::worktree_rel("layer-1/task-03"),
        ".worktrees/layer-1__task-03"
    );
}

#[test]
fn unit_of_task_id_cases() {
    assert_eq!(
        dispatch::unit_of_task_id("layer-1/task-03").unwrap(),
        "layer-1"
    );
    assert_eq!(
        dispatch::unit_of_task_id("feature-x/phase-2/task-01").unwrap(),
        "feature-x-phase-2"
    );
    assert!(dispatch::unit_of_task_id("garbage").is_err());
}

#[test]
fn files_of_task_parses_section() {
    let text = "\
# Task 3

## Files to Create/Modify
- `src/a.rs`
- `src/b.rs`
* src/c.rs

## Acceptance
- something else
- `not/a/file/because/after/heading.rs`
";
    let files = dispatch::files_of_task(text);
    assert_eq!(files, vec!["src/a.rs", "src/b.rs", "src/c.rs"]);
}

#[test]
fn plan_waves_separates_file_conflicts() {
    let tasks = vec![
        Task {
            id: "t1".into(),
            file: "t1".into(),
            files: vec!["a.rs".into()],
        },
        Task {
            id: "t2".into(),
            file: "t2".into(),
            files: vec!["b.rs".into()],
        },
        Task {
            id: "t3".into(),
            file: "t3".into(),
            files: vec!["a.rs".into()],
        }, // conflicts t1
    ];
    let waves = dispatch::plan_waves(&tasks);
    // wave 1: t1 + t2 (no shared files); wave 2: t3 (shares a.rs with t1)
    assert_eq!(waves.len(), 2);
    assert_eq!(waves[0].len(), 2);
    assert_eq!(waves[0][0].id, "t1");
    assert_eq!(waves[0][1].id, "t2");
    assert_eq!(waves[1].len(), 1);
    assert_eq!(waves[1][0].id, "t3");
}

#[test]
fn worktree_belongs_to_unit_matches_prefix() {
    assert!(dispatch::worktree_belongs_to_unit(
        "/x/.worktrees/layer-1__task-03",
        "layer-1"
    ));
    assert!(!dispatch::worktree_belongs_to_unit(
        "/x/.worktrees/layer-2__task-03",
        "layer-1"
    ));
}
