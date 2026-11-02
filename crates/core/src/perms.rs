//! Sync reviewer/spec-validator bash allow-rules with verify commands
//! (port of scripts/apply-verify-permissions.mjs).
//!
//! Edits only the block between `# verify-commands:start` / `# verify-commands:end`
//! markers inside `.opencode/agent/reviewer.md` and `.opencode/agent/spec-validator.md`.
//! Idempotent. Safety: only auto-allows commands that are safe to serialize in YAML
//! and do NOT look destructive. Generated patterns are EXACT (no trailing wildcard)
//! so `cmd && destructive` cannot ride along.

use std::path::{Path, PathBuf};

pub const START: &str = "# verify-commands:start";
pub const END: &str = "# verify-commands:end";

const COMMAND_FIELDS: &[&str] = &[
    "web_typecheck_command",
    "web_lint_command",
    "api_typecheck_command",
    "api_lint_command",
    "lint_command",
    "typecheck_command",
    "test_command",
    "build_command",
    "install_command",
];

/// Substrings/patterns that must never be auto-allowed (DB/git mutating, force, rm…).
/// Matched case-insensitively with word-ish boundaries like the legacy regexes.
fn is_blocked(cmd: &str) -> bool {
    let lc = cmd.to_lowercase();
    let contains_word = |hay: &str, needle: &str| hay.contains(needle);
    let blocked_phrases = [
        "db push",
        "drizzle-kit push",
        "drizzle-kit migrate",
        "migrate reset",
        "prisma db seed",
        "supabase db reset",
        "db:push",
        "db:reset",
        "db:seed",
        "git push",
        "git reset",
        "git checkout",
        "deploy",
    ];
    if blocked_phrases.iter().any(|p| contains_word(&lc, p)) {
        return true;
    }
    // `rm` as a word
    if word_present(&lc, "rm") {
        return true;
    }
    // --force / -f flags
    if lc.contains("--force") || flag_present(&lc, "-f") {
        return true;
    }
    false
}

/// True if `word` appears bounded by non-alphanumeric chars (rough `\brm\b`).
fn word_present(s: &str, word: &str) -> bool {
    let bytes = s.as_bytes();
    let w = word.as_bytes();
    let mut i = 0;
    while let Some(pos) = s[i..].find(word) {
        let start = i + pos;
        let end = start + w.len();
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after_ok = end >= bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        i = start + 1;
    }
    false
}

/// True if `-f` appears as a standalone flag (bounded by whitespace / ends).
fn flag_present(s: &str, flag: &str) -> bool {
    s.split(|c: char| c.is_whitespace()).any(|tok| tok == flag)
}

const SHELL_META: &[char] = &[';', '&', '|', '`', '$', '(', ')', '{', '}', '<', '>', '*'];

/// Classification verdict for one command.
pub enum Verdict {
    Ok(String),
    Skip(String),
}

/// Decide whether `command` is safe to auto-allow, returning the exact pattern or a reason.
pub fn classify(command: &str) -> Verdict {
    if command == "*" || command.starts_with('*') {
        return Verdict::Skip("quá rộng (wildcard toàn bộ)".into());
    }
    if command.contains('"')
        || command.contains('\\')
        || command.contains('\r')
        || command.contains('\n')
    {
        return Verdict::Skip("chứa quote/backslash/newline — không an toàn để ghi YAML".into());
    }
    if command.chars().any(|c| SHELL_META.contains(&c)) {
        return Verdict::Skip(
            "chứa shell metacharacter (chaining/wildcard) — thêm rule tay nếu cần".into(),
        );
    }
    if is_blocked(command) {
        return Verdict::Skip("khớp pattern phá hoại".into());
    }
    Verdict::Ok(command.to_string())
}

/// Parse `*_command:` fields from project-config text (dedup, order-preserving).
pub fn parse_commands(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let Some(colon) = trimmed.find(':') else {
            continue;
        };
        let key = trimmed[..colon].trim();
        if !key.chars().all(|c| c.is_ascii_lowercase() || c == '_') || !key.ends_with("_command") {
            continue;
        }
        if !COMMAND_FIELDS.contains(&key) {
            continue;
        }
        // Strip trailing ` # comment`
        let mut value = trimmed[colon + 1..].to_string();
        if let Some(hash) = value.find(" #") {
            value = value[..hash].to_string();
        }
        let mut value = value.trim().to_string();
        // Unquote
        if value.len() >= 2 {
            let b = value.as_bytes();
            if (b[0] == b'"' && b[value.len() - 1] == b'"')
                || (b[0] == b'\'' && b[value.len() - 1] == b'\'')
            {
                value = value[1..value.len() - 1].trim().to_string();
            }
        }
        if value.is_empty() || value == "null" || value == "~" {
            continue;
        }
        if value.starts_with('<') && value.ends_with('>') {
            continue;
        }
        if value.to_lowercase().starts_with("skip") {
            continue;
        }
        if !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

/// Result of patching a single agent file.
#[derive(Debug, PartialEq, Eq)]
pub enum PatchStatus {
    Missing,
    NoMarkers,
    Unchanged,
    WouldUpdate,
    Updated,
}

/// Patch the marker block in `content`, returning (new_content, status).
/// Pure string transform so it is easy to test without touching the filesystem.
pub fn patch_content(content: &str, patterns: &[String], eol: &str) -> (String, PatchStatus) {
    let Some(start_idx) = content.find(START) else {
        return (content.to_string(), PatchStatus::NoMarkers);
    };
    let Some(end_idx) = content.find(END) else {
        return (content.to_string(), PatchStatus::NoMarkers);
    };
    if end_idx < start_idx {
        return (content.to_string(), PatchStatus::NoMarkers);
    }
    // block starts at the line after START marker
    let block_start = match content[start_idx..].find('\n') {
        Some(nl) => start_idx + nl + 1,
        None => return (content.to_string(), PatchStatus::NoMarkers),
    };
    // end line starts at the last newline before END
    let end_line_start = content[..end_idx].rfind('\n').map(|i| i + 1).unwrap_or(0);

    let generated = patterns
        .iter()
        .map(|p| format!("    \"{p}\": allow"))
        .collect::<Vec<_>>()
        .join(eol);
    let replacement = if generated.is_empty() {
        String::new()
    } else {
        format!("{generated}{eol}")
    };
    let updated = format!(
        "{}{}{}",
        &content[..block_start],
        replacement,
        &content[end_line_start..]
    );
    if updated == content {
        (updated, PatchStatus::Unchanged)
    } else {
        (updated, PatchStatus::WouldUpdate)
    }
}

/// The two agent files this tool manages, relative to project root.
pub fn agent_files(root: &Path) -> Vec<PathBuf> {
    vec![
        root.join(".opencode/agent/reviewer.md"),
        root.join(".opencode/agent/spec-validator.md"),
    ]
}

/// Detect CRLF vs LF from a sample content.
pub fn detect_eol(sample: &str) -> &'static str {
    if sample.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}
