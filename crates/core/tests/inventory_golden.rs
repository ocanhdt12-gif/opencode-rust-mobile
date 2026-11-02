//! Golden tests for `inventory`. Build a fixture tree, render, assert the
//! Markdown shape matches the legacy generate-inventory.mjs output.

use opencode_core::inventory::{self, InventoryProfile};
use std::fs;
use std::path::{Path, PathBuf};

fn tmpdir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("ocrt-inv-{tag}-{nanos}"));
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
fn renders_header_counts_and_grouping() {
    let root = tmpdir("basic");
    write(&root, "src/main.rs", "fn main() {}\n");
    write(&root, "src/lib.rs", "// lib\n");
    write(&root, "src/util/helpers.rs", "// helpers\n");
    write(&root, "src/README.md", "# readme\n");
    // ignored dir must be skipped
    write(&root, "src/target/junk.rs", "// skip me\n");

    let profile = InventoryProfile {
        source_roots: vec!["src".into()],
        package_manager: "cargo".into(),
    };
    let (md, count) = inventory::render(&root, &profile);

    assert_eq!(count, 4, "target/ skipped → 4 files");
    assert!(md.starts_with("# Generated Inventory"));
    assert!(md.contains("> ⚠️ AUTO-GENERATED"));
    assert!(md.contains("`cargo docs:inventory`"));
    assert!(md.contains("- Total files: 4"));
    assert!(md.contains("- Source roots: `src`"));

    // By extension: .rs(3) ranks above .md(1)
    let ext_tbl = md.split("## By extension").nth(1).unwrap();
    let rs_pos = ext_tbl.find("| `.rs` | 3 |").expect(".rs row");
    let md_pos = ext_tbl.find("| `.md` | 1 |").expect(".md row");
    assert!(
        rs_pos < md_pos,
        ".rs (count 3) must appear before .md (count 1)"
    );

    // Files grouped by dir heading
    assert!(md.contains("### src"));
    assert!(md.contains("### src/util"));
    assert!(md.contains("- `src/main.rs`"));
    assert!(md.contains("- `src/util/helpers.rs`"));
    assert!(!md.contains("junk.rs"), "ignored dir file must not appear");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn empty_source_roots_show_placeholder() {
    let root = tmpdir("empty");
    let profile = InventoryProfile {
        source_roots: vec!["src".into()],
        package_manager: "npm".into(),
    };
    let (md, count) = inventory::render(&root, &profile);
    assert_eq!(count, 0);
    assert!(md.contains("_Chưa có source files. Chạy lại sau khi code._"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn reads_source_roots_from_project_config() {
    let root = tmpdir("cfg");
    write(
        &root,
        ".context/project-config.md",
        "# config\nsource_roots: [\"crates/a\", \"crates/b\"]\npackage_manager: cargo\n",
    );
    write(&root, "crates/a/main.rs", "fn main() {}\n");
    write(&root, "crates/b/lib.rs", "// b\n");

    let profile = inventory::read_profile(&root);
    assert_eq!(
        profile.source_roots,
        vec!["crates/a".to_string(), "crates/b".to_string()]
    );
    assert_eq!(profile.package_manager, "cargo");

    let (_, count) = inventory::render(&root, &profile);
    assert_eq!(count, 2);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn missing_config_defaults_to_src_npm() {
    let root = tmpdir("nocfg");
    let profile = inventory::read_profile(&root);
    assert_eq!(profile.source_roots, vec!["src".to_string()]);
    assert_eq!(profile.package_manager, "npm");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn generate_writes_file() {
    let root = tmpdir("write");
    write(&root, "src/a.rs", "// a\n");
    let (out, count) = inventory::generate(&root).unwrap();
    assert_eq!(out, "docs/generated/inventory.md");
    assert_eq!(count, 1);
    let written = fs::read_to_string(root.join("docs/generated/inventory.md")).unwrap();
    assert!(written.contains("# Generated Inventory"));
    fs::remove_dir_all(&root).ok();
}
