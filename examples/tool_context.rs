use std::sync::Arc;

use forge::runtime::constants::{END, START};
use forge::runtime::error::{GraphError, ResumeCommand};
use forge::runtime::event::PermissionReply;
use forge::runtime::executor::ExecutionResult;
use forge::runtime::graph::StateGraph;
use forge::runtime::permission::{
    PermissionDecision, PermissionPolicy, PermissionRule, PermissionSession,
};
use forge::runtime::prelude::LoopNode;
use forge::runtime::state::GraphState;
use forge::runtime::tool::{
    AttachmentPayload, AttachmentPolicy, ToolAttachment, ToolCall, ToolContext, ToolOutput,
    ToolRegistry,
};
use futures::executor::block_on;

#[derive(Clone, Default)]
struct ContextState {
    logs: Vec<String>,
    abort: bool,
}

impl GraphState for ContextState {}

fn describe_attachment(attachment: &ToolAttachment) -> String {
    match &attachment.payload {
        AttachmentPayload::Inline { .. } => {
            format!("inline:{}:{}", attachment.name, attachment.mime_type)
        }
        AttachmentPayload::Reference { reference } => format!(
            "ref:{}:{}:{}",
            attachment.name, attachment.mime_type, reference
        ),
    }
}

fn run_with_permissions(
    compiled: &forge::runtime::executor::CompiledGraph<ContextState>,
    gate: Arc<PermissionSession>,
    state: ContextState,
) -> Result<ContextState, GraphError> {
    match block_on(compiled.invoke_resumable(state))? {
        ExecutionResult::Complete(state) => Ok(state),
        ExecutionResult::Interrupted { checkpoint, .. } => {
            gate.apply_reply("perm:report", PermissionReply::Always);
            let resumed = block_on(compiled.resume(checkpoint, ResumeCommand::new("always")))?;
            match resumed {
                ExecutionResult::Complete(state) => Ok(state),
                ExecutionResult::Interrupted { .. } => {
                    Err(GraphError::Other("run is still interrupted".to_string()))
                }
            }
        }
    }
}

fn main() -> Result<(), GraphError> {
    let gate = Arc::new(PermissionSession::new(PermissionPolicy::new(vec![
        PermissionRule::new(PermissionDecision::Ask, vec!["perm:report".to_string()]),
    ])));

    let mut registry = ToolRegistry::new();
    registry.register(
        "report",
        Arc::new(|call, ctx: ToolContext| {
            Box::pin(async move {
                ctx.ask_permission("perm:report")?;
                if call.input.get("abort").and_then(|value| value.as_bool()) == Some(true) {
                    return ctx.abort("user requested abort");
                }

                let summary = serde_json::json!({
                    "title": "Weekly report",
                    "items": ["alpha", "beta", "gamma"],
                });
                let big_blob = serde_json::json!("this payload is larger than the inline limit");

                Ok(ToolOutput::text("report ready")
                    .with_mime_type("text/plain")
                    .with_attachment(ToolAttachment::inline(
                        "summary.json",
                        "application/json",
                        summary,
                    ))
                    .with_attachment(ToolAttachment::inline("blob.txt", "text/plain", big_blob)))
            })
        }),
    );

    let registry = Arc::new(registry);
    let node = LoopNode::with_tools_and_gate_and_policy(
        "tool-context",
        Arc::clone(&registry),
        Arc::clone(&gate),
        AttachmentPolicy::new(16),
        |mut state: ContextState, ctx| async move {
            let output = ctx
                .run_tool(ToolCall::new(
                    "report",
                    "call-1",
                    serde_json::json!({"abort": state.abort}),
                ))
                .await?;
            state.logs.push(format!("content: {}", output.content));
            for attachment in &output.attachments {
                state.logs.push(describe_attachment(attachment));
            }
            Ok(state)
        },
    );

    let mut graph = StateGraph::<ContextState>::new();
    graph.add_node_spec(node.into_node());
    graph.add_edge(START, "tool-context");
    graph.add_edge("tool-context", END);

    let compiled = graph.compile()?;

    let ok_state = run_with_permissions(&compiled, Arc::clone(&gate), ContextState::default())?;
    println!("Success logs: {:?}", ok_state.logs);

    let abort_state = ContextState {
        abort: true,
        ..ContextState::default()
    };
    let abort_result = run_with_permissions(&compiled, Arc::clone(&gate), abort_state);
    match abort_result {
        Err(GraphError::Aborted { reason }) => {
            println!("Run aborted: {}", reason);
        }
        Err(err) => return Err(err),
        Ok(_) => println!("Abort run completed unexpectedly"),
    }

    Ok(())
}
