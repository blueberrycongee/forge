//! Tool lifecycle types for streaming execution.

use std::collections::HashMap;
use std::sync::Arc;

use crate::runtime::cancel::CancellationToken;
use crate::runtime::error::{GraphError, GraphResult, Interrupt};
use crate::runtime::event::{Event, EventSink, ToolUpdate};
use crate::runtime::permission::{PermissionDecision, PermissionGate, PermissionRequest};
use serde::{Deserialize, Serialize};

/// Tool lifecycle states for execution tracking.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub fn new(
        tool: impl Into<String>,
        call_id: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        Self {
            tool: tool.into(),
            call_id: call_id.into(),
            input,
        }
    }
}

/// Attachment payload for tool outputs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AttachmentPayload {
    Inline { data: serde_json::Value },
    Reference { reference: String },
}

/// Tool attachment descriptor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolAttachment {
    pub name: String,
    pub mime_type: String,
    pub size: Option<u64>,
    pub payload: AttachmentPayload,
}

impl ToolAttachment {
    pub fn inline(
        name: impl Into<String>,
        mime_type: impl Into<String>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            mime_type: mime_type.into(),
            size: None,
            payload: AttachmentPayload::Inline { data },
        }
    }

    pub fn reference(
        name: impl Into<String>,
        mime_type: impl Into<String>,
        reference: impl Into<String>,
        size: Option<u64>,
    ) -> Self {
        Self {
            name: name.into(),
            mime_type: mime_type.into(),
            size,
            payload: AttachmentPayload::Reference {
                reference: reference.into(),
            },
        }
    }
}

/// Policy for inline vs reference attachment handling.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AttachmentPolicy {
    pub max_inline_bytes: usize,
}

impl AttachmentPolicy {
    pub fn new(max_inline_bytes: usize) -> Self {
        Self { max_inline_bytes }
    }
}

impl Default for AttachmentPolicy {
    fn default() -> Self {
        Self {
            max_inline_bytes: 64 * 1024,
        }
    }
}

/// Attachment persistence interface.
pub trait AttachmentStore: Send + Sync {
    fn store(&self, attachment: &ToolAttachment) -> GraphResult<String>;
}

/// Context passed to tools for emitting events and requesting permissions.
#[derive(Clone)]
pub struct ToolContext {
    sink: Arc<dyn EventSink>,
    gate: Arc<dyn PermissionGate>,
    attachment_policy: AttachmentPolicy,
    attachment_store: Option<Arc<dyn AttachmentStore>>,
    tool: String,
    call_id: String,
    cancel: CancellationToken,
}

impl ToolContext {
    pub fn new(
        sink: Arc<dyn EventSink>,
        gate: Arc<dyn PermissionGate>,
        attachment_policy: AttachmentPolicy,
        tool: impl Into<String>,
        call_id: impl Into<String>,
    ) -> Self {
        Self {
            sink,
            gate,
            attachment_policy,
            attachment_store: None,
            tool: tool.into(),
            call_id: call_id.into(),
            cancel: CancellationToken::new(),
        }
    }

    pub fn tool(&self) -> &str {
        &self.tool
    }

    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    pub fn attachment_policy(&self) -> &AttachmentPolicy {
        &self.attachment_policy
    }

    pub fn with_attachment_store(mut self, store: Arc<dyn AttachmentStore>) -> Self {
        self.attachment_store = Some(store);
        self
    }

    pub fn with_cancellation_token(mut self, token: CancellationToken) -> Self {
        self.cancel = token;
        self
    }

    pub fn attachment_store(&self) -> Option<Arc<dyn AttachmentStore>> {
        self.attachment_store.clone()
    }

    pub fn emit(&self, event: Event) -> GraphResult<()> {
        self.sink.emit(event)
    }

    pub fn emit_tool_update(&self, update: ToolUpdate) -> GraphResult<()> {
        self.sink.emit(Event::ToolUpdate {
            tool: self.tool.clone(),
            call_id: self.call_id.clone(),
            update,
        })
    }

    pub fn sink(&self) -> Arc<dyn EventSink> {
        Arc::clone(&self.sink)
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    pub fn check_cancelled(&self) -> GraphResult<()> {
        if self.cancel.is_cancelled() {
            return Err(GraphError::Aborted {
                reason: self.cancel.abort_reason(),
            });
        }
        Ok(())
    }

    pub fn ask_permission(&self, permission: impl Into<String>) -> GraphResult<()> {
        let permission = permission.into();
        match self.gate.decide(&permission) {
            PermissionDecision::Allow => Ok(()),
            PermissionDecision::Ask => {
                let mut metadata = serde_json::Map::new();
                metadata.insert("tool".to_string(), serde_json::json!(self.tool));
                metadata.insert("call_id".to_string(), serde_json::json!(self.call_id));
                let request = PermissionRequest::new(permission.clone(), vec![permission.clone()])
                    .with_metadata(metadata)
                    .with_always(vec![permission.clone()]);
                self.emit(request.to_event())?;
                Err(GraphError::Interrupted(vec![Interrupt::new(
                    request,
                    format!("permission:{}", permission),
                )]))
            }
            PermissionDecision::Deny => Err(GraphError::PermissionDenied {
                permission,
                message: "permission denied".to_string(),
            }),
        }
    }

    pub fn abort<T>(&self, reason: impl Into<String>) -> GraphResult<T> {
        Err(GraphError::Aborted {
            reason: reason.into(),
        })
    }
}

/// Describes a tool's input/output contract.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub sensitive: bool,
}

impl ToolDefinition {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: None,
            output_schema: None,
            sensitive: false,
        }
    }

    pub fn with_input_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    pub fn with_output_schema(mut self, schema: serde_json::Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    pub fn mark_sensitive(mut self) -> Self {
        self.sensitive = true;
        self
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
    pub attachments: Vec<ToolAttachment>,
}

impl ToolOutput {
    pub fn new(content: serde_json::Value) -> Self {
        Self {
            content,
            metadata: None,
            attachments: Vec::new(),
        }
    }

    pub fn with_metadata(content: serde_json::Value, metadata: ToolMetadata) -> Self {
        Self {
            content,
            metadata: Some(metadata),
            attachments: Vec::new(),
        }
    }

    pub fn text(text: impl Into<String>) -> Self {
        Self::new(serde_json::Value::String(text.into()))
    }

    pub fn with_attachment(mut self, attachment: ToolAttachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    pub fn with_attachments(mut self, attachments: Vec<ToolAttachment>) -> Self {
        self.attachments = attachments;
        self
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
        context: ToolContext,
        run: F,
    ) -> GraphResult<ToolOutput>
    where
        F: FnOnce(ToolCall, ToolContext) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = GraphResult<ToolOutput>> + Send + 'static,
    {
        let tool = call.tool.clone();
        let call_id = call.call_id.clone();
        let input = call.input.clone();
        let sink = context.sink();

        sink.emit(Event::ToolStatus {
            tool: tool.clone(),
            call_id: call_id.clone(),
            state: ToolState::Pending,
        })?;
        sink.emit(Event::ToolStart {
            tool: tool.clone(),
            call_id: call_id.clone(),
            input,
        })?;
        sink.emit(Event::ToolStatus {
            tool: tool.clone(),
            call_id: call_id.clone(),
            state: ToolState::Running,
        })?;

        let context_clone = context.clone();
        let result = run(call, context).await;

        match result {
            Ok(mut output) => {
                match process_attachments(&context_clone, &tool, output.attachments.clone()) {
                    Ok(attachments) => {
                        output.attachments = attachments.clone();
                        sink.emit(Event::ToolStatus {
                            tool: tool.clone(),
                            call_id: call_id.clone(),
                            state: ToolState::Completed,
                        })?;
                        for attachment in attachments {
                            sink.emit(Event::ToolAttachment {
                                tool: tool.clone(),
                                call_id: call_id.clone(),
                                attachment,
                            })?;
                        }
                        sink.emit(Event::ToolResult {
                            tool: tool.clone(),
                            call_id: call_id.clone(),
                            output: output.clone(),
                        })?;
                        Ok(output)
                    }
                    Err(err) => {
                        sink.emit(Event::ToolStatus {
                            tool: tool.clone(),
                            call_id: call_id.clone(),
                            state: ToolState::Error,
                        })?;
                        sink.emit(Event::ToolError {
                            tool: tool.clone(),
                            call_id: call_id.clone(),
                            error: err.to_string(),
                        })?;
                        Err(err)
                    }
                }
            }
            Err(err) => {
                sink.emit(Event::ToolStatus {
                    tool: tool.clone(),
                    call_id: call_id.clone(),
                    state: ToolState::Error,
                })?;
                sink.emit(Event::ToolError {
                    tool: tool.clone(),
                    call_id: call_id.clone(),
                    error: err.to_string(),
                })?;
                Err(err)
            }
        }
    }
}

fn process_attachments(
    context: &ToolContext,
    tool: &str,
    attachments: Vec<ToolAttachment>,
) -> GraphResult<Vec<ToolAttachment>> {
    let mut processed = Vec::new();
    for attachment in attachments {
        processed.push(normalize_attachment(context, tool, attachment)?);
    }
    Ok(processed)
}

fn normalize_attachment(
    context: &ToolContext,
    tool: &str,
    mut attachment: ToolAttachment,
) -> GraphResult<ToolAttachment> {
    if attachment.mime_type.trim().is_empty() {
        return Err(GraphError::ExecutionError {
            node: format!("tool:{}", tool),
            message: "attachment mime_type missing".to_string(),
        });
    }

    match &attachment.payload {
        AttachmentPayload::Inline { data } => {
            let bytes = serde_json::to_vec(data).map_err(|err| GraphError::ExecutionError {
                node: format!("tool:{}", tool),
                message: format!("attachment serialization failed: {}", err),
            })?;
            let size = bytes.len() as u64;
            if size <= context.attachment_policy.max_inline_bytes as u64 {
                attachment.size = Some(size);
                return Ok(attachment);
            }
            let store = context
                .attachment_store()
                .ok_or_else(|| GraphError::ExecutionError {
                    node: format!("tool:{}", tool),
                    message: "attachment store unavailable".to_string(),
                })?;
            let reference = store.store(&attachment)?;
            Ok(ToolAttachment::reference(
                attachment.name,
                attachment.mime_type,
                reference,
                Some(size),
            ))
        }
        AttachmentPayload::Reference { .. } => Ok(attachment),
    }
}

/// Tool handler signature for registry execution.
pub type ToolHandler = Arc<
    dyn Fn(
            ToolCall,
            ToolContext,
        ) -> crate::runtime::node::BoxFuture<'static, GraphResult<ToolOutput>>
        + Send
        + Sync,
>;

/// Minimal tool registry for dispatching by name.
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolHandler>,
    definitions: HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            definitions: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: impl Into<String>, handler: ToolHandler) {
        self.tools.insert(name.into(), handler);
    }

    pub fn register_with_definition(&mut self, definition: ToolDefinition, handler: ToolHandler) {
        let name = definition.name.clone();
        self.tools.insert(name.clone(), handler);
        self.definitions.insert(name, definition);
    }

    pub fn definition(&self, name: &str) -> Option<&ToolDefinition> {
        self.definitions.get(name)
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.definitions.values().cloned().collect()
    }

    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    pub async fn run_with_events(
        &self,
        call: ToolCall,
        context: ToolContext,
    ) -> GraphResult<ToolOutput> {
        let handler =
            self.tools
                .get(&call.tool)
                .cloned()
                .ok_or_else(|| GraphError::ExecutionError {
                    node: format!("tool:{}", call.tool),
                    message: "tool not found".to_string(),
                })?;

        ToolRunner::run_with_events(call, context, move |call, ctx| handler(call, ctx)).await
    }
}

/// Registry of common tool output schemas/metadata.
#[derive(Clone, Debug, Default)]
pub struct ToolSchemaRegistry {
    schemas: HashMap<String, ToolMetadata>,
}

impl ToolSchemaRegistry {
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    pub fn with_common_schemas() -> Self {
        let mut registry = Self::new();
        registry.register(
            "read",
            ToolMetadata::new()
                .with_mime_type("text/plain")
                .with_schema("tool.read.v1"),
        );
        registry.register(
            "grep",
            ToolMetadata::new()
                .with_mime_type("application/json")
                .with_schema("tool.grep.v1"),
        );
        registry.register(
            "ls",
            ToolMetadata::new()
                .with_mime_type("application/json")
                .with_schema("tool.ls.v1"),
        );
        registry
    }

    pub fn register(&mut self, tool: impl Into<String>, metadata: ToolMetadata) {
        self.schemas.insert(tool.into(), metadata);
    }

    pub fn get(&self, tool: &str) -> Option<&ToolMetadata> {
        self.schemas.get(tool)
    }

    pub fn annotate_output(&self, tool: &str, output: ToolOutput) -> ToolOutput {
        if output.metadata.is_some() {
            return output;
        }
        let mut output = output;
        if let Some(metadata) = self.schemas.get(tool) {
            output.metadata = Some(metadata.clone());
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AttachmentPolicy, ToolCall, ToolContext, ToolMetadata, ToolOutput, ToolRegistry,
        ToolRunner, ToolSchemaRegistry, ToolState,
    };
    use crate::runtime::event::{Event, EventSink};
    use crate::runtime::permission::{PermissionPolicy, PermissionSession};
    use futures::executor::block_on;
    use std::sync::{Arc, Mutex};

    struct CaptureSink {
        events: Arc<Mutex<Vec<Event>>>,
    }

    impl EventSink for CaptureSink {
        fn emit(&self, event: Event) -> crate::runtime::error::GraphResult<()> {
            self.events.lock().unwrap().push(event);
            Ok(())
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
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink {
            events: events.clone(),
        });

        let call = ToolCall::new("grep", "call-1", serde_json::json!({"q": "hi"}));
        let gate = Arc::new(PermissionSession::new(PermissionPolicy::default()));
        let context = ToolContext::new(
            Arc::clone(&sink),
            gate,
            AttachmentPolicy::default(),
            call.tool.clone(),
            call.call_id.clone(),
        );
        let result = block_on(ToolRunner::run_with_events(
            call,
            context,
            |call, _ctx| async move { Ok(ToolOutput::text(format!("ok:{}", call.tool))) },
        ))
        .expect("tool run");

        assert_eq!(
            result.content,
            serde_json::Value::String("ok:grep".to_string())
        );
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
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink {
            events: events.clone(),
        });
        let mut registry = ToolRegistry::new();

        registry.register(
            "echo",
            Arc::new(|call, _ctx| {
                Box::pin(async move { Ok(ToolOutput::text(format!("echo:{}", call.tool))) })
            }),
        );

        let call = ToolCall::new("echo", "call-2", serde_json::json!({"msg": "hi"}));
        let gate = Arc::new(PermissionSession::new(PermissionPolicy::default()));
        let context = ToolContext::new(
            Arc::clone(&sink),
            gate,
            AttachmentPolicy::default(),
            call.tool.clone(),
            call.call_id.clone(),
        );
        let result = block_on(registry.run_with_events(call, context)).expect("registry run");

        assert_eq!(
            result.content,
            serde_json::Value::String("echo:echo".to_string())
        );
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
        let gate = Arc::new(PermissionSession::new(PermissionPolicy::default()));
        let context = ToolContext::new(
            Arc::clone(&sink),
            gate,
            AttachmentPolicy::default(),
            call.tool.clone(),
            call.call_id.clone(),
        );

        let result = block_on(registry.run_with_events(call, context));
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
        assert_eq!(
            metadata.attributes.get("lang"),
            Some(&serde_json::json!("en"))
        );
    }

    #[test]
    fn tool_output_with_metadata_accepts_struct() {
        let metadata = ToolMetadata::new()
            .with_mime_type("application/json")
            .with_attribute("size", serde_json::json!(12));
        let output = ToolOutput::with_metadata(serde_json::json!({"ok": true}), metadata.clone());

        assert_eq!(output.metadata, Some(metadata));
    }

    #[test]
    fn tool_schema_registry_has_common_entries() {
        let registry = ToolSchemaRegistry::with_common_schemas();
        assert!(registry.get("read").is_some());
        assert!(registry.get("grep").is_some());
        assert!(registry.get("ls").is_some());
    }

    #[test]
    fn tool_schema_registry_annotates_missing_metadata() {
        let registry = ToolSchemaRegistry::with_common_schemas();
        let output = ToolOutput::text("hello");
        let annotated = registry.annotate_output("read", output);
        let metadata = annotated.metadata.expect("metadata");
        assert_eq!(metadata.schema.as_deref(), Some("tool.read.v1"));
        assert_eq!(metadata.mime_type.as_deref(), Some("text/plain"));
    }

    #[test]
    fn tool_schema_registry_does_not_override_existing_metadata() {
        let registry = ToolSchemaRegistry::with_common_schemas();
        let output = ToolOutput::text("hello").with_schema("custom.schema");
        let annotated = registry.annotate_output("read", output.clone());
        assert_eq!(annotated.metadata, output.metadata);
    }
}
