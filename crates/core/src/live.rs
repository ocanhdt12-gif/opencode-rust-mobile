//! vibe-studio live hook (port of .claude/hooks/live-studio.mjs).
//!
//! vibe-studio itself stays TypeScript (decision #1). This hook only maps opencode
//! lifecycle events to enter/leave character actions and emits them. The mapping:
//!
//! | lifecycle event        | action      |
//! |------------------------|-------------|
//! | UserPromptSubmit       | chat-enter  |
//! | Stop                   | chat-leave  |
//! | PreToolUse(Agent)      | agent-enter |
//! | SubagentStop           | agent-leave |
//! | PreToolUse(SendMessage)| agent-enter |
//! | PostToolUse(Agent)     | agent-leave |
//!
//! Characters are keyed by a stable agent id (`role-<agentId>`) so enter/leave pair up.

/// Enter/leave action a hook can emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    ChatEnter,
    ChatLeave,
    AgentEnter,
    AgentLeave,
}

impl Action {
    /// Wire string emitted to vibe-studio.
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::ChatEnter => "chat-enter",
            Action::ChatLeave => "chat-leave",
            Action::AgentEnter => "agent-enter",
            Action::AgentLeave => "agent-leave",
        }
    }
}

/// Map an opencode lifecycle event (+ optional tool name) to an action.
pub fn map_event(event: &str, tool: Option<&str>) -> Option<Action> {
    match event {
        "UserPromptSubmit" => Some(Action::ChatEnter),
        "Stop" => Some(Action::ChatLeave),
        "SubagentStop" => Some(Action::AgentLeave),
        "PreToolUse" => match tool {
            Some("Agent") | Some("SendMessage") => Some(Action::AgentEnter),
            _ => None,
        },
        "PostToolUse" => match tool {
            Some("Agent") => Some(Action::AgentLeave),
            _ => None,
        },
        _ => None,
    }
}

/// Stable character key for an agent id.
pub fn character_key(agent_id: &str) -> String {
    format!("role-{agent_id}")
}
