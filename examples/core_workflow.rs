use std::sync::Arc;

use forge::runtime::constants::{END, START};
use forge::runtime::error::GraphError;
use forge::runtime::graph::StateGraph;
use forge::runtime::prelude::LoopNode;
use forge::runtime::state::GraphState;
use forge::runtime::{builtin_tool_registry, tool};
use futures::executor::block_on;

#[derive(Clone, Default)]
struct WorkflowState {
    results: Vec<String>,
}

impl GraphState for WorkflowState {}

fn main() -> Result<(), GraphError> {
    let root = std::env::temp_dir().join(format!("forge-workflow-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("create workspace");
    std::fs::write(root.join("input.txt"), "alpha\nbeta\ngamma\n").expect("seed file");

    let registry = Arc::new(builtin_tool_registry(&root));

    let node = LoopNode::with_tools("workflow", Arc::clone(&registry), |mut state: WorkflowState, ctx| async move {
        let read = ctx
            .run_tool(tool::ToolCall::new(
                "read",
                "call-read",
                serde_json::json!({"path": "input.txt"}),
            ))
            .await?;
        let content = read
            .content
            .get("content")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        state.results.push(content);

        ctx.run_tool(tool::ToolCall::new(
            "write",
            "call-write",
            serde_json::json!({"path": "output.txt", "content": "done"}),
        ))
        .await?;

        let confirm = ctx
            .run_tool(tool::ToolCall::new(
                "read",
                "call-confirm",
                serde_json::json!({"path": "output.txt"}),
            ))
            .await?;
        let confirm_content = confirm
            .content
            .get("content")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        state.results.push(confirm_content);

        Ok(state)
    });

    let mut graph = StateGraph::<WorkflowState>::new();
    graph.add_node_spec(node.into_node());
    graph.add_edge(START, "workflow");
    graph.add_edge("workflow", END);

    let compiled = graph.compile()?;
    let final_state = block_on(compiled.invoke(WorkflowState::default()))?;

    println!("Results: {:?}", final_state.results);
    println!("Workspace: {}", root.display());

    Ok(())
}
