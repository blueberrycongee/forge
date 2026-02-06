use std::sync::{Arc, Mutex};

use forge::runtime::error::GraphError;
use forge::runtime::event::{Event, EventSink};
use forge::runtime::permission::{PermissionPolicy, PermissionSession};
use forge::runtime::tool::{
    AttachmentPolicy, AttachmentStore, ToolAttachment, ToolCall, ToolContext, ToolOutput,
    ToolRunner,
};
use futures::executor::block_on;

struct CaptureSink {
    events: Arc<Mutex<Vec<Event>>>,
}

impl EventSink for CaptureSink {
    fn emit(&self, event: Event) -> Result<(), GraphError> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

#[derive(Clone, Default)]
struct MemoryAttachmentStore {
    stored: Arc<Mutex<Vec<ToolAttachment>>>,
}

impl AttachmentStore for MemoryAttachmentStore {
    fn store(&self, attachment: &ToolAttachment) -> Result<String, GraphError> {
        self.stored.lock().unwrap().push(attachment.clone());
        Ok("attachment://mem-1".to_string())
    }
}

#[test]
fn attachment_policy_converts_large_inline_payloads() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink: Arc<dyn EventSink> = Arc::new(CaptureSink {
        events: events.clone(),
    });
    let gate = Arc::new(PermissionSession::new(PermissionPolicy::default()));
    let store = Arc::new(MemoryAttachmentStore::default());

    let call = ToolCall::new("emit", "call-1", serde_json::json!({}));
    let context = ToolContext::new(
        Arc::clone(&sink),
        gate,
        AttachmentPolicy::new(4),
        call.tool.clone(),
        call.call_id.clone(),
    )
    .with_attachment_store(store);

    let output = ToolOutput::text("ok").with_attachment(ToolAttachment::inline(
        "file.txt",
        "text/plain",
        serde_json::json!("this-is-large"),
    ));

    let result = block_on(ToolRunner::run_with_events(
        call,
        context,
        |_call, _ctx| async move { Ok(output) },
    ))
    .expect("run");

    assert_eq!(result.content, serde_json::Value::String("ok".to_string()));
    let captured = events.lock().unwrap();
    assert!(captured.iter().any(|event| matches!(
        event,
        Event::ToolAttachment { attachment, .. }
            if matches!(attachment.payload, forge::runtime::tool::AttachmentPayload::Reference { .. })
    )));
}

#[test]
fn attachment_policy_rejects_empty_mime_type() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink: Arc<dyn EventSink> = Arc::new(CaptureSink {
        events: events.clone(),
    });
    let gate = Arc::new(PermissionSession::new(PermissionPolicy::default()));

    let call = ToolCall::new("emit", "call-2", serde_json::json!({}));
    let context = ToolContext::new(
        Arc::clone(&sink),
        gate,
        AttachmentPolicy::default(),
        call.tool.clone(),
        call.call_id.clone(),
    );

    let output = ToolOutput::text("ok").with_attachment(ToolAttachment::inline(
        "file.bin",
        "",
        serde_json::json!("payload"),
    ));

    let result = block_on(ToolRunner::run_with_events(
        call,
        context,
        |_call, _ctx| async move { Ok(output) },
    ));

    assert!(result.is_err());
    let captured = events.lock().unwrap();
    assert!(captured
        .iter()
        .any(|event| matches!(event, Event::ToolError { .. })));
}
