//! Stdout/stderr formatting kept consistent with the legacy `.mjs` scripts.
//!
//! Agents and hooks parse tool output, so the Rust ports must print in the same
//! shape (prefixes, dry-run notices, JSON vs human `--md`). Centralise formatting
//! here so every subcommand stays consistent.

/// Print an informational line (stdout).
pub fn info(msg: &str) {
    println!("{msg}");
}

/// Print a warning line (stderr).
pub fn warn(msg: &str) {
    eprintln!("⚠️  {msg}");
}

/// Standard notice shown when a command defaults to dry-run.
pub fn dry_run_notice() {
    println!("\n(dry-run mặc định — thêm `--write` để ghi file)");
}
