//! Tool lifecycle types for streaming execution.

use std::sync::Arc;

use crate::langgraph::error::GraphResult;
use crate::langgraph::event::{Event, EventSink};

/// Tool lifecycle states for execution tracking.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolState {
    Pending,
    Running,
    Completed,
    Error,
}

/// Tool invocation metadata.
#[derive(Clone, Debug)]
pub struct ToolCall {
    pub tool: String,
    pub call_id: String,
    pub input: serde_json::Value,
}

impl ToolCall {
    pub fn new(tool: impl Into<String>, call_id: impl Into<String>, input: serde_json::Value) -> Self {
        Self {
            tool: tool.into(),
            call_id: call_id.into(),
            input,
        }
    }
}

/// Tool execution facade that emits lifecycle events.
pub struct ToolRunner;

impl ToolRunner {
    pub async fn run_with_events<F, Fut>(
        call: ToolCall,
        sink: Arc<dyn EventSink>,
        run: F,
    ) -> GraphResult<String>
    where
        F: FnOnce(ToolCall) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = GraphResult<String>> + Send + 'static,
    {
        let tool = call.tool.clone();
        let call_id = call.call_id.clone();
        let input = call.input.clone();

        sink.emit(Event::ToolStatus {
            tool: tool.clone(),
            call_id: call_id.clone(),
            state: ToolState::Pending,
        });
        sink.emit(Event::ToolStart {
            tool: tool.clone(),
            call_id: call_id.clone(),
            input,
        });
        sink.emit(Event::ToolStatus {
            tool: tool.clone(),
            call_id: call_id.clone(),
            state: ToolState::Running,
        });

        let result = run(call).await;

        match &result {
            Ok(output) => {
                sink.emit(Event::ToolStatus {
                    tool: tool.clone(),
                    call_id: call_id.clone(),
                    state: ToolState::Completed,
                });
                sink.emit(Event::ToolResult {
                    tool: tool.clone(),
                    call_id: call_id.clone(),
                    output: output.clone(),
                });
            }
            Err(err) => {
                sink.emit(Event::ToolStatus {
                    tool: tool.clone(),
                    call_id: call_id.clone(),
                    state: ToolState::Error,
                });
                sink.emit(Event::ToolError {
                    tool: tool.clone(),
                    call_id: call_id.clone(),
                    error: err.to_string(),
                });
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::{ToolCall, ToolRunner, ToolState};
    use crate::langgraph::event::{Event, EventSink};
    use futures::executor::block_on;
    use std::sync::{Arc, Mutex};

    struct CaptureSink {
        events: Arc<Mutex<Vec<Event>>>,
    }

    impl EventSink for CaptureSink {
        fn emit(&self, event: Event) {
            self.events.lock().unwrap().push(event);
        }
    }

    #[test]
    fn tool_state_equality() {
        assert_eq!(ToolState::Pending, ToolState::Pending);
        assert_ne!(ToolState::Pending, ToolState::Running);
    }

    #[test]
    fn tool_runner_emits_status_and_result() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });

        let call = ToolCall::new("grep", "call-1", serde_json::json!({"q": "hi"}));
        let result = block_on(ToolRunner::run_with_events(call, sink, |call| async move {
            Ok(format!("ok:{}", call.tool))
        }))
        .expect("tool run");

        assert_eq!(result, "ok:grep");
        let kinds: Vec<&'static str> = events
            .lock()
            .unwrap()
            .iter()
            .map(|event| match event {
                Event::ToolStatus { state, .. } => match state {
                    ToolState::Pending => "pending",
                    ToolState::Running => "running",
                    ToolState::Completed => "completed",
                    ToolState::Error => "error",
                },
                Event::ToolStart { .. } => "start",
                Event::ToolResult { .. } => "result",
                Event::ToolError { .. } => "error_event",
                _ => "other",
            })
            .collect();

        assert_eq!(
            kinds,
            vec!["pending", "start", "running", "completed", "result"]
        );
    }
}
