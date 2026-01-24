use std::sync::Arc;

use forge::runtime::error::GraphError;
use forge::runtime::event::NoopEventSink;
use forge::runtime::cancel::CancellationToken;
use forge::runtime::permission::{PermissionPolicy, PermissionSession};
use forge::runtime::tool::{AttachmentPolicy, ToolContext};

#[test]
fn tool_context_abort_returns_aborted_error() {
    let sink: Arc<dyn forge::runtime::event::EventSink> = Arc::new(NoopEventSink);
    let gate = Arc::new(PermissionSession::new(PermissionPolicy::default()));
    let ctx = ToolContext::new(
        Arc::clone(&sink),
        gate,
        AttachmentPolicy::default(),
        "aborter",
        "call-1",
    );

    let result: Result<(), GraphError> = ctx.abort("stop");
    match result {
        Err(GraphError::Aborted { reason }) => assert_eq!(reason, "stop"),
        other => panic!("expected aborted error, got {:?}", other),
    }
}

#[test]
fn tool_context_check_cancelled_returns_aborted_error() {
    let sink: Arc<dyn forge::runtime::event::EventSink> = Arc::new(NoopEventSink);
    let gate = Arc::new(PermissionSession::new(PermissionPolicy::default()));
    let token = CancellationToken::new();
    token.cancel("user");
    let ctx = ToolContext::new(
        Arc::clone(&sink),
        gate,
        AttachmentPolicy::default(),
        "aborter",
        "call-2",
    )
    .with_cancellation_token(token);

    let result = ctx.check_cancelled();
    match result {
        Err(GraphError::Aborted { reason }) => assert_eq!(reason, "user"),
        other => panic!("expected aborted error, got {:?}", other),
    }
}
