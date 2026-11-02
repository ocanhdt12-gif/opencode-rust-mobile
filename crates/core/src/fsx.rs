//! Safe filesystem helpers.
//!
//! - Atomic write (temp file + rename) so a crash never leaves a half-written file.
//! - Managed-path guard: template-sync must only ever write paths owned by the
//!   template (`AGENTS.md`, `.agent/`, `.opencode/`, `scripts/`, `skills/`), never
//!   user code/spec/docs. The guard enforces that invariant.
//!
//! Pha 2 implements these.

use crate::Result;
use std::path::Path;

/// Paths the template owns and may overwrite during sync.
pub const MANAGED_PATHS: &[&str] = &["AGENTS.md", ".agent/", ".opencode/", "scripts/", "skills/"];

/// Atomically write `contents` to `path` (temp + rename).
///
/// Pha 1: stub.
pub fn atomic_write(_path: &Path, _contents: &[u8]) -> Result<()> {
    // TODO(Pha 2): write to `<path>.tmp` then rename over `path`.
    todo!("Pha 2: atomic write")
}

/// Return true if `rel` falls under a managed path.
///
/// Pha 1: stub.
pub fn is_managed(_rel: &Path) -> bool {
    // TODO(Pha 2): prefix-match against MANAGED_PATHS.
    false
}
