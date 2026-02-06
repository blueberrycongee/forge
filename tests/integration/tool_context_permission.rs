use std::sync::Arc;

use forge::runtime::constants::{END, START};
use forge::runtime::error::ResumeCommand;
use forge::runtime::event::PermissionReply;
use forge::runtime::executor::ExecutionResult;
use forge::runtime::graph::StateGraph;
use forge::runtime::permission::{PermissionDecision, PermissionPolicy, PermissionRule, PermissionSession};
use forge::runtime::prelude::LoopNode;
use forge::runtime::state::GraphState;
use forge::runtime::tool::{ToolCall, ToolOutput, ToolRegistry};
use futures::executor::block_on;

#[derive(Clone, Default)]
struct PermissionState {
    completed: bool,
}

impl GraphState for PermissionState {}

#[test]
fn tool_context_permission_interrupts_and_resumes() {
    let gate = Arc::new(PermissionSession::new(PermissionPolicy::new(vec![
        PermissionRule::new(PermissionDecision::Ask, vec!["perm:danger".to_string()]),
    ])));

    let mut registry = ToolRegistry::new();
    registry.register("danger", Arc::new(|_call, ctx| {
        Box::pin(async move {
            ctx.ask_permission("perm:danger")?;
            Ok(ToolOutput::text("ok"))
        })
    }));
    let registry = Arc::new(registry);

    let node = LoopNode::with_tools_and_gate(
        "loop",
        Arc::clone(&registry),
        Arc::clone(&gate),
        |mut state: PermissionState, ctx| async move {
            let output = ctx
                .run_tool(ToolCall::new("danger", "call-1", serde_json::json!({})))
                .await?;
            if output.content == serde_json::Value::String("ok".to_string()) {
                state.completed = true;
            }
            Ok(state)
        },
    );

    let mut graph = StateGraph::<PermissionState>::new();
    graph.add_node_spec(node.into_node());
    graph.add_edge(START, "loop");
    graph.add_edge("loop", END);

    let compiled = graph.compile().expect("compile");
    let result = block_on(compiled.invoke_resumable(PermissionState::default())).expect("run");
    let checkpoint = match result {
        ExecutionResult::Interrupted { checkpoint, .. } => checkpoint,
        _ => panic!("expected interrupt"),
    };

    gate.apply_reply("perm:danger", PermissionReply::Once);

    let resumed = block_on(compiled.resume(checkpoint, ResumeCommand::new("allow")))
        .expect("resume");
    match resumed {
        ExecutionResult::Complete(state) => assert!(state.completed),
        _ => panic!("expected completion"),
    }
}

#[test]
fn tool_context_permission_without_reply_stays_interrupted() {
    let gate = Arc::new(PermissionSession::new(PermissionPolicy::new(vec![
        PermissionRule::new(PermissionDecision::Ask, vec!["perm:danger".to_string()]),
    ])));

    let mut registry = ToolRegistry::new();
    registry.register("danger", Arc::new(|_call, ctx| {
        Box::pin(async move {
            ctx.ask_permission("perm:danger")?;
            Ok(ToolOutput::text("ok"))
        })
    }));
    let registry = Arc::new(registry);

    let node = LoopNode::with_tools_and_gate(
        "loop",
        Arc::clone(&registry),
        Arc::clone(&gate),
        |state: PermissionState, ctx| async move {
            ctx.run_tool(ToolCall::new("danger", "call-1", serde_json::json!({})))
                .await?;
            Ok(state)
        },
    );

    let mut graph = StateGraph::<PermissionState>::new();
    graph.add_node_spec(node.into_node());
    graph.add_edge(START, "loop");
    graph.add_edge("loop", END);

    let compiled = graph.compile().expect("compile");
    let result = block_on(compiled.invoke_resumable(PermissionState::default())).expect("run");
    match result {
        ExecutionResult::Interrupted { .. } => {}
        _ => panic!("expected interrupt"),
    }
}
