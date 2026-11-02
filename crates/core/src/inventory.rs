//! Deterministic project inventory generator (port of scripts/generate-inventory.mjs).
//!
//! Reads source roots from `.context/project-config.md` and writes
//! `docs/generated/inventory.md`. Output depends only on the repo tree + git HEAD,
//! so it is safe to run repeatedly. The generated Markdown matches the legacy
//! script's shape byte-for-byte for the same inputs.

use crate::git;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const IGNORE_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "dist",
    "build",
    "out",
    "coverage",
    ".next",
    ".turbo",
    ".cache",
    "vendor",
    "target",
];

/// Minimal profile view needed by inventory.
pub struct InventoryProfile {
    pub source_roots: Vec<String>,
    pub package_manager: String,
}

/// Parse `.context/project-config.md` for `source_roots: [...]` and
/// `package_manager: <x>`. Mirrors the regex behaviour of the legacy script.
pub fn read_profile(root: &Path) -> InventoryProfile {
    let path = root.join(".context/project-config.md");
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            return InventoryProfile {
                source_roots: vec!["src".to_string()],
                package_manager: "npm".to_string(),
            }
        }
    };

    let source_roots = parse_source_roots(&text).unwrap_or_else(|| vec!["src".to_string()]);
    let package_manager = parse_package_manager(&text).unwrap_or_else(|| "npm".to_string());

    InventoryProfile {
        source_roots,
        package_manager,
    }
}

/// Extract the bracketed list after `source_roots:` and split/trim/unquote.
fn parse_source_roots(text: &str) -> Option<Vec<String>> {
    let start = text.find("source_roots:")?;
    let after = &text[start..];
    let open = after.find('[')?;
    let close = after[open..].find(']')? + open;
    let inner = &after[open + 1..close];
    let roots: Vec<String> = inner
        .split(',')
        .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if roots.is_empty() {
        None
    } else {
        Some(roots)
    }
}

/// Extract `package_manager: <lowercase-letters>`.
fn parse_package_manager(text: &str) -> Option<String> {
    let key = "package_manager:";
    let start = text.find(key)? + key.len();
    let rest = &text[start..];
    let val: String = rest
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_lowercase())
        .collect();
    if val.is_empty() {
        None
    } else {
        Some(val)
    }
}

/// Recursively collect files under `dir`, skipping IGNORE_DIRS, visiting entries
/// in name order (localeCompare-ish: byte order on ASCII).
fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = match std::fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };
    entries.sort_by_key(|a| a.file_name());
    for entry in entries {
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy().to_string();
        if ft.is_dir() {
            if IGNORE_DIRS.contains(&name.as_str()) {
                continue;
            }
            walk(&entry.path(), out);
        } else if ft.is_file() {
            out.push(entry.path());
        }
    }
}

/// File extension incl. dot (`.rs`), or `(no-ext)` — matches node `extname`.
fn ext_of(path: &Path) -> String {
    match path.extension() {
        Some(e) => format!(".{}", e.to_string_lossy()),
        None => "(no-ext)".to_string(),
    }
}

/// Path relative to `root` using forward slashes.
fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Build the inventory Markdown. Returns (markdown, file_count).
pub fn render(root: &Path, profile: &InventoryProfile) -> (String, usize) {
    let mut files: Vec<PathBuf> = Vec::new();
    for r in &profile.source_roots {
        walk(&root.join(r), &mut files);
    }
    let mut rels: Vec<String> = files.iter().map(|f| rel(root, f)).collect();
    rels.sort();

    // By extension (count desc, then ext asc).
    let mut by_ext: BTreeMap<String, usize> = BTreeMap::new();
    for f in &files {
        *by_ext.entry(ext_of(f)).or_insert(0) += 1;
    }
    let mut ext_sorted: Vec<(String, usize)> = by_ext.into_iter().collect();
    // count desc, then ext asc — genuine compound ordering, sort_by is correct here.
    #[allow(clippy::unnecessary_sort_by)]
    ext_sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    // By directory (dir asc), files already sorted.
    let mut by_dir: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for rpath in &rels {
        let dir = match rpath.rfind('/') {
            Some(i) => rpath[..i].to_string(),
            None => ".".to_string(),
        };
        by_dir.entry(dir).or_default().push(rpath.clone());
    }

    let sha = git::short_head_or(root, "no-git");

    let mut lines: Vec<String> = Vec::new();
    lines.push("# Generated Inventory".into());
    lines.push(String::new());
    lines.push("> ⚠️ AUTO-GENERATED — **không sửa tay**. Chạy lại:".into());
    lines.push(format!(
        "> `{} docs:inventory` hoặc `opencode-tools inventory`.",
        profile.package_manager
    ));
    lines.push(String::new());
    lines.push(format!("- Git: `{sha}`"));
    lines.push(format!(
        "- Source roots: {}",
        profile
            .source_roots
            .iter()
            .map(|r| format!("`{r}`"))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    lines.push(format!("- Total files: {}", rels.len()));
    lines.push(String::new());

    lines.push("## By extension".into());
    lines.push(String::new());
    lines.push("| Ext | Count |".into());
    lines.push("|-----|-------|".into());
    for (ext, count) in &ext_sorted {
        lines.push(format!("| `{ext}` | {count} |"));
    }
    lines.push(String::new());

    lines.push("## Files".into());
    lines.push(String::new());
    if rels.is_empty() {
        lines.push("_Chưa có source files. Chạy lại sau khi code._".into());
    } else {
        for (dir, list) in &by_dir {
            lines.push(format!("### {dir}"));
            lines.push(String::new());
            for f in list {
                lines.push(format!("- `{f}`"));
            }
            lines.push(String::new());
        }
    }

    (lines.join("\n"), rels.len())
}

/// Generate and write `docs/generated/inventory.md`. Returns the relative out path.
pub fn generate(root: &Path) -> crate::Result<(String, usize)> {
    let profile = read_profile(root);
    let (md, count) = render(root, &profile);
    let out_dir = root.join("docs/generated");
    std::fs::create_dir_all(&out_dir)?;
    let out_file = out_dir.join("inventory.md");
    std::fs::write(&out_file, md)?;
    Ok((rel(root, &out_file), count))
}
