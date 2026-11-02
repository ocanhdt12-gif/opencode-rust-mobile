//! Golden tests for `sync` pure helpers: repo name, config resolve, rsync parse.

use opencode_core::sync::{self, Change, SyncConfig};

#[test]
fn repo_name_strips_git_and_path() {
    assert_eq!(
        sync::repo_name("git@github.com:org/opencode-rust-mobile.git"),
        "opencode-rust-mobile"
    );
    assert_eq!(sync::repo_name("https://github.com/org/foo/"), "foo");
    assert_eq!(sync::repo_name("https://github.com/org/bar.git"), "bar");
}

#[test]
fn resolve_precedence_flag_over_config_over_default() {
    let cfg = SyncConfig {
        managed_paths: None,
        report_only_paths: None,
        template_repo: Some("git@github.com:org/from-config.git".into()),
    };
    // flag wins
    let r = sync::resolve(&cfg, Some("git@github.com:org/from-flag.git"));
    assert_eq!(r.name, "from-flag");
    // config used when no flag
    let r2 = sync::resolve(&cfg, None);
    assert_eq!(r2.name, "from-config");
    // default managed paths present
    assert!(r2.managed.contains(&".opencode".to_string()));
    assert!(r2.managed.contains(&"AGENTS.md".to_string()));
}

#[test]
fn resolve_uses_default_template_when_empty() {
    let cfg = SyncConfig::default();
    let r = sync::resolve(&cfg, None);
    assert_eq!(r.repo_url, sync::TEMPLATE_REPO);
    assert_eq!(r.report_only, vec!["opencode.jsonc".to_string()]);
}

#[test]
fn parse_rsync_classifies_add_update_delete() {
    let stdout = "\
>f+++++++++ AGENTS.md
>f.st...... scripts/x.mjs
*deleting   skills/old/SKILL.md
some noise line
";
    let ch = sync::parse_rsync_itemize(stdout);
    assert!(ch.contains(&Change::Add("AGENTS.md".to_string())));
    assert!(ch.contains(&Change::Update("scripts/x.mjs".to_string())));
    assert!(ch.contains(&Change::Delete("skills/old/SKILL.md".to_string())));
    // lines not starting with > or * (rsync "cd+++" dir-create, plain noise)
    // are ignored, matching the legacy /^[>*]/ filter.
    assert_eq!(ch.len(), 3, "only >/* transfer/delete lines counted");
}

#[test]
fn summarize_counts() {
    let changes = vec![
        Change::Add("a".into()),
        Change::Add("b".into()),
        Change::Update("c".into()),
        Change::Delete("d".into()),
    ];
    let s = sync::summarize(&changes);
    assert_eq!(s.added, 2);
    assert_eq!(s.updated, 1);
    assert_eq!(s.deleted, 1);
}
