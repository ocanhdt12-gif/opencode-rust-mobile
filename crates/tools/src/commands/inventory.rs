//! `inventory` — port of scripts/generate-inventory.mjs.
//!
//! Legacy CLI: no flags. Reads source roots from `.context/project-config.md`
//! and writes `docs/generated/inventory.md`. Deterministic; depends only on the
//! repo tree + git HEAD.

use clap::Args as ClapArgs;
use opencode_core::inventory;
use std::env;

#[derive(ClapArgs)]
pub struct Args {}

pub fn run(_args: Args) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let (out_path, count) = inventory::generate(&root)?;
    println!("Wrote {out_path} ({count} files)");
    Ok(())
}
