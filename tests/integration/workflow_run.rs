use std::sync::Arc;

use crate::fixtures::workspace::WorkspaceFixture;
use crate::helpers::events::EventCollector;
use forge::runtime::constants::{END, START};
use forge::runtime::event::Event;
use forge::runtime::graph::StateGraph;
use forge::runtime::prelude::LoopNode;
use forge::runtime::state::GraphState;
use forge::runtime::tool::{ToolCall, ToolRegistry};
use forge::runtime::toolkit::file_tools::register_file_tools;
use futures::executor::block_on;

#[derive(Clone, Default)]
struct WorkflowState {
    log: Vec<String>,
    matches: usize,
}

impl GraphState for WorkflowState {}

#[test]
fn workflow_runs_with_file_tools() {
    let fixture = WorkspaceFixture::with_sample_files().expect("fixture");

    let mut registry = ToolRegistry::new();
    register_file_tools(&mut registry, fixture.root());
    let registry = Arc::new(registry);

    let node = LoopNode::with_tools(
        "workflow",
        Arc::clone(&registry),
        |mut state: WorkflowState, ctx| async move {
            let read = ctx
                .run_tool(ToolCall::new(
                    "read",
                    "call-1",
                    serde_json::json!({"path": "notes/todo.txt"}),
                ))
                .await?;
            let content = read
                .content
                .get("content")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string();
            state.log.push(content);

            let search = ctx
                .run_tool(ToolCall::new(
                    "search",
                    "call-2",
                    serde_json::json!({"query": "beta"}),
                ))
                .await?;
            let matches = search
                .content
                .get("matches")
                .and_then(|value| value.as_array())
                .map(|items| items.len())
                .unwrap_or(0);
            state.matches = matches;

            ctx.run_tool(ToolCall::new(
                "write",
                "call-3",
                serde_json::json!({"path": "out.txt", "content": "done"}),
            ))
            .await?;

            let confirm = ctx
                .run_tool(ToolCall::new(
                    "read",
                    "call-4",
                    serde_json::json!({"path": "out.txt"}),
                ))
                .await?;
            let confirm_content = confirm
                .content
                .get("content")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string();
            state.log.push(confirm_content);

            Ok(state)
        },
    );

    let mut graph = StateGraph::<WorkflowState>::new();
    graph.add_node_spec(node.into_node());
    graph.add_edge(START, "workflow");
    graph.add_edge("workflow", END);

    let compiled = graph.compile().expect("compile");
    let collector = EventCollector::new();
    let final_state =
        block_on(compiled.stream_events(WorkflowState::default(), collector.sink())).expect("run");

    assert!(final_state.log.iter().any(|entry| entry.contains("alpha")));
    assert_eq!(final_state.log.last().map(String::as_str), Some("done"));
    assert_eq!(final_state.matches, 1);

    let events = collector.events();
    assert!(events
        .iter()
        .any(|event| matches!(event, Event::ToolResult { tool, .. } if tool == "read")));
    assert!(events
        .iter()
        .any(|event| matches!(event, Event::ToolResult { tool, .. } if tool == "write")));
}
