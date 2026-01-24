use std::sync::{Arc, Mutex};

use forge::runtime::error::GraphError;
use forge::runtime::event::{Event, EventSink, PermissionReply};
use forge::runtime::permission::{PermissionDecision, PermissionPolicy, PermissionRule, PermissionSession};
use forge::runtime::prelude::LoopNode;
use forge::runtime::session_state::SessionState;
use forge::runtime::state::GraphState;
use forge::runtime::tool::{ToolCall, ToolOutput, ToolRegistry};
use futures::executor::block_on;

#[derive(Clone, Default)]
struct FlowState {
    log: Vec<String>,
}

impl GraphState for FlowState {}

struct CaptureSink {
    events: Arc<Mutex<Vec<Event>>>,
}

impl EventSink for CaptureSink {
    fn emit(&self, event: Event) {
        self.events.lock().unwrap().push(event);
    }
}

#[test]
fn permission_flow_interrupts_on_ask() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });

    let mut registry = ToolRegistry::new();
    registry.register("echo", Arc::new(|call| {
        Box::pin(async move { Ok(ToolOutput::text(call.tool)) })
    }));
    let registry = Arc::new(registry);

    let gate = Arc::new(PermissionSession::new(PermissionPolicy::new(vec![
        PermissionRule::new(PermissionDecision::Ask, vec!["tool:echo".to_string()]),
    ])));

    let node = LoopNode::with_tools_and_gate(
        "flow",
        Arc::clone(&registry),
        Arc::clone(&gate),
        |state: FlowState, ctx| async move {
            ctx.run_tool(ToolCall::new("echo", "call-1", serde_json::json!({})))
                .await?;
            Ok(state)
        },
    );

    let result = block_on(node.run(FlowState::default(), sink));
    match result {
        Err(GraphError::Interrupted(interrupts)) => {
            assert_eq!(interrupts.len(), 1);
        }
        other => panic!("expected interrupted, got {:?}", other),
    }

    let captured = events.lock().unwrap();
    assert!(captured.iter().any(|event| matches!(event, Event::PermissionAsked { .. })));
}

#[test]
fn permission_flow_records_reply_and_allows() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });
    let session_state = Arc::new(Mutex::new(SessionState::new("s1", "m1")));

    let mut registry = ToolRegistry::new();
    registry.register("echo", Arc::new(|call| {
        Box::pin(async move { Ok(ToolOutput::text(call.tool)) })
    }));
    let registry = Arc::new(registry);

    let gate = Arc::new(PermissionSession::new(PermissionPolicy::new(vec![
        PermissionRule::new(PermissionDecision::Ask, vec!["tool:echo".to_string()]),
    ])));

    let node = LoopNode::with_tools_and_gate(
        "flow",
        Arc::clone(&registry),
        Arc::clone(&gate),
        |mut state: FlowState, ctx| async move {
            ctx.reply_permission("tool:echo", PermissionReply::Once);
            let output = ctx
                .run_tool(ToolCall::new("echo", "call-2", serde_json::json!({})))
                .await?;
            state.log.push(
                output
                    .content
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            );
            Ok(state)
        },
    );

    let result = block_on(node.run_with_session_state(
        FlowState::default(),
        Arc::clone(&session_state),
        sink,
    ));
    assert!(result.is_ok());

    let session_state = session_state.lock().unwrap();
    assert_eq!(session_state.permission_decisions.len(), 1);
    assert_eq!(session_state.permission_decisions[0].permission, "tool:echo");

    let captured = events.lock().unwrap();
    assert!(captured
        .iter()
        .any(|event| matches!(event, Event::PermissionReplied { .. })));
}

#[test]
fn permission_flow_denies_tool() {
    let mut registry = ToolRegistry::new();
    registry.register("echo", Arc::new(|call| {
        Box::pin(async move { Ok(ToolOutput::text(call.tool)) })
    }));
    let registry = Arc::new(registry);

    let gate = Arc::new(PermissionSession::new(PermissionPolicy::new(vec![
        PermissionRule::new(PermissionDecision::Deny, vec!["tool:echo".to_string()]),
    ])));

    let node = LoopNode::with_tools_and_gate(
        "flow",
        Arc::clone(&registry),
        Arc::clone(&gate),
        |state: FlowState, ctx| async move {
            ctx.run_tool(ToolCall::new("echo", "call-3", serde_json::json!({})))
                .await?;
            Ok(state)
        },
    );

    let result = block_on(node.run(FlowState::default(), Arc::new(CaptureSink {
        events: Arc::new(Mutex::new(Vec::new())),
    })));

    match result {
        Err(GraphError::PermissionDenied { permission, .. }) => {
            assert_eq!(permission, "tool:echo");
        }
        other => panic!("expected permission denied, got {:?}", other),
    }
}
