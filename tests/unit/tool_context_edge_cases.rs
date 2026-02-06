use std::sync::{Arc, Mutex};

use forge::runtime::error::GraphError;
use forge::runtime::event::{Event, EventSink};
use forge::runtime::permission::{PermissionPolicy, PermissionSession};
use forge::runtime::tool::{
    AttachmentPayload, AttachmentPolicy, ToolAttachment, ToolCall, ToolContext, ToolOutput,
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

#[test]
fn attachment_store_missing_for_oversize_inline_payload_errors() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink: Arc<dyn EventSink> = Arc::new(CaptureSink {
        events: events.clone(),
    });
    let gate = Arc::new(PermissionSession::new(PermissionPolicy::default()));

    let call = ToolCall::new("emit", "call-1", serde_json::json!({}));
    let context = ToolContext::new(
        Arc::clone(&sink),
        gate,
        AttachmentPolicy::new(4),
        call.tool.clone(),
        call.call_id.clone(),
    );

    let output = ToolOutput::text("ok").with_attachment(ToolAttachment::inline(
        "blob.txt",
        "text/plain",
        serde_json::json!("this-is-large"),
    ));

    let result = block_on(ToolRunner::run_with_events(
        call,
        context,
        |_call, _ctx| async move { Ok(output) },
    ));

    assert!(matches!(
        result,
        Err(GraphError::ExecutionError { message, .. }) if message.contains("attachment store unavailable")
    ));
    assert!(events
        .lock()
        .unwrap()
        .iter()
        .any(|event| matches!(event, Event::ToolError { .. })));
}

#[test]
fn attachment_inline_at_threshold_keeps_inline_payload() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink: Arc<dyn EventSink> = Arc::new(CaptureSink {
        events: events.clone(),
    });
    let gate = Arc::new(PermissionSession::new(PermissionPolicy::default()));

    let payload = serde_json::json!("data");
    let size = serde_json::to_vec(&payload).expect("serialize").len();
    let call = ToolCall::new("emit", "call-2", serde_json::json!({}));
    let context = ToolContext::new(
        Arc::clone(&sink),
        gate,
        AttachmentPolicy::new(size),
        call.tool.clone(),
        call.call_id.clone(),
    );

    let output = ToolOutput::text("ok").with_attachment(ToolAttachment::inline(
        "inline.json",
        "application/json",
        payload,
    ));

    let result = block_on(ToolRunner::run_with_events(
        call,
        context,
        |_call, _ctx| async move { Ok(output) },
    ))
    .expect("run");

    let attachment = result.attachments.first().expect("attachment");
    assert!(matches!(
        attachment.payload,
        AttachmentPayload::Inline { .. }
    ));
    assert_eq!(attachment.size, Some(size as u64));
    assert!(events
        .lock()
        .unwrap()
        .iter()
        .any(|event| matches!(event, Event::ToolAttachment { .. })));
}

#[test]
fn attachment_reference_payload_passes_through_without_store() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink: Arc<dyn EventSink> = Arc::new(CaptureSink {
        events: events.clone(),
    });
    let gate = Arc::new(PermissionSession::new(PermissionPolicy::default()));

    let call = ToolCall::new("emit", "call-3", serde_json::json!({}));
    let context = ToolContext::new(
        Arc::clone(&sink),
        gate,
        AttachmentPolicy::default(),
        call.tool.clone(),
        call.call_id.clone(),
    );

    let output = ToolOutput::text("ok").with_attachment(ToolAttachment::reference(
        "image.png",
        "image/png",
        "attachment://ref-1",
        Some(12),
    ));

    let result = block_on(ToolRunner::run_with_events(
        call,
        context,
        |_call, _ctx| async move { Ok(output) },
    ))
    .expect("run");

    let attachment = result.attachments.first().expect("attachment");
    match &attachment.payload {
        AttachmentPayload::Reference { reference } => {
            assert_eq!(reference, "attachment://ref-1");
        }
        _ => panic!("expected reference payload"),
    }
    assert_eq!(attachment.size, Some(12));
    assert!(events
        .lock()
        .unwrap()
        .iter()
        .any(|event| matches!(event, Event::ToolAttachment { .. })));
}
