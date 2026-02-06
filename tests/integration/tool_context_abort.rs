use std::sync::Arc;
use std::time::{Duration, Instant};

use forge::runtime::constants::{END, START};
use forge::runtime::error::GraphError;
use forge::runtime::event::{Event, EventSink};
use forge::runtime::executor::{ExecutionConfig, ExecutionResult};
use forge::runtime::graph::StateGraph;
use forge::runtime::permission::{PermissionPolicy, PermissionSession};
use forge::runtime::prelude::LoopNode;
use forge::runtime::state::GraphState;
use forge::runtime::tool::{ToolCall, ToolRegistry};
use futures::executor::block_on;

#[derive(Clone, Default)]
struct AbortState {
    steps: usize,
}

impl GraphState for AbortState {}

#[derive(Default)]
struct CaptureSink {
    events: std::sync::Mutex<Vec<Event>>,
}

impl EventSink for CaptureSink {
    fn emit(&self, event: Event) -> Result<(), GraphError> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

#[test]
fn aborting_tool_run_marks_aborted() {
    let gate = Arc::new(PermissionSession::new(PermissionPolicy::default()));
    let mut registry = ToolRegistry::new();
    registry.register(
        "abort",
        Arc::new(|_call, ctx| Box::pin(async move { ctx.abort("stop") })),
    );
    let registry = Arc::new(registry);

    let node = LoopNode::with_tools_and_gate(
        "loop",
        Arc::clone(&registry),
        Arc::clone(&gate),
        |mut state: AbortState, ctx| async move {
            state.steps += 1;
            ctx.run_tool(ToolCall::new("abort", "call-1", serde_json::json!({})))
                .await?;
            Ok(state)
        },
    );

    let mut graph = StateGraph::<AbortState>::new();
    graph.add_node_spec(node.into_node());
    graph.add_edge(START, "loop");
    graph.add_edge("loop", END);

    let capture = Arc::new(CaptureSink::default());
    let sink: Arc<dyn EventSink> = capture.clone();
    let compiled = graph
        .compile()
        .expect("compile")
        .with_config(ExecutionConfig::new().with_run_event_sink(Arc::clone(&sink)));

    let start = Instant::now();
    let result = block_on(compiled.invoke_resumable(AbortState::default()));
    assert!(start.elapsed() < Duration::from_secs(5));

    match result {
        Err(GraphError::Aborted { .. }) => {}
        Ok(ExecutionResult::Complete(_)) => panic!("expected aborted"),
        Ok(ExecutionResult::Interrupted { .. }) => panic!("expected aborted"),
        Err(err) => panic!("unexpected error: {}", err),
    }

    let events = capture.events.lock().unwrap();
    assert!(events
        .iter()
        .any(|event| matches!(event, Event::RunAborted { .. })));
}
