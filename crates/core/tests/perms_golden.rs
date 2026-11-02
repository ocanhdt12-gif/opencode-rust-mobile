//! Golden tests for `perms` — command parsing, safety classification, marker patch.

use opencode_core::perms::{self, PatchStatus, Verdict};

#[test]
fn parses_only_known_command_fields() {
    let text = "\
test_command: \"cargo test\"
lint_command: cargo clippy -- -D warnings  # comment
random_field: should-ignore
build_command: cargo build --release
install_command: null
typecheck_command: ~
";
    let cmds = perms::parse_commands(text);
    assert!(cmds.contains(&"cargo test".to_string()));
    assert!(cmds.contains(&"cargo build --release".to_string()));
    // clippy has shell meta? no. kept.
    assert!(cmds.iter().any(|c| c.starts_with("cargo clippy")));
    // null / ~ skipped
    assert!(!cmds.iter().any(|c| c == "null" || c == "~"));
    // unknown field skipped
    assert!(!cmds.iter().any(|c| c.contains("should-ignore")));
}

#[test]
fn dedups_commands() {
    let text = "test_command: cargo test\nlint_command: cargo test\n";
    let cmds = perms::parse_commands(text);
    assert_eq!(cmds.iter().filter(|c| *c == "cargo test").count(), 1);
}

#[test]
fn classify_allows_safe_cargo() {
    assert!(matches!(perms::classify("cargo test"), Verdict::Ok(_)));
    assert!(matches!(
        perms::classify("cargo build --release"),
        Verdict::Ok(_)
    ));
    assert!(matches!(
        perms::classify("cargo clippy -- -D warnings"),
        Verdict::Ok(_)
    ));
}

#[test]
fn classify_blocks_destructive_and_meta() {
    // shell meta
    assert!(matches!(
        perms::classify("cargo test && rm -rf x"),
        Verdict::Skip(_)
    ));
    assert!(matches!(
        perms::classify("echo $(whoami)"),
        Verdict::Skip(_)
    ));
    // destructive
    assert!(matches!(
        perms::classify("git push origin main"),
        Verdict::Skip(_)
    ));
    assert!(matches!(
        perms::classify("prisma db push"),
        Verdict::Skip(_)
    ));
    assert!(matches!(perms::classify("rm file"), Verdict::Skip(_)));
    assert!(matches!(perms::classify("deploy prod"), Verdict::Skip(_)));
    // wildcard
    assert!(matches!(perms::classify("*"), Verdict::Skip(_)));
    // quote
    assert!(matches!(perms::classify("echo \"hi\""), Verdict::Skip(_)));
}

#[test]
fn rm_in_other_word_not_blocked() {
    // "npm" contains no standalone rm; "warm" shouldn't trip \brm\b
    assert!(matches!(perms::classify("npm run build"), Verdict::Ok(_)));
    assert!(matches!(perms::classify("cargo run"), Verdict::Ok(_)));
}

#[test]
fn patch_replaces_marker_block() {
    let content = "\
header
# verify-commands:start
    \"old\": allow
# verify-commands:end
footer
";
    let patterns = vec![
        "cargo test".to_string(),
        "cargo build --release".to_string(),
    ];
    let (updated, st) = perms::patch_content(content, &patterns, "\n");
    assert_eq!(st, PatchStatus::WouldUpdate);
    assert!(updated.contains("    \"cargo test\": allow"));
    assert!(updated.contains("    \"cargo build --release\": allow"));
    assert!(!updated.contains("\"old\""));
    assert!(updated.contains("# verify-commands:start"));
    assert!(updated.contains("# verify-commands:end"));
    assert!(updated.contains("footer"));
}

#[test]
fn patch_idempotent() {
    let content = "\
# verify-commands:start
    \"cargo test\": allow
# verify-commands:end
";
    let patterns = vec!["cargo test".to_string()];
    let (updated, st) = perms::patch_content(content, &patterns, "\n");
    assert_eq!(
        st,
        PatchStatus::Unchanged,
        "re-applying same patterns is a no-op"
    );
    assert_eq!(updated, content);
}

#[test]
fn patch_without_markers_reports_nomarkers() {
    let content = "no markers here\n";
    let (_, st) = perms::patch_content(content, &[], "\n");
    assert_eq!(st, PatchStatus::NoMarkers);
}
