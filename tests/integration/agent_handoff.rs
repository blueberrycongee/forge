use forge::runtime::constants::{END, START};
use forge::runtime::error::GraphError;
use forge::runtime::graph::StateGraph;
use forge::runtime::node::NodeSpec;
use forge::runtime::state::{GraphState, SharedState};
use futures::executor::block_on;

#[derive(Clone, Default)]
struct AgentState {
    shared: SharedState,
    log: Vec<String>,
}

impl GraphState for AgentState {}

async fn planner(mut state: AgentState) -> Result<AgentState, GraphError> {
    let update = SharedState::with_value("plan", serde_json::json!("draft"));
    state.shared = state.shared.merge(&update);
    state.log.push("planner".to_string());
    Ok(state)
}

async fn worker(mut state: AgentState) -> Result<AgentState, GraphError> {
    let update = SharedState::with_value("work", serde_json::json!("done"));
    state.shared = state.shared.merge(&update);
    state.log.push("worker".to_string());
    Ok(state)
}

async fn reviewer(mut state: AgentState) -> Result<AgentState, GraphError> {
    let update = SharedState::with_value("review", serde_json::json!("approved"));
    state.shared = state.shared.merge(&update);
    state.log.push("reviewer".to_string());
    Ok(state)
}

#[test]
fn agent_handoff_merges_shared_state() {
    let planner_node = NodeSpec::new("planner", planner).with_role("planner");
    let worker_node = NodeSpec::new("worker", worker).with_role("worker");
    let reviewer_node = NodeSpec::new("reviewer", reviewer).with_role("reviewer");

    assert_eq!(
        planner_node
            .metadata
            .as_ref()
            .and_then(|meta| meta.role.as_deref()),
        Some("planner")
    );

    let mut graph = StateGraph::<AgentState>::new();
    graph.add_node_spec(planner_node);
    graph.add_node_spec(worker_node);
    graph.add_node_spec(reviewer_node);
    graph.add_edge(START, "planner");
    graph.add_edge("planner", "worker");
    graph.add_edge("worker", "reviewer");
    graph.add_edge("reviewer", END);

    let compiled = graph.compile().expect("compile");
    let final_state = block_on(compiled.invoke(AgentState::default())).expect("run");

    assert_eq!(final_state.log, vec!["planner", "worker", "reviewer"]);
    assert_eq!(
        final_state.shared.get("plan"),
        Some(&serde_json::json!("draft"))
    );
    assert_eq!(
        final_state.shared.get("work"),
        Some(&serde_json::json!("done"))
    );
    assert_eq!(
        final_state.shared.get("review"),
        Some(&serde_json::json!("approved"))
    );
}
