use std::sync::{Arc, Mutex};

use forge::runtime::error::GraphResult;
use forge::runtime::event::{Event, EventSink};

/// Capture runtime events for test assertions.
#[derive(Clone, Default)]
pub struct EventCollector {
    events: Arc<Mutex<Vec<Event>>>,
}

impl EventCollector {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn sink(&self) -> Arc<dyn EventSink> {
        Arc::new(CollectorSink {
            events: Arc::clone(&self.events),
        })
    }

    pub fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    pub fn count_where<F>(&self, predicate: F) -> usize
    where
        F: Fn(&Event) -> bool,
    {
        self.events.lock().unwrap().iter().filter(|event| predicate(event)).count()
    }
}

struct CollectorSink {
    events: Arc<Mutex<Vec<Event>>>,
}

impl EventSink for CollectorSink {
    fn emit(&self, event: Event) -> GraphResult<()> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

pub fn event_name(event: &Event) -> &'static str {
    match event {
        Event::RunStarted { .. } => "run_started",
        Event::RunPaused { .. } => "run_paused",
        Event::RunResumed { .. } => "run_resumed",
        Event::RunCompleted { .. } => "run_completed",
        Event::RunFailed { .. } => "run_failed",
        Event::RunAborted { .. } => "run_aborted",
        Event::TextDelta { .. } => "text_delta",
        Event::TextFinal { .. } => "text_final",
        Event::Attachment { .. } => "attachment",
        Event::Error { .. } => "error",
        Event::ToolStart { .. } => "tool_start",
        Event::ToolUpdate { .. } => "tool_update",
        Event::ToolResult { .. } => "tool_result",
        Event::ToolAttachment { .. } => "tool_attachment",
        Event::ToolError { .. } => "tool_error",
        Event::ToolStatus { .. } => "tool_status",
        Event::StepStart { .. } => "step_start",
        Event::StepFinish { .. } => "step_finish",
        Event::PermissionAsked { .. } => "permission_asked",
        Event::PermissionReplied { .. } => "permission_replied",
        Event::SessionCompacted { .. } => "session_compacted",
        Event::SessionPhaseChanged { .. } => "session_phase_changed",
        Event::SessionPhaseTransitionRejected { .. } => "session_phase_transition_rejected",
    }
}

pub fn event_names(events: &[Event]) -> Vec<&'static str> {
    events.iter().map(event_name).collect()
}
