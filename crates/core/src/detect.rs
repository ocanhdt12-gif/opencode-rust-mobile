//! Deterministic project profile detector (port of scripts/detect-profile.mjs).
//!
//! Reads the repo tree (no mutation) and produces suggested values for
//! `.context/project-config.md`. `/brainstorm` consumes the JSON and asks the user
//! to confirm.
//!
//! Compatibility: on a Node project the JSON output matches the legacy `.mjs`
//! byte-for-byte (same field order, same `serde_json` pretty = 2-space indent).
//! This crate additionally detects a **Rust** stack (Cargo.toml) — the Pha 3 goal —
//! exposed through the same schema (package_manager = "cargo", cargo commands).

use serde::Serialize;
use serde_json::{Map, Value};
use std::path::Path;

const LOCKFILES: &[(&str, &str)] = &[
    ("pnpm-lock.yaml", "pnpm"),
    ("yarn.lock", "yarn"),
    ("bun.lockb", "bun"),
    ("bun.lock", "bun"),
    ("package-lock.json", "npm"),
];

const SOURCE_CANDIDATES: &[&str] = &[
    "src", "app", "apps", "packages", "lib", "server", "client", "web", "api",
];
const MONOREPO_DIRS: &[&str] = &["apps", "packages"];

/// Detected commands block.
#[derive(Debug, Serialize, Default)]
pub struct Commands {
    pub install: Option<String>,
    pub test: Option<String>,
    pub lint: Option<String>,
    pub typecheck: Option<String>,
    pub build: Option<String>,
}

/// Full detection result. Field order matches the legacy JSON exactly.
#[derive(Debug, Serialize)]
pub struct Profile {
    pub has_app: bool,
    pub package_manager: Option<String>,
    pub lockfiles: Vec<String>,
    pub source_roots: Vec<String>,
    /// Raw `scripts` object from package.json (empty for non-Node).
    pub package_scripts: Value,
    pub commands: Commands,
    pub db_tool: String,
    pub migration_required: bool,
    pub warnings: Vec<String>,
}

fn exists(root: &Path, rel: &str) -> bool {
    root.join(rel).exists()
}

fn exists_dir(path: &Path) -> bool {
    path.is_dir()
}

fn list_subdirs(path: &Path) -> Vec<String> {
    let mut out: Vec<String> = match std::fs::read_dir(path) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .filter_map(|e| e.file_name().into_string().ok())
            .collect(),
        Err(_) => return Vec::new(),
    };
    out.sort();
    out
}

fn read_json(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn detect_package_manager(root: &Path, pkg: Option<&Value>) -> (Option<String>, Vec<String>) {
    let mut lockfiles = Vec::new();
    let mut manager: Option<String> = None;
    for (file, name) in LOCKFILES {
        if exists(root, file) {
            lockfiles.push((*file).to_string());
            if manager.is_none() {
                manager = Some((*name).to_string());
            }
        }
    }
    if manager.is_none() {
        if let Some(Value::String(pm)) = pkg.and_then(|p| p.get("packageManager")) {
            let name = pm.split('@').next().unwrap_or("");
            if !name.is_empty() {
                manager = Some(name.to_string());
            }
        }
    }
    (manager, lockfiles)
}

fn detect_source_roots(root: &Path) -> Vec<String> {
    let mut roots = Vec::new();
    for dir in SOURCE_CANDIDATES {
        if !exists_dir(&root.join(dir)) {
            continue;
        }
        if MONOREPO_DIRS.contains(dir) {
            let children: Vec<String> = list_subdirs(&root.join(dir))
                .into_iter()
                .map(|c| format!("{dir}/{c}"))
                .collect();
            if !children.is_empty() {
                roots.extend(children);
                continue;
            }
        }
        roots.push((*dir).to_string());
    }
    roots
}

fn run_command(pm: Option<&str>, script: &str) -> Option<String> {
    let pm = pm?;
    Some(match pm {
        "pnpm" => format!("pnpm {script}"),
        "yarn" => format!("yarn {script}"),
        "bun" => format!("bun run {script}"),
        _ => format!("npm run {script}"),
    })
}

fn detect_commands(pm: Option<&str>, pkg: Option<&Value>) -> Commands {
    let scripts = pkg
        .and_then(|p| p.get("scripts"))
        .cloned()
        .unwrap_or(Value::Null);
    let has = |name: &str| -> bool { scripts.get(name).and_then(|v| v.as_str()).is_some() };
    let pick =
        |names: &[&str]| -> Option<String> { names.iter().find(|n| has(n)).map(|s| s.to_string()) };
    let test = pick(&["test"]);
    let lint = pick(&["lint"]);
    let typecheck = pick(&["typecheck", "type-check", "tsc"]);
    let build = pick(&["build"]);
    Commands {
        install: pm.map(|p| format!("{p} install")),
        test: test.and_then(|s| run_command(pm, &s)),
        lint: lint.and_then(|s| run_command(pm, &s)),
        typecheck: typecheck.and_then(|s| run_command(pm, &s)),
        build: build.and_then(|s| run_command(pm, &s)),
    }
}

fn detect_db(root: &Path) -> (String, bool) {
    if exists(root, "prisma/schema.prisma") {
        return ("prisma".into(), true);
    }
    for f in [
        "drizzle.config.ts",
        "drizzle.config.js",
        "drizzle.config.mjs",
    ] {
        if exists(root, f) {
            return ("drizzle".into(), true);
        }
    }
    ("none".into(), false)
}

/// Rust-stack detection (Pha 3): recognise a Cargo project and map it onto the
/// same schema so `/brainstorm` treats it uniformly.
fn detect_rust(root: &Path) -> Option<Profile> {
    if !exists(root, "Cargo.toml") {
        return None;
    }
    let mut source_roots = Vec::new();
    if exists_dir(&root.join("src")) {
        source_roots.push("src".to_string());
    }
    for member in ["crates", "libs"] {
        if exists_dir(&root.join(member)) {
            for c in list_subdirs(&root.join(member)) {
                source_roots.push(format!("{member}/{c}"));
            }
        }
    }
    let dioxus = is_dioxus(root);
    let commands = if dioxus {
        Commands {
            install: Some("cargo fetch".into()),
            test: Some("cargo test".into()),
            lint: Some("cargo clippy --all-targets -- -D warnings".into()),
            typecheck: Some("cargo check".into()),
            // dx = dioxus CLI; build bundle cho mobile/desktop
            build: Some("dx build --release".into()),
        }
    } else {
        Commands {
            install: Some("cargo fetch".into()),
            test: Some("cargo test".into()),
            lint: Some("cargo clippy -- -D warnings".into()),
            typecheck: Some("cargo check".into()),
            build: Some("cargo build --release".into()),
        }
    };
    let mut warnings = Vec::new();
    if dioxus {
        warnings.push(
            "Phát hiện Dioxus (mobile/multiplatform). Build iOS/Android cần `dx` CLI + Xcode/Android SDK — xem skills/rust-mobile-guidelines.".into(),
        );
    }
    if source_roots.is_empty() {
        warnings.push(
            "Cargo.toml có nhưng không thấy src/ hoặc crates/* — kiểm tra workspace layout.".into(),
        );
    }
    Some(Profile {
        has_app: true,
        package_manager: Some("cargo".into()),
        lockfiles: if exists(root, "Cargo.lock") {
            vec!["Cargo.lock".into()]
        } else {
            vec![]
        },
        source_roots,
        package_scripts: Value::Object(Map::new()),
        commands,
        db_tool: "none".into(),
        migration_required: false,
        warnings,
    })
}

/// True nếu project dùng Dioxus (mobile/multiplatform UI full-Rust).
fn is_dioxus(root: &Path) -> bool {
    if exists(root, "Dioxus.toml") {
        return true;
    }
    // dependency dioxus trong Cargo.toml bất kỳ (root hoặc workspace member)
    let mut cargo_texts = Vec::new();
    if let Ok(t) = std::fs::read_to_string(root.join("Cargo.toml")) {
        cargo_texts.push(t);
    }
    for member in ["crates", "libs"] {
        for c in list_subdirs(&root.join(member)) {
            if let Ok(t) = std::fs::read_to_string(root.join(member).join(&c).join("Cargo.toml")) {
                cargo_texts.push(t);
            }
        }
    }
    cargo_texts.iter().any(|t| {
        t.lines().any(|l| {
            let l = l.trim_start();
            l.starts_with("dioxus ")
                || l.starts_with("dioxus=")
                || l.starts_with("dioxus-")
                || l.contains("dioxus = ")
        })
    })
}

/// Detect the project profile rooted at `root`.
///
/// A Rust project (Cargo.toml) takes precedence; otherwise falls back to the
/// Node detector that mirrors the legacy script.
pub fn detect(root: &Path) -> Profile {
    if let Some(rust) = detect_rust(root) {
        return rust;
    }

    let pkg = read_json(&root.join("package.json"));
    let (package_manager, lockfiles) = detect_package_manager(root, pkg.as_ref());
    let source_roots = detect_source_roots(root);
    let scripts = pkg.as_ref().and_then(|p| p.get("scripts")).cloned();
    let commands = detect_commands(package_manager.as_deref(), pkg.as_ref());
    let (db_tool, migration_required) = detect_db(root);

    let mut warnings = Vec::new();
    if pkg.is_none() {
        warnings.push(
            "Không tìm thấy package.json — có thể repo không phải Node, hoặc package nằm trong subdir.".into(),
        );
    }
    if package_manager.is_none() {
        warnings.push(
            "Không detect được package manager (thiếu lockfile + packageManager field).".into(),
        );
    }
    let scripts_empty = scripts
        .as_ref()
        .and_then(|s| s.as_object())
        .map(|o| o.is_empty())
        .unwrap_or(true);
    if scripts_empty {
        warnings.push("package.json không có scripts — verify commands phải khai tay.".into());
    }

    let has_app = pkg.is_some() || !source_roots.is_empty();
    let package_scripts = scripts.unwrap_or(Value::Object(Map::new()));

    Profile {
        has_app,
        package_manager,
        lockfiles,
        source_roots,
        package_scripts,
        commands,
        db_tool,
        migration_required,
        warnings,
    }
}

/// Render the human-readable Markdown form (`--md`), matching the legacy script.
pub fn to_markdown(p: &Profile) -> String {
    fn line(k: &str, v: &str) -> String {
        if v.is_empty() {
            format!("- {k}: _(không detect được)_")
        } else {
            format!("- {k}: `{v}`")
        }
    }
    fn opt(v: &Option<String>) -> String {
        v.clone().unwrap_or_default()
    }
    let mut out = vec![
        "# Detected project profile".to_string(),
        String::new(),
        line("has_app", &p.has_app.to_string()),
        line("package_manager", &opt(&p.package_manager)),
        line("lockfiles", &p.lockfiles.join(", ")),
        line("source_roots", &p.source_roots.join(", ")),
        line("test_command", &opt(&p.commands.test)),
        line("lint_command", &opt(&p.commands.lint)),
        line("typecheck_command", &opt(&p.commands.typecheck)),
        line("build_command", &opt(&p.commands.build)),
        line("install_command", &opt(&p.commands.install)),
        line("db_tool", &p.db_tool),
        line("migration_required", &p.migration_required.to_string()),
    ];
    if !p.warnings.is_empty() {
        out.push(String::new());
        out.push("## Warnings".to_string());
        for w in &p.warnings {
            out.push(format!("- {w}"));
        }
    }
    out.join("\n")
}

/// Serialize to the pretty JSON form matching the legacy script (2-space indent).
pub fn to_json(p: &Profile) -> String {
    serde_json::to_string_pretty(p).expect("Profile serializes")
}
