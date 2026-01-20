use std::sync::Arc;

use crate::langgraph::error::GraphResult;
use crate::langgraph::event::{Event, EventSink};
use crate::langgraph::node::NodeSpec;
use crate::langgraph::state::GraphState;
use crate::langgraph::tool::{ToolCall, ToolRegistry};

/// LoopContext bundles tool registry + event sink for loop handlers.
#[derive(Clone)]
pub struct LoopContext {
    sink: Arc<dyn EventSink>,
    tools: Arc<ToolRegistry>,
}

impl LoopContext {
    pub fn new(sink: Arc<dyn EventSink>, tools: Arc<ToolRegistry>) -> Self {
        Self { sink, tools }
    }

    pub fn emit(&self, event: Event) {
        self.sink.emit(event);
    }

    pub async fn run_tool(&self, call: ToolCall) -> GraphResult<String> {
        self.tools
            .run_with_events(call, Arc::clone(&self.sink))
            .await
    }
}

/// LoopNode is the OpenCode-style streaming loop abstraction.
///
/// It is intentionally minimal in Phase 2: a callable unit that emits events
/// and returns updated state, and can be converted into a stream-capable node.
pub struct LoopNode<S: GraphState> {
    name: String,
    tools: Arc<ToolRegistry>,
    handler: Arc<dyn Fn(S, LoopContext) -> crate::langgraph::node::BoxFuture<'static, GraphResult<S>> + Send + Sync>,
}

impl<S: GraphState> LoopNode<S> {
    pub fn new<F, Fut>(name: impl Into<String>, handler: F) -> Self
    where
        F: Fn(S, LoopContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = GraphResult<S>> + Send + 'static,
    {
        let tools = Arc::new(ToolRegistry::new());
        Self::with_tools(name, tools, handler)
    }

    pub fn with_tools<F, Fut>(
        name: impl Into<String>,
        tools: Arc<ToolRegistry>,
        handler: F,
    ) -> Self
    where
        F: Fn(S, LoopContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = GraphResult<S>> + Send + 'static,
    {
        Self {
            name: name.into(),
            tools,
            handler: Arc::new(move |state, ctx| Box::pin(handler(state, ctx))),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn run(&self, state: S, sink: Arc<dyn EventSink>) -> crate::langgraph::node::BoxFuture<'static, GraphResult<S>> {
        let ctx = LoopContext::new(sink, Arc::clone(&self.tools));
        (self.handler)(state, ctx)
    }

    pub fn into_node(self) -> NodeSpec<S> {
        let handler = Arc::clone(&self.handler);
        let tools = Arc::clone(&self.tools);
        NodeSpec::new_stream(self.name, move |state, sink| {
            let ctx = LoopContext::new(sink, Arc::clone(&tools));
            handler(state, ctx)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::langgraph::event::{Event, EventSink};
    use crate::langgraph::state::GraphState;
    use crate::langgraph::tool::{ToolCall, ToolRegistry};
    use std::sync::{Arc, Mutex};
    use futures::executor::block_on;

    #[derive(Clone, Default)]
    struct LoopState {
        log: Vec<String>,
    }

    impl GraphState for LoopState {}

    struct CaptureSink {
        events: Arc<Mutex<Vec<Event>>>,
    }

    impl EventSink for CaptureSink {
        fn emit(&self, event: Event) {
            self.events.lock().unwrap().push(event);
        }
    }

    #[test]
    fn loop_node_emits_events() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });

        let node = LoopNode::new("loop", |mut state: LoopState, ctx| async move {
            ctx.emit(Event::TextDelta {
                session_id: "s1".to_string(),
                message_id: "m1".to_string(),
                delta: "hello".to_string(),
            });
            state.log.push("emitted".to_string());
            Ok(state)
        });

        let result = block_on(node.run(LoopState::default(), sink)).expect("run");
        assert_eq!(result.log, vec!["emitted".to_string()]);
        assert_eq!(events.lock().unwrap().len(), 1);
    }

    #[test]
    fn loop_node_runs_tools_via_registry() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });
        let mut registry = ToolRegistry::new();
        registry.register("echo", Arc::new(|call| {
            Box::pin(async move { Ok(format!("ok:{}", call.tool)) })
        }));
        let registry = Arc::new(registry);

        let node = LoopNode::with_tools("loop", Arc::clone(&registry), |mut state: LoopState, ctx| async move {
            let output = ctx
                .run_tool(ToolCall::new("echo", "call-1", serde_json::json!({"msg": "hi"})))
                .await?;
            state.log.push(output);
            Ok(state)
        });

        let result = block_on(node.run(LoopState::default(), sink)).expect("run");
        assert_eq!(result.log, vec!["ok:echo".to_string()]);
        assert!(events
            .lock()
            .unwrap()
            .iter()
            .any(|event| matches!(event, Event::ToolResult { .. })));
    }
}
