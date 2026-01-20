//! Session state model for runtime loop processing.

use crate::runtime::message::{Message, Part};

#[derive(Clone, Debug, PartialEq)]
pub enum SessionRouting {
    Next,
    Complete,
    Interrupt { reason: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ToolCallStatus {
    Pending,
    Running,
    Completed,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolCallRecord {
    pub tool: String,
    pub call_id: String,
    pub status: ToolCallStatus,
}

impl ToolCallRecord {
    pub fn new(tool: impl Into<String>, call_id: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            call_id: call_id.into(),
            status: ToolCallStatus::Pending,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionState {
    pub session_id: String,
    pub parent_id: Option<String>,
    pub message_id: String,
    pub step: u64,
    pub messages: Vec<Message>,
    pub pending_parts: Vec<Part>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub routing: SessionRouting,
}

impl SessionState {
    pub fn new(session_id: impl Into<String>, message_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            parent_id: None,
            message_id: message_id.into(),
            step: 0,
            messages: Vec::new(),
            pending_parts: Vec::new(),
            tool_calls: Vec::new(),
            routing: SessionRouting::Next,
        }
    }

    pub fn with_parent_id(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    pub fn advance_step(&mut self) {
        self.step = self.step.saturating_add(1);
    }

    pub fn route_next(&mut self) {
        self.routing = SessionRouting::Next;
    }

    pub fn route_complete(&mut self) {
        self.routing = SessionRouting::Complete;
    }

    pub fn route_interrupt(&mut self, reason: impl Into<String>) {
        self.routing = SessionRouting::Interrupt {
            reason: reason.into(),
        };
    }

    pub fn push_tool_call(&mut self, tool: impl Into<String>, call_id: impl Into<String>) {
        self.tool_calls.push(ToolCallRecord::new(tool, call_id));
    }

    pub fn update_tool_call(&mut self, call_id: &str, status: ToolCallStatus) -> bool {
        if let Some(entry) = self.tool_calls.iter_mut().find(|entry| entry.call_id == call_id) {
            entry.status = status;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionRouting, SessionState, ToolCallStatus};

    #[test]
    fn session_state_new_initializes_fields() {
        let state = SessionState::new("s1", "m1");

        assert_eq!(state.session_id, "s1");
        assert_eq!(state.parent_id, None);
        assert_eq!(state.message_id, "m1");
        assert_eq!(state.step, 0);
        assert!(state.messages.is_empty());
        assert!(state.pending_parts.is_empty());
        assert!(state.tool_calls.is_empty());
        assert_eq!(state.routing, SessionRouting::Next);
    }

    #[test]
    fn session_state_routes_between_states() {
        let mut state = SessionState::new("s1", "m1");

        state.route_interrupt("need approval");
        assert_eq!(
            state.routing,
            SessionRouting::Interrupt {
                reason: "need approval".to_string()
            }
        );

        state.route_complete();
        assert_eq!(state.routing, SessionRouting::Complete);

        state.route_next();
        assert_eq!(state.routing, SessionRouting::Next);
    }

    #[test]
    fn session_state_tracks_tool_calls() {
        let mut state = SessionState::new("s1", "m1");

        state.push_tool_call("read", "c1");
        assert_eq!(state.tool_calls.len(), 1);
        assert_eq!(state.tool_calls[0].status, ToolCallStatus::Pending);

        assert!(state.update_tool_call("c1", ToolCallStatus::Running));
        assert_eq!(state.tool_calls[0].status, ToolCallStatus::Running);
        assert!(!state.update_tool_call("missing", ToolCallStatus::Completed));
    }
}
