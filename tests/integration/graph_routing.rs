use forge::runtime::constants::{END, START};
use forge::runtime::error::GraphError;
use forge::runtime::graph::StateGraph;
use forge::runtime::state::GraphState;
use futures::executor::block_on;

#[derive(Clone, Default)]
struct RouteState {
    route: String,
    visited: Vec<String>,
}

impl GraphState for RouteState {}

async fn decide(state: RouteState) -> Result<RouteState, GraphError> {
    Ok(state)
}

async fn approve(mut state: RouteState) -> Result<RouteState, GraphError> {
    state.visited.push("approve".to_string());
    Ok(state)
}

async fn reject(mut state: RouteState) -> Result<RouteState, GraphError> {
    state.visited.push("reject".to_string());
    Ok(state)
}

fn route_by_state(state: RouteState) -> Result<String, GraphError> {
    Ok(state.route.clone())
}

#[test]
fn conditional_routing_selects_approve() {
    let mut graph = StateGraph::<RouteState>::new();
    graph.add_node("decide", decide);
    graph.add_node("approve", approve);
    graph.add_node("reject", reject);
    graph.add_edge(START, "decide");
    graph.add_conditional_edges(
        "decide",
        route_by_state,
        Some(
            [
                ("approve".to_string(), "approve".to_string()),
                ("reject".to_string(), "reject".to_string()),
            ]
            .into_iter()
            .collect(),
        ),
    );
    graph.add_edge("approve", END);
    graph.add_edge("reject", END);

    let compiled = graph.compile().expect("compile");
    let state = RouteState {
        route: "approve".to_string(),
        visited: Vec::new(),
    };
    let final_state = block_on(compiled.invoke(state)).expect("run");

    assert_eq!(final_state.visited, vec!["approve".to_string()]);
}

#[test]
fn conditional_routing_selects_reject() {
    let mut graph = StateGraph::<RouteState>::new();
    graph.add_node("decide", decide);
    graph.add_node("approve", approve);
    graph.add_node("reject", reject);
    graph.add_edge(START, "decide");
    graph.add_conditional_edges(
        "decide",
        route_by_state,
        Some(
            [
                ("approve".to_string(), "approve".to_string()),
                ("reject".to_string(), "reject".to_string()),
            ]
            .into_iter()
            .collect(),
        ),
    );
    graph.add_edge("approve", END);
    graph.add_edge("reject", END);

    let compiled = graph.compile().expect("compile");
    let state = RouteState {
        route: "reject".to_string(),
        visited: Vec::new(),
    };
    let final_state = block_on(compiled.invoke(state)).expect("run");

    assert_eq!(final_state.visited, vec!["reject".to_string()]);
}
