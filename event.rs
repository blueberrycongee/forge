//! Event protocol for streaming execution.
//!
//! This is the foundation for OpenCode-style runtime events
//! (text/tool/step/permission) so clients can consume a single stream.

use std::fmt::Debug;

/// Token usage breakdown (input/output/reasoning/cache).
#[derive(Clone, Debug, Default)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub reasoning: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

/// Permission reply outcomes.
#[derive(Clone, Debug)]
pub enum PermissionReply {
    Once,
    Always,
    Reject,
}

/// Runtime events emitted during execution.
#[derive(Clone, Debug)]
pub enum Event {
    TextDelta {
        session_id: String,
        message_id: String,
        delta: String,
    },
    ToolStart {
        tool: String,
        call_id: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool: String,
        call_id: String,
        output: String,
    },
    ToolError {
        tool: String,
        call_id: String,
        error: String,
    },
    StepStart {
        session_id: String,
    },
    StepFinish {
        session_id: String,
        tokens: TokenUsage,
        cost: f64,
    },
    PermissionAsked {
        permission: String,
        patterns: Vec<String>,
    },
    PermissionReplied {
        permission: String,
        reply: PermissionReply,
    },
}

/// Event sink for streaming runtime events to UI/CLI/SSE/etc.
pub trait EventSink: Send + Sync {
    fn emit(&self, event: Event);
}

/// A no-op event sink for tests or silent execution.
pub struct NoopEventSink;

impl EventSink for NoopEventSink {
    fn emit(&self, _event: Event) {}
}
