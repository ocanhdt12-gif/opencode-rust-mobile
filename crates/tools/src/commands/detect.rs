//! `detect` — port of scripts/detect-profile.mjs.
//!
//! Legacy CLI: default prints JSON; `--md` prints human-readable. Read-only
//! profile detector consumed by /brainstorm. In the Rust template this also
//! learns Rust signals (Cargo.toml, src/main.rs) — implemented in Pha 2/3.

use clap::Args as ClapArgs;
use opencode_core::detect;
use std::env;

#[derive(ClapArgs)]
pub struct Args {
    /// Emit human-readable Markdown instead of JSON.
    #[arg(long)]
    md: bool,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let profile = detect::detect(&root);
    if args.md {
        println!("{}", detect::to_markdown(&profile));
    } else {
        println!("{}", detect::to_json(&profile));
    }
    Ok(())
}
