//! Parse `.context/project-config.md`.
//!
//! The legacy template stores project profile values (branch, package manager,
//! verify commands, DB, models, deploy target…) inside `.context/project-config.md`
//! as simple `key: value` blocks. `/brainstorm` writes it; tooling reads it.
//!
//! Pha 2 will implement real parsing. For now this is a documented stub.

use crate::Result;
use std::collections::BTreeMap;
use std::path::Path;

/// Parsed project configuration (flat key → value for Pha 1).
#[derive(Debug, Default, Clone)]
pub struct ProjectConfig {
    pub values: BTreeMap<String, String>,
}

impl ProjectConfig {
    /// Read and parse `.context/project-config.md` at `root`.
    ///
    /// Pha 1: stub — returns an empty config. Pha 2: real parser.
    pub fn load(_root: &Path) -> Result<Self> {
        // TODO(Pha 2): port parsing from scripts/detect-profile.mjs + project-config format.
        Ok(Self::default())
    }
}
