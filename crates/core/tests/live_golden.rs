//! Golden tests for `live` event mapping.

use opencode_core::live::{self, Action};

#[test]
fn maps_chat_events() {
    assert_eq!(
        live::map_event("UserPromptSubmit", None),
        Some(Action::ChatEnter)
    );
    assert_eq!(live::map_event("Stop", None), Some(Action::ChatLeave));
    assert_eq!(
        live::map_event("SubagentStop", None),
        Some(Action::AgentLeave)
    );
}

#[test]
fn maps_tool_events() {
    assert_eq!(
        live::map_event("PreToolUse", Some("Agent")),
        Some(Action::AgentEnter)
    );
    assert_eq!(
        live::map_event("PreToolUse", Some("SendMessage")),
        Some(Action::AgentEnter)
    );
    assert_eq!(
        live::map_event("PostToolUse", Some("Agent")),
        Some(Action::AgentLeave)
    );
    // unrelated tool → no action
    assert_eq!(live::map_event("PreToolUse", Some("Read")), None);
    assert_eq!(live::map_event("Unknown", None), None);
}

#[test]
fn action_wire_strings() {
    assert_eq!(Action::ChatEnter.as_str(), "chat-enter");
    assert_eq!(Action::ChatLeave.as_str(), "chat-leave");
    assert_eq!(Action::AgentEnter.as_str(), "agent-enter");
    assert_eq!(Action::AgentLeave.as_str(), "agent-leave");
}

#[test]
fn character_key_format() {
    assert_eq!(live::character_key("builder"), "role-builder");
}
