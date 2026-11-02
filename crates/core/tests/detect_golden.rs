//! Golden tests for `detect`. We build fixture project trees in a tempdir and
//! assert the detected profile JSON matches an expected snapshot.
//!
//! The Node-project expectations mirror the legacy scripts/detect-profile.mjs
//! output; the Rust-project case checks the Pha 3 extension.

use opencode_core::detect;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// Minimal unique tempdir under the OS temp dir (no external crate).
fn tmpdir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("ocrt-detect-{tag}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(root: &Path, rel: &str, contents: &str) {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(p, contents).unwrap();
}

#[test]
fn node_pnpm_project_matches_legacy_shape() {
    let root = tmpdir("node");
    write(
        &root,
        "package.json",
        r#"{"name":"demo","scripts":{"test":"vitest","lint":"eslint .","build":"vite build"}}"#,
    );
    write(&root, "pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
    fs::create_dir_all(root.join("src")).unwrap();

    let p = detect::detect(&root);
    let v: Value = serde_json::from_str(&detect::to_json(&p)).unwrap();

    assert_eq!(v["package_manager"], "pnpm");
    assert_eq!(v["lockfiles"][0], "pnpm-lock.yaml");
    assert_eq!(v["source_roots"][0], "src");
    assert_eq!(v["commands"]["test"], "pnpm test");
    assert_eq!(v["commands"]["lint"], "pnpm lint");
    assert_eq!(v["commands"]["build"], "pnpm build");
    assert_eq!(v["commands"]["install"], "pnpm install");
    assert_eq!(v["db_tool"], "none");
    assert_eq!(v["has_app"], true);
    // typecheck absent → null
    assert!(v["commands"]["typecheck"].is_null());

    fs::remove_dir_all(&root).ok();
}

#[test]
fn node_prisma_detected() {
    let root = tmpdir("prisma");
    write(
        &root,
        "package.json",
        r#"{"name":"d","scripts":{"test":"jest"}}"#,
    );
    write(&root, "package-lock.json", "{}");
    write(&root, "prisma/schema.prisma", "generator client {}\n");

    let p = detect::detect(&root);
    assert_eq!(p.package_manager.as_deref(), Some("npm"));
    assert_eq!(p.db_tool, "prisma");
    assert!(p.migration_required);
    assert_eq!(p.commands.test.as_deref(), Some("npm run test"));

    fs::remove_dir_all(&root).ok();
}

#[test]
fn no_package_json_emits_warnings() {
    let root = tmpdir("empty");
    let p = detect::detect(&root);
    assert!(p.package_manager.is_none());
    assert!(!p.warnings.is_empty());
    assert!(!p.has_app);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn rust_cargo_project_detected() {
    let root = tmpdir("rust");
    write(
        &root,
        "Cargo.toml",
        "[package]\nname=\"x\"\nversion=\"0.1.0\"\n",
    );
    write(&root, "Cargo.lock", "# lock\n");
    fs::create_dir_all(root.join("src")).unwrap();

    let p = detect::detect(&root);
    assert_eq!(p.package_manager.as_deref(), Some("cargo"));
    assert_eq!(p.lockfiles, vec!["Cargo.lock".to_string()]);
    assert_eq!(p.source_roots, vec!["src".to_string()]);
    assert_eq!(p.commands.test.as_deref(), Some("cargo test"));
    assert_eq!(
        p.commands.lint.as_deref(),
        Some("cargo clippy -- -D warnings")
    );
    assert_eq!(p.commands.build.as_deref(), Some("cargo build --release"));
    assert!(p.has_app);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn rust_workspace_crates_listed() {
    let root = tmpdir("rustws");
    write(
        &root,
        "Cargo.toml",
        "[workspace]\nmembers=[\"crates/a\",\"crates/b\"]\n",
    );
    write(&root, "crates/a/Cargo.toml", "[package]\nname=\"a\"\n");
    write(&root, "crates/b/Cargo.toml", "[package]\nname=\"b\"\n");

    let p = detect::detect(&root);
    assert_eq!(p.package_manager.as_deref(), Some("cargo"));
    assert!(p.source_roots.contains(&"crates/a".to_string()));
    assert!(p.source_roots.contains(&"crates/b".to_string()));

    fs::remove_dir_all(&root).ok();
}

#[test]
fn json_is_two_space_pretty() {
    let root = tmpdir("fmt");
    write(&root, "package.json", r#"{"name":"d"}"#);
    let p = detect::detect(&root);
    let json = detect::to_json(&p);
    // serde pretty uses 2-space indent, matching JSON.stringify(x, null, 2).
    assert!(json.contains("\n  \"has_app\""));
    fs::remove_dir_all(&root).ok();
}
