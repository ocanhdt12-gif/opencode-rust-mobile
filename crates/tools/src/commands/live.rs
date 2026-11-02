//! `live` — port of .claude/hooks/live-studio.mjs.
//!
//! Maps an opencode lifecycle event to a vibe-studio enter/leave action and prints
//! it as `<action> <character-key>`. vibe-studio stays TypeScript (decision #1).

use clap::Args as ClapArgs;
use opencode_core::live;

#[derive(ClapArgs)]
pub struct Args {
    /// Lifecycle event, e.g. UserPromptSubmit / Stop / PreToolUse / SubagentStop.
    event: String,
    /// Tool name for PreToolUse/PostToolUse (e.g. Agent, SendMessage).
    #[arg(long)]
    tool: Option<String>,
    /// Stable agent id for the character key (default: "default").
    #[arg(long)]
    agent: Option<String>,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    match live::map_event(&args.event, args.tool.as_deref()) {
        Some(action) => {
            let key = live::character_key(args.agent.as_deref().unwrap_or("default"));
            println!("{} {}", action.as_str(), key);
            Ok(())
        }
        None => {
            // Unmapped events are a no-op (hook stays silent), exit 0.
            Ok(())
        }
    }
}
