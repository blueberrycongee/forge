//! Tool lifecycle types for streaming execution.

use std::collections::HashMap;
use std::sync::Arc;

use crate::langgraph::error::GraphResult;
use crate::langgraph::event::{Event, EventSink};
use crate::langgraph::error::GraphError;
use serde::{Deserialize, Serialize};

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

/// Typed metadata for tool output.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub mime_type: Option<String>,
    pub schema: Option<String>,
    pub source: Option<String>,
    pub attributes: serde_json::Map<String, serde_json::Value>,
}

impl ToolMetadata {
    pub fn new() -> Self {
        Self {
            mime_type: None,
            schema: None,
            source: None,
            attributes: serde_json::Map::new(),
        }
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }
}

impl Default for ToolMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Structured tool output payload.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolOutput {
    pub content: serde_json::Value,
    pub metadata: Option<ToolMetadata>,
}

impl ToolOutput {
    pub fn new(content: serde_json::Value) -> Self {
        Self {
            content,
            metadata: None,
        }
    }

    pub fn with_metadata(content: serde_json::Value, metadata: ToolMetadata) -> Self {
        Self {
            content,
            metadata: Some(metadata),
        }
    }

    pub fn text(text: impl Into<String>) -> Self {
        Self::new(serde_json::Value::String(text.into()))
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        let metadata = self.metadata.get_or_insert_with(ToolMetadata::new);
        metadata.mime_type = Some(mime_type.into());
        self
    }

    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        let metadata = self.metadata.get_or_insert_with(ToolMetadata::new);
        metadata.schema = Some(schema.into());
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        let metadata = self.metadata.get_or_insert_with(ToolMetadata::new);
        metadata.source = Some(source.into());
        self
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        let metadata = self.metadata.get_or_insert_with(ToolMetadata::new);
        metadata.attributes.insert(key.into(), value);
        self
    }
}

/// Tool execution facade that emits lifecycle events.
pub struct ToolRunner;

impl ToolRunner {
    pub async fn run_with_events<F, Fut>(
        call: ToolCall,
        sink: Arc<dyn EventSink>,
        run: F,
    ) -> GraphResult<ToolOutput>
    where
        F: FnOnce(ToolCall) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = GraphResult<ToolOutput>> + Send + 'static,
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

/// Tool handler signature for registry execution.
pub type ToolHandler =
    Arc<dyn Fn(ToolCall) -> crate::langgraph::node::BoxFuture<'static, GraphResult<ToolOutput>> + Send + Sync>;

/// Minimal tool registry for dispatching by name.
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolHandler>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: impl Into<String>, handler: ToolHandler) {
        self.tools.insert(name.into(), handler);
    }

    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    pub async fn run_with_events(
        &self,
        call: ToolCall,
        sink: Arc<dyn EventSink>,
    ) -> GraphResult<ToolOutput> {
        let handler = self.tools.get(&call.tool).cloned().ok_or_else(|| {
            GraphError::ExecutionError {
                node: format!("tool:{}", call.tool),
                message: "tool not found".to_string(),
            }
        })?;

        ToolRunner::run_with_events(call, sink, move |call| handler(call)).await
    }
}

#[cfg(test)]
mod tests {
    use super::{ToolCall, ToolMetadata, ToolOutput, ToolRegistry, ToolRunner, ToolState};
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
            Ok(ToolOutput::text(format!("ok:{}", call.tool)))
        }))
        .expect("tool run");

        assert_eq!(result.content, serde_json::Value::String("ok:grep".to_string()));
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

    #[test]
    fn tool_registry_dispatches_by_name() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });
        let mut registry = ToolRegistry::new();

        registry.register("echo", Arc::new(|call| {
            Box::pin(async move { Ok(ToolOutput::text(format!("echo:{}", call.tool))) })
        }));

        let call = ToolCall::new("echo", "call-2", serde_json::json!({"msg": "hi"}));
        let result = block_on(registry.run_with_events(call, sink)).expect("registry run");

        assert_eq!(result.content, serde_json::Value::String("echo:echo".to_string()));
        assert!(events
            .lock()
            .unwrap()
            .iter()
            .any(|event| matches!(event, Event::ToolResult { .. })));
    }

    #[test]
    fn tool_registry_returns_error_for_missing_tool() {
        let registry = ToolRegistry::new();
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink {
            events: Arc::new(Mutex::new(Vec::new())),
        });
        let call = ToolCall::new("missing", "call-3", serde_json::json!({}));

        let result = block_on(registry.run_with_events(call, sink));
        assert!(result.is_err());
    }

    #[test]
    fn tool_output_metadata_helpers() {
        let output = ToolOutput::text("hello")
            .with_mime_type("text/plain")
            .with_schema("v1")
            .with_source("unit-test")
            .with_attribute("lang", serde_json::json!("en"));

        let metadata = output.metadata.expect("metadata");
        assert_eq!(metadata.mime_type.as_deref(), Some("text/plain"));
        assert_eq!(metadata.schema.as_deref(), Some("v1"));
        assert_eq!(metadata.source.as_deref(), Some("unit-test"));
        assert_eq!(metadata.attributes.get("lang"), Some(&serde_json::json!("en")));
    }

    #[test]
    fn tool_output_with_metadata_accepts_struct() {
        let metadata = ToolMetadata::new()
            .with_mime_type("application/json")
            .with_attribute("size", serde_json::json!(12));
        let output = ToolOutput::with_metadata(serde_json::json!({"ok": true}), metadata.clone());

        assert_eq!(output.metadata, Some(metadata));
    }
}
