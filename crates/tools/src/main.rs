//! `opencode-tools` — single multi-command binary (decision #3).
//!
//! Ports the legacy Node tooling (Tầng B) of the opencode template. Each
//! subcommand mirrors one `.mjs` script's CLI contract so the orchestration
//! layer (Tầng A) can call it without knowing the implementation changed.
//!
//! | subcommand | legacy script                        |
//! |------------|--------------------------------------|
//! | dispatch   | scripts/parallel-dispatch.mjs        |
//! | sync       | scripts/template-sync.mjs            |
//! | detect     | scripts/detect-profile.mjs          |
//! | inventory  | scripts/generate-inventory.mjs      |
//! | perms      | scripts/apply-verify-permissions.mjs |
//! | live       | .claude/hooks/live-studio.mjs        |
//!
//! Pha 1: CLI surface is complete; command bodies are stubs (exit non-zero).

mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "opencode-tools",
    version,
    about = "Rust tooling for opencode-rust-template (Tầng B).",
    long_about = "Multi-command binary that ports the opencode template Node scripts to Rust. \
Pha 1 scaffold: subcommands parse their real arguments but are not yet implemented."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run independent layer tasks in parallel (git worktree-per-task + auto-merge).
    ///
    /// Legacy: scripts/parallel-dispatch.mjs
    Dispatch(commands::dispatch::Args),

    /// Pull latest managed paths from the canonical template into a project.
    ///
    /// Legacy: scripts/template-sync.mjs
    Sync(commands::sync::Args),

    /// Detect project stack/profile and emit JSON (or `--md`) for /brainstorm.
    ///
    /// Legacy: scripts/detect-profile.mjs
    Detect(commands::detect::Args),

    /// Generate docs/generated/inventory.md from the project tree + git HEAD.
    ///
    /// Legacy: scripts/generate-inventory.mjs
    Inventory(commands::inventory::Args),

    /// Sync reviewer/spec-validator bash allow-rules with verify commands.
    ///
    /// Legacy: scripts/apply-verify-permissions.mjs
    Perms(commands::perms::Args),

    /// vibe-studio live hook: enter/leave events for agent characters.
    ///
    /// Legacy: .claude/hooks/live-studio.mjs
    Live(commands::live::Args),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Dispatch(a) => commands::dispatch::run(a),
        Command::Sync(a) => commands::sync::run(a),
        Command::Detect(a) => commands::detect::run(a),
        Command::Inventory(a) => commands::inventory::run(a),
        Command::Perms(a) => commands::perms::run(a),
        Command::Live(a) => commands::live::run(a),
    }
}
