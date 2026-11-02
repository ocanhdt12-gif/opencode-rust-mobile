# Architecture — opencode-rust-template

This condenses the approved solution (`projects/opencode-rust-template/solution.html`)
so future readers don't need the HTML.

## Goal

Recreate `opencode-project-template` with the **same flow/architecture**, changing
only the implementation language of the machine tooling (Node/TS → Rust) and the
default stack generated for child projects (React/Node → Rust).

## The two layers

- **Layer A — AI orchestration (unchanged).** Markdown/config only: `AGENTS.md`
  router, 9 agents under `.opencode/agent/`, commands `/start /change /graph /design
  /bug …`, `.agent/` workflows (loop, devops, rollback), `skills/`, `spec/`,
  `opencode.jsonc`. No programming language involved — copied verbatim in Pha 4.
- **Layer B — Machine tooling (ported).** The code that executes: 5 `scripts/*.mjs`
  plus the `.claude/hooks/live-studio.mjs` hook. Rewritten as the Rust binary
  `opencode-tools`.

Because Layer A invokes Layer B through a CLI + file I/O contract, porting B to
Rust while preserving that contract keeps the whole flow working.

## Rust layout

Single Cargo workspace:

- `crates/core` — shared library:
  - `config` — parse `.context/project-config.md`.
  - `git` — shell-out helpers (decision #4): `git rev-parse`, worktree add/merge.
  - `fsx` — atomic write + managed-path guard (only AGENTS.md, .agent/, .opencode/,
    scripts/, skills/ are writable by sync).
  - `log` — output formatting matching the legacy `.mjs` (prefixes, dry-run notice,
    JSON vs `--md`).
- `crates/tools` — binary `opencode-tools` (decision #3: one multi-command binary),
  six subcommands mapping 1:1 to the legacy scripts.

Dependencies: `clap` (derive) for the CLI, `serde`/`serde_json` for detect's JSON
output, `anyhow`/`thiserror` for errors.

## Compatibility rules (so the flow never breaks)

1. **Same CLI.** `opencode-tools detect --md` == `node scripts/detect-profile.mjs --md`.
2. **Same generated files.** Same paths and formats (inventory.md, allow-rules block
   between `# verify-commands:start/end` markers).
3. **Same stdout.** Agents/hooks parse output; formatting must match.
4. **AGENTS.md only changes the *call*.** `node scripts/x.mjs` → the Rust binary;
   logic/flow unchanged.
5. **Build step.** Template bootstrap adds `cargo build --release`; the binary goes
   on PATH (or `bin/`) so agents call it directly.

## Child-project stack (Pha 3)

- Default web = full-stack Rust: **Axum** backend + **Leptos** (WASM) frontend.
- `detect` learns Rust signals: `Cargo.toml`, `src/main.rs`, workspace members.
- Default verify commands: `cargo build`, `cargo test`, `cargo clippy -- -D warnings`,
  `cargo fmt --check`.
- New `skills/rust-guidelines/`: conventions, error handling (`thiserror`/`anyhow`),
  async (`tokio`), test layout.

## Out of scope (Pha 1)

- vibe-studio stays TypeScript (decision #1). Only its hook (`live`) is ported so it
  can emit enter/leave events; sprite scripts may be ported in optional Pha 5.
