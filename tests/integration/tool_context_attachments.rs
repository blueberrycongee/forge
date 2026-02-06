use std::sync::Arc;

use crate::helpers::events::EventCollector;
use forge::runtime::event::Event;
use forge::runtime::prelude::LoopNode;
use forge::runtime::state::GraphState;
use forge::runtime::tool::{ToolAttachment, ToolCall, ToolOutput, ToolRegistry};
use futures::executor::block_on;

#[derive(Clone, Default)]
struct AttachmentState {
    count: usize,
}

impl GraphState for AttachmentState {}

#[test]
fn tool_emits_attachment_events() {
    let collector = EventCollector::new();
    let sink = collector.sink();

    let mut registry = ToolRegistry::new();
    registry.register(
        "emit",
        Arc::new(|_call, _ctx| {
            Box::pin(async move {
                let output = ToolOutput::text("ok").with_attachment(ToolAttachment::inline(
                    "file.txt",
                    "text/plain",
                    serde_json::json!("hello"),
                ));
                Ok(output)
            })
        }),
    );
    let registry = Arc::new(registry);

    let node = LoopNode::with_tools(
        "loop",
        Arc::clone(&registry),
        |mut state: AttachmentState, ctx| async move {
            ctx.run_tool(ToolCall::new("emit", "call-1", serde_json::json!({})))
                .await?;
            state.count += 1;
            Ok(state)
        },
    );

    let result = block_on(node.run(AttachmentState::default(), sink)).expect("run");
    assert_eq!(result.count, 1);

    let events = collector.events();
    assert!(events
        .iter()
        .any(|event| matches!(event, Event::ToolAttachment { .. })));
}
