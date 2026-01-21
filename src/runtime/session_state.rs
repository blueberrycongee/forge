//! Session state model for runtime loop processing.

use crate::runtime::message::{Message, Part};

#[derive(Clone, Debug, PartialEq)]
pub enum SessionRouting {
    Next,
    Complete,
    Interrupt { reason: String },
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SessionPhase {
    UserInput,
    ModelThinking,
    AssistantStreaming,
    ToolProposed,
    ToolRunning,
    ToolResult,
    AssistantFinalize,
    Completed,
    Interrupted,
    Resumed,
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
    pub phase: SessionPhase,
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
            phase: SessionPhase::UserInput,
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

    pub fn mark_user_input(&mut self) {
        self.phase = SessionPhase::UserInput;
    }

    pub fn mark_model_thinking(&mut self) {
        self.phase = SessionPhase::ModelThinking;
    }

    pub fn mark_assistant_streaming(&mut self) {
        self.phase = SessionPhase::AssistantStreaming;
    }

    pub fn mark_tool_proposed(&mut self) {
        self.phase = SessionPhase::ToolProposed;
    }

    pub fn mark_tool_running(&mut self) {
        self.phase = SessionPhase::ToolRunning;
    }

    pub fn mark_tool_result(&mut self) {
        self.phase = SessionPhase::ToolResult;
    }

    pub fn mark_assistant_finalize(&mut self) {
        self.phase = SessionPhase::AssistantFinalize;
    }

    pub fn mark_completed(&mut self) {
        self.phase = SessionPhase::Completed;
    }

    pub fn mark_interrupted(&mut self) {
        self.phase = SessionPhase::Interrupted;
    }

    pub fn mark_resumed(&mut self) {
        self.phase = SessionPhase::Resumed;
    }

    pub fn can_transition(&self, next: &SessionPhase) -> bool {
        use SessionPhase::*;
        if &self.phase == next {
            return true;
        }
        if matches!(next, Interrupted) {
            return !matches!(self.phase, Completed);
        }
        match (&self.phase, next) {
            (UserInput, ModelThinking) => true,
            (ModelThinking, AssistantStreaming) => true,
            (AssistantStreaming, ToolProposed) => true,
            (AssistantStreaming, AssistantFinalize) => true,
            (ToolProposed, ToolRunning) => true,
            (ToolRunning, ToolResult) => true,
            (ToolResult, AssistantStreaming) => true,
            (ToolResult, AssistantFinalize) => true,
            (AssistantFinalize, Completed) => true,
            (Interrupted, Resumed) => true,
            (Resumed, ModelThinking) => true,
            _ => false,
        }
    }

    pub fn try_transition(&mut self, next: SessionPhase) -> Result<(), String> {
        if self.can_transition(&next) {
            self.phase = next;
            Ok(())
        } else {
            Err(format!(
                "invalid transition {:?} -> {:?}",
                self.phase, next
            ))
        }
    }

    pub fn try_transition_with_event(
        &mut self,
        next: SessionPhase,
    ) -> Result<Option<crate::runtime::event::Event>, String> {
        if self.phase == next {
            return Ok(None);
        }
        if self.can_transition(&next) {
            let from = self.phase.clone();
            self.phase = next.clone();
            Ok(Some(crate::runtime::event::Event::SessionPhaseChanged {
                session_id: self.session_id.clone(),
                message_id: self.message_id.clone(),
                from,
                to: next,
            }))
        } else {
            Err(format!(
                "invalid transition {:?} -> {:?}",
                self.phase, next
            ))
        }
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

    pub fn finalize_message(&mut self, role: crate::runtime::message::MessageRole) -> Option<Message> {
        if self.pending_parts.is_empty() {
            return None;
        }
        let mut message = Message::new(role);
        message.parts.extend(self.pending_parts.drain(..));
        self.messages.push(message.clone());
        Some(message)
    }

    pub fn apply_event(&mut self, event: &crate::runtime::event::Event) -> bool {
        let (handled, _events) = self.apply_event_with_events(event);
        handled
    }

    pub fn apply_event_with_events(
        &mut self,
        event: &crate::runtime::event::Event,
    ) -> (bool, Vec<crate::runtime::event::Event>) {
        use crate::runtime::event::Event;
        let mut events = Vec::new();
        match event {
            Event::TextDelta { delta, .. } => {
                if let Ok(Some(event)) =
                    self.try_transition_with_event(SessionPhase::AssistantStreaming)
                {
                    events.push(event);
                }
                self.pending_parts.push(Part::TextDelta {
                    delta: delta.clone(),
                });
                (true, events)
            }
            Event::TextFinal { text, .. } => {
                if let Ok(Some(event)) =
                    self.try_transition_with_event(SessionPhase::AssistantStreaming)
                {
                    events.push(event);
                }
                self.pending_parts.push(Part::TextFinal {
                    text: text.clone(),
                });
                (true, events)
            }
            Event::ToolStart {
                tool,
                call_id,
                input,
            } => {
                if self.phase == SessionPhase::AssistantStreaming {
                    if let Ok(Some(event)) =
                        self.try_transition_with_event(SessionPhase::ToolProposed)
                    {
                        events.push(event);
                    }
                }
                if let Ok(Some(event)) =
                    self.try_transition_with_event(SessionPhase::ToolRunning)
                {
                    events.push(event);
                }
                self.pending_parts.push(Part::ToolCall {
                    tool: tool.clone(),
                    call_id: call_id.clone(),
                    input: input.clone(),
                });
                if !self.update_tool_call(call_id, ToolCallStatus::Running) {
                    let mut record = ToolCallRecord::new(tool.clone(), call_id.clone());
                    record.status = ToolCallStatus::Running;
                    self.tool_calls.push(record);
                }
                (true, events)
            }
            Event::ToolResult {
                tool,
                call_id,
                output,
            } => {
                if let Ok(Some(event)) = self.try_transition_with_event(SessionPhase::ToolResult) {
                    events.push(event);
                }
                self.pending_parts.push(Part::ToolResult {
                    tool: tool.clone(),
                    call_id: call_id.clone(),
                    output: output.clone(),
                });
                self.update_tool_call(call_id, ToolCallStatus::Completed);
                (true, events)
            }
            Event::ToolError {
                tool,
                call_id,
                error,
            } => {
                if let Ok(Some(event)) = self.try_transition_with_event(SessionPhase::ToolResult) {
                    events.push(event);
                }
                self.pending_parts.push(Part::ToolError {
                    tool: tool.clone(),
                    call_id: call_id.clone(),
                    error: error.clone(),
                });
                self.update_tool_call(call_id, ToolCallStatus::Error);
                (true, events)
            }
            Event::StepFinish { tokens, .. } => {
                if let Ok(Some(event)) =
                    self.try_transition_with_event(SessionPhase::AssistantFinalize)
                {
                    events.push(event);
                }
                self.pending_parts.push(Part::TokenUsage {
                    usage: tokens.clone(),
                });
                (true, events)
            }
            Event::Attachment {
                name,
                mime_type,
                data,
                ..
            } => {
                self.pending_parts.push(Part::Attachment {
                    name: name.clone(),
                    mime_type: mime_type.clone(),
                    data: data.clone(),
                });
                (true, events)
            }
            Event::Error { message, .. } => {
                self.pending_parts.push(Part::Error {
                    message: message.clone(),
                });
                (true, events)
            }
            _ => (false, events),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionPhase, SessionRouting, SessionState, ToolCallStatus};
    use crate::runtime::event::{Event, TokenUsage};
    use crate::runtime::message::{MessageRole, Part};
    use crate::runtime::tool::ToolOutput;

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
        assert_eq!(state.phase, SessionPhase::UserInput);
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
    fn session_state_phase_transitions() {
        let mut state = SessionState::new("s1", "m1");

        state.mark_model_thinking();
        assert_eq!(state.phase, SessionPhase::ModelThinking);

        state.mark_assistant_streaming();
        assert_eq!(state.phase, SessionPhase::AssistantStreaming);

        state.mark_tool_running();
        assert_eq!(state.phase, SessionPhase::ToolRunning);

        state.mark_tool_result();
        assert_eq!(state.phase, SessionPhase::ToolResult);

        state.mark_assistant_finalize();
        assert_eq!(state.phase, SessionPhase::AssistantFinalize);

        state.mark_completed();
        assert_eq!(state.phase, SessionPhase::Completed);
    }

    #[test]
    fn session_state_phase_interrupt_and_resume() {
        let mut state = SessionState::new("s1", "m1");

        state.mark_interrupted();
        assert_eq!(state.phase, SessionPhase::Interrupted);

        state.mark_resumed();
        assert_eq!(state.phase, SessionPhase::Resumed);
    }

    #[test]
    fn session_state_try_transition_allows_happy_path() {
        let mut state = SessionState::new("s1", "m1");

        assert!(state.try_transition(SessionPhase::ModelThinking).is_ok());
        assert!(state.try_transition(SessionPhase::AssistantStreaming).is_ok());
        assert!(state.try_transition(SessionPhase::ToolProposed).is_ok());
        assert!(state.try_transition(SessionPhase::ToolRunning).is_ok());
        assert!(state.try_transition(SessionPhase::ToolResult).is_ok());
        assert!(state.try_transition(SessionPhase::AssistantFinalize).is_ok());
        assert!(state.try_transition(SessionPhase::Completed).is_ok());
    }

    #[test]
    fn session_state_try_transition_rejects_invalid() {
        let mut state = SessionState::new("s1", "m1");

        assert!(state.try_transition(SessionPhase::ToolRunning).is_err());
        assert_eq!(state.phase, SessionPhase::UserInput);
    }

    #[test]
    fn session_state_transition_emits_event() {
        let mut state = SessionState::new("s1", "m1");

        let event = state
            .try_transition_with_event(SessionPhase::ModelThinking)
            .expect("transition")
            .expect("event");

        assert_eq!(
            event,
            crate::runtime::event::Event::SessionPhaseChanged {
                session_id: "s1".to_string(),
                message_id: "m1".to_string(),
                from: SessionPhase::UserInput,
                to: SessionPhase::ModelThinking,
            }
        );
    }

    #[test]
    fn session_state_transition_same_phase_emits_none() {
        let mut state = SessionState::new("s1", "m1");
        let event = state
            .try_transition_with_event(SessionPhase::UserInput)
            .expect("transition");

        assert!(event.is_none());
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

    #[test]
    fn session_state_finalize_message_merges_pending_parts_in_order() {
        let mut state = SessionState::new("s1", "m1");
        state.pending_parts.push(Part::TextDelta {
            delta: "he".to_string(),
        });
        state.pending_parts.push(Part::TextFinal {
            text: "llo".to_string(),
        });

        let message = state.finalize_message(MessageRole::Assistant).expect("message");

        assert_eq!(message.role, MessageRole::Assistant);
        assert_eq!(
            message.parts,
            vec![
                Part::TextDelta {
                    delta: "he".to_string()
                },
                Part::TextFinal {
                    text: "llo".to_string()
                }
            ]
        );
        assert!(state.pending_parts.is_empty());
        assert_eq!(state.messages.len(), 1);
    }

    #[test]
    fn session_state_finalize_message_skips_when_no_pending_parts() {
        let mut state = SessionState::new("s1", "m1");
        let message = state.finalize_message(MessageRole::User);

        assert!(message.is_none());
        assert!(state.messages.is_empty());
    }

    #[test]
    fn session_state_apply_event_appends_text_delta() {
        let mut state = SessionState::new("s1", "m1");
        let event = Event::TextDelta {
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            delta: "hi".to_string(),
        };

        assert!(state.apply_event(&event));
        assert_eq!(
            state.pending_parts,
            vec![Part::TextDelta {
                delta: "hi".to_string()
            }]
        );
    }

    #[test]
    fn session_state_apply_event_tracks_tool_lifecycle() {
        let mut state = SessionState::new("s1", "m1");
        let start = Event::ToolStart {
            tool: "read".to_string(),
            call_id: "c1".to_string(),
            input: serde_json::json!({"path": "file.txt"}),
        };
        let result = Event::ToolResult {
            tool: "read".to_string(),
            call_id: "c1".to_string(),
            output: ToolOutput::text("ok"),
        };

        assert!(state.apply_event(&start));
        assert_eq!(state.tool_calls.len(), 1);
        assert_eq!(state.tool_calls[0].status, ToolCallStatus::Running);

        assert!(state.apply_event(&result));
        assert_eq!(state.tool_calls[0].status, ToolCallStatus::Completed);
        assert_eq!(state.pending_parts.len(), 2);
    }

    #[test]
    fn session_state_apply_event_advances_phase_for_text_delta() {
        let mut state = SessionState::new("s1", "m1");
        state.mark_model_thinking();

        let event = Event::TextDelta {
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            delta: "hi".to_string(),
        };

        assert!(state.apply_event(&event));
        assert_eq!(state.phase, SessionPhase::AssistantStreaming);
    }

    #[test]
    fn session_state_apply_event_advances_phase_for_tool_start() {
        let mut state = SessionState::new("s1", "m1");
        state.mark_assistant_streaming();

        let event = Event::ToolStart {
            tool: "read".to_string(),
            call_id: "c1".to_string(),
            input: serde_json::json!({"path": "file.txt"}),
        };

        assert!(state.apply_event(&event));
        assert_eq!(state.phase, SessionPhase::ToolRunning);
    }

    #[test]
    fn session_state_apply_event_advances_phase_for_tool_result() {
        let mut state = SessionState::new("s1", "m1");
        state.mark_tool_running();

        let event = Event::ToolResult {
            tool: "read".to_string(),
            call_id: "c1".to_string(),
            output: ToolOutput::text("ok"),
        };

        assert!(state.apply_event(&event));
        assert_eq!(state.phase, SessionPhase::ToolResult);
    }

    #[test]
    fn session_state_apply_event_advances_phase_for_step_finish() {
        let mut state = SessionState::new("s1", "m1");
        state.mark_tool_result();

        let event = Event::StepFinish {
            session_id: "s1".to_string(),
            tokens: TokenUsage {
                input: 1,
                output: 2,
                reasoning: 3,
                cache_read: 4,
                cache_write: 5,
            },
            cost: 0.01,
        };

        assert!(state.apply_event(&event));
        assert_eq!(state.phase, SessionPhase::AssistantFinalize);
    }

    #[test]
    fn session_state_apply_event_with_events_emits_phase_change() {
        let mut state = SessionState::new("s1", "m1");
        state.mark_model_thinking();

        let event = Event::TextDelta {
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            delta: "hi".to_string(),
        };

        let (handled, events) = state.apply_event_with_events(&event);
        assert!(handled);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            Event::SessionPhaseChanged {
                session_id: "s1".to_string(),
                message_id: "m1".to_string(),
                from: SessionPhase::ModelThinking,
                to: SessionPhase::AssistantStreaming,
            }
        );
        assert_eq!(state.phase, SessionPhase::AssistantStreaming);
    }

    #[test]
    fn session_state_apply_event_with_events_emits_tool_phase_steps() {
        let mut state = SessionState::new("s1", "m1");
        state.mark_assistant_streaming();

        let event = Event::ToolStart {
            tool: "read".to_string(),
            call_id: "c1".to_string(),
            input: serde_json::json!({"path": "file.txt"}),
        };

        let (handled, events) = state.apply_event_with_events(&event);
        assert!(handled);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], Event::SessionPhaseChanged {
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            from: SessionPhase::AssistantStreaming,
            to: SessionPhase::ToolProposed,
        });
        assert_eq!(events[1], Event::SessionPhaseChanged {
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            from: SessionPhase::ToolProposed,
            to: SessionPhase::ToolRunning,
        });
        assert_eq!(state.phase, SessionPhase::ToolRunning);
    }

    #[test]
    fn session_state_apply_event_with_events_skips_invalid_transition() {
        let mut state = SessionState::new("s1", "m1");
        let event = Event::ToolStart {
            tool: "read".to_string(),
            call_id: "c1".to_string(),
            input: serde_json::json!({"path": "file.txt"}),
        };

        let (handled, events) = state.apply_event_with_events(&event);
        assert!(handled);
        assert!(events.is_empty());
        assert_eq!(state.phase, SessionPhase::UserInput);
    }

    #[test]
    fn session_state_apply_event_appends_text_final() {
        let mut state = SessionState::new("s1", "m1");
        let event = Event::TextFinal {
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            text: "done".to_string(),
        };

        assert!(state.apply_event(&event));
        assert_eq!(
            state.pending_parts,
            vec![Part::TextFinal {
                text: "done".to_string()
            }]
        );
    }

    #[test]
    fn session_state_apply_event_appends_token_usage() {
        let mut state = SessionState::new("s1", "m1");
        let event = Event::StepFinish {
            session_id: "s1".to_string(),
            tokens: TokenUsage {
                input: 1,
                output: 2,
                reasoning: 3,
                cache_read: 4,
                cache_write: 5,
            },
            cost: 0.01,
        };

        assert!(state.apply_event(&event));
        assert_eq!(
            state.pending_parts,
            vec![Part::TokenUsage {
                usage: TokenUsage {
                    input: 1,
                    output: 2,
                    reasoning: 3,
                    cache_read: 4,
                    cache_write: 5,
                }
            }]
        );
    }

    #[test]
    fn session_state_apply_event_appends_attachment() {
        let mut state = SessionState::new("s1", "m1");
        let event = Event::Attachment {
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            name: "file.txt".to_string(),
            mime_type: "text/plain".to_string(),
            data: serde_json::json!({"size": 4}),
        };

        assert!(state.apply_event(&event));
        assert_eq!(
            state.pending_parts,
            vec![Part::Attachment {
                name: "file.txt".to_string(),
                mime_type: "text/plain".to_string(),
                data: serde_json::json!({"size": 4}),
            }]
        );
    }

    #[test]
    fn session_state_apply_event_appends_error() {
        let mut state = SessionState::new("s1", "m1");
        let event = Event::Error {
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            message: "boom".to_string(),
        };

        assert!(state.apply_event(&event));
        assert_eq!(
            state.pending_parts,
            vec![Part::Error {
                message: "boom".to_string()
            }]
        );
    }
}
