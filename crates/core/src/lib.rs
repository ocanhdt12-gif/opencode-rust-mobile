//! # opencode-core
//!
//! Shared library for the `opencode-rust-template` tooling (Tầng B — machine tooling).
//!
//! This crate holds helpers reused by every `opencode-tools` subcommand. In Pha 1
//! (scaffold) the modules expose the intended API surface with documented stubs;
//! real logic is ported from the legacy Node `.mjs` scripts in Pha 2.
//!
//! ## Module map
//! - [`config`]  — parse `.context/project-config.md` (key:value blocks).
//! - [`git`]     — git helpers via **shell-out** (no libgit2), matching legacy behaviour.
//! - [`fsx`]     — safe filesystem ops (atomic write, managed-path guard).
//! - [`log`]     — stdout/stderr formatting kept consistent with the old `.mjs` output.

pub mod config;
pub mod detect;
pub mod dispatch;
pub mod fsx;
pub mod git;
pub mod inventory;
pub mod live;
pub mod log;
pub mod perms;
pub mod sync;

/// Crate-wide result alias.
pub type Result<T> = anyhow::Result<T>;
