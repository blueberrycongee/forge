use std::sync::{Arc, Mutex};

use forge::runtime::error::GraphError;
use forge::runtime::event::{Event, EventSink};
use forge::runtime::tool::{ToolCall, ToolDefinition, ToolOutput, ToolRegistry, ToolRunner};
use futures::executor::block_on;

struct CaptureSink {
    events: Arc<Mutex<Vec<Event>>>,
}

impl EventSink for CaptureSink {
    fn emit(&self, event: Event) {
        self.events.lock().unwrap().push(event);
    }
}

#[test]
fn tool_runner_reports_success_events() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });

    let call = ToolCall::new("echo", "call-1", serde_json::json!({"msg": "hi"}));
    let result = block_on(ToolRunner::run_with_events(call, sink, |call| async move {
        Ok(ToolOutput::text(format!("ok:{}", call.tool)))
    }))
    .expect("tool run");

    assert_eq!(result.content, serde_json::Value::String("ok:echo".to_string()));
    let captured = events.lock().unwrap();
    assert!(captured.iter().any(|event| matches!(event, Event::ToolStart { .. })));
    assert!(captured.iter().any(|event| matches!(event, Event::ToolResult { .. })));
}

#[test]
fn tool_runner_reports_error_events() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });

    let call = ToolCall::new("fail", "call-2", serde_json::json!({}));
    let result = block_on(ToolRunner::run_with_events(call, sink, |_call| async move {
        Err(GraphError::ExecutionError {
            node: "tool:fail".to_string(),
            message: "boom".to_string(),
        })
    }));

    assert!(result.is_err());
    let captured = events.lock().unwrap();
    assert!(captured.iter().any(|event| matches!(event, Event::ToolError { .. })));
    assert!(captured
        .iter()
        .any(|event| matches!(event, Event::ToolStatus { state: forge::runtime::tool::ToolState::Error, .. })));
}

#[test]
fn tool_registry_exposes_definitions() {
    let mut registry = ToolRegistry::new();
    registry.register_with_definition(
        ToolDefinition::new("echo", "Echo input"),
        Arc::new(|call| Box::pin(async move { Ok(ToolOutput::text(call.tool)) })),
    );

    let definition = registry.definition("echo").expect("definition");
    assert_eq!(definition.name, "echo");
    assert_eq!(definition.description, "Echo input");
}
