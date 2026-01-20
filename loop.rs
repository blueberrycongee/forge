use std::sync::Arc;

use crate::langgraph::event::{Event, EventSink};
use crate::langgraph::error::{GraphError, GraphResult, Interrupt, ResumeCommand};
use crate::langgraph::permission::{
    PermissionDecision,
    PermissionGate,
    PermissionPolicy,
    PermissionRequest,
    PermissionSession,
};
use crate::langgraph::node::NodeSpec;
use crate::langgraph::state::GraphState;
use crate::langgraph::tool::{ToolCall, ToolOutput, ToolRegistry};

/// LoopContext bundles tool registry + event sink for loop handlers.
#[derive(Clone)]
pub struct LoopContext {
    sink: Arc<dyn EventSink>,
    tools: Arc<ToolRegistry>,
    gate: Arc<PermissionSession>,
}

impl LoopContext {
    pub fn new(sink: Arc<dyn EventSink>, tools: Arc<ToolRegistry>) -> Self {
        Self::new_with_gate(
            sink,
            tools,
            Arc::new(PermissionSession::new(PermissionPolicy::default())),
        )
    }

    pub fn new_with_gate(
        sink: Arc<dyn EventSink>,
        tools: Arc<ToolRegistry>,
        gate: Arc<PermissionSession>,
    ) -> Self {
        Self { sink, tools, gate }
    }

    pub fn emit(&self, event: Event) {
        self.sink.emit(event);
    }

    pub fn reply_permission(
        &self,
        permission: impl Into<String>,
        reply: crate::langgraph::event::PermissionReply,
    ) {
        let permission = permission.into();
        self.gate.apply_reply(&permission, reply.clone());
        self.emit(Event::PermissionReplied { permission, reply });
    }

    pub fn resume_permission(
        &self,
        permission: impl Into<String>,
        command: &ResumeCommand,
    ) -> Option<crate::langgraph::event::PermissionReply> {
        let permission = permission.into();
        let reply = self.gate.apply_resume(&permission, command)?;
        self.emit(Event::PermissionReplied {
            permission,
            reply: reply.clone(),
        });
        Some(reply)
    }

    pub async fn run_tool(&self, call: ToolCall) -> GraphResult<ToolOutput> {
        let permission = format!("tool:{}", call.tool);
        match self.gate.decide(&permission) {
            PermissionDecision::Allow => {
                self.tools
                    .run_with_events(call, Arc::clone(&self.sink))
                    .await
            }
            PermissionDecision::Ask => {
                self.emit(Event::PermissionAsked {
                    permission: permission.clone(),
                    patterns: vec![permission.clone()],
                });
                Err(GraphError::Interrupted(vec![Interrupt::new(
                    PermissionRequest {
                        permission: permission.clone(),
                        patterns: vec![permission.clone()],
                    },
                    format!("permission:{}", permission),
                )]))
            }
            PermissionDecision::Deny => Err(GraphError::ExecutionError {
                node: format!("permission:{}", permission),
                message: "permission denied".to_string(),
            }),
        }
    }
}

/// LoopNode is the OpenCode-style streaming loop abstraction.
///
/// It is intentionally minimal in Phase 2: a callable unit that emits events
/// and returns updated state, and can be converted into a stream-capable node.
pub struct LoopNode<S: GraphState> {
    name: String,
    tools: Arc<ToolRegistry>,
    gate: Arc<PermissionSession>,
    handler: Arc<
        dyn Fn(S, LoopContext) -> crate::langgraph::node::BoxFuture<'static, GraphResult<S>>
            + Send
            + Sync,
    >,
}

impl<S: GraphState> LoopNode<S> {
    pub fn new<F, Fut>(name: impl Into<String>, handler: F) -> Self
    where
        F: Fn(S, LoopContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = GraphResult<S>> + Send + 'static,
    {
        let tools = Arc::new(ToolRegistry::new());
        let gate = Arc::new(PermissionSession::new(PermissionPolicy::default()));
        Self::with_tools_and_gate(name, tools, gate, handler)
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
        let gate = Arc::new(PermissionSession::new(PermissionPolicy::default()));
        Self::with_tools_and_gate(name, tools, gate, handler)
    }

    pub fn with_tools_and_gate<F, Fut>(
        name: impl Into<String>,
        tools: Arc<ToolRegistry>,
        gate: Arc<PermissionSession>,
        handler: F,
    ) -> Self
    where
        F: Fn(S, LoopContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = GraphResult<S>> + Send + 'static,
    {
        Self {
            name: name.into(),
            tools,
            gate,
            handler: Arc::new(move |state, ctx| Box::pin(handler(state, ctx))),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn run(&self, state: S, sink: Arc<dyn EventSink>) -> crate::langgraph::node::BoxFuture<'static, GraphResult<S>> {
        let ctx = LoopContext::new_with_gate(
            sink,
            Arc::clone(&self.tools),
            Arc::clone(&self.gate),
        );
        (self.handler)(state, ctx)
    }

    pub fn into_node(self) -> NodeSpec<S> {
        let handler = Arc::clone(&self.handler);
        let tools = Arc::clone(&self.tools);
        let gate = Arc::clone(&self.gate);
        NodeSpec::new_stream(self.name, move |state, sink| {
            let ctx = LoopContext::new_with_gate(
                sink,
                Arc::clone(&tools),
                Arc::clone(&gate),
            );
            handler(state, ctx)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::langgraph::event::{Event, EventSink};
    use crate::langgraph::permission::{PermissionDecision, PermissionPolicy, PermissionRule, PermissionSession};
    use crate::langgraph::state::GraphState;
    use crate::langgraph::tool::{ToolCall, ToolOutput, ToolRegistry};
    use std::sync::{Arc, Mutex};
    use futures::executor::block_on;

    #[derive(Clone, Default, Debug)]
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
            Box::pin(async move { Ok(ToolOutput::text(format!("ok:{}", call.tool))) })
        }));
        let registry = Arc::new(registry);

        let node = LoopNode::with_tools("loop", Arc::clone(&registry), |mut state: LoopState, ctx| async move {
            let output = ctx
                .run_tool(ToolCall::new("echo", "call-1", serde_json::json!({"msg": "hi"})))
                .await?;
            let text = output.content.as_str().unwrap_or_default().to_string();
            state.log.push(text);
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

    #[test]
    fn loop_context_asks_permission_for_tool() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });
        let mut registry = ToolRegistry::new();
        registry.register("echo", Arc::new(|call| {
            Box::pin(async move { Ok(ToolOutput::text(format!("ok:{}", call.tool))) })
        }));
        let registry = Arc::new(registry);
        let gate = Arc::new(PermissionSession::new(PermissionPolicy::new(vec![PermissionRule::new(
            PermissionDecision::Ask,
            vec!["tool:echo".to_string()],
        )])));

        let node = LoopNode::with_tools_and_gate(
            "loop",
            Arc::clone(&registry),
            gate,
            |state: LoopState, ctx| async move {
                ctx.run_tool(ToolCall::new("echo", "call-ask", serde_json::json!({})))
                    .await?;
                Ok(state)
            },
        );

        let result = block_on(node.run(LoopState::default(), sink));
        match result {
            Err(GraphError::Interrupted(interrupts)) => {
                assert_eq!(interrupts.len(), 1);
                let value = &interrupts[0].value;
                let request: PermissionRequest =
                    serde_json::from_value(value.clone()).expect("permission request");
                assert_eq!(request.permission, "tool:echo");
            }
            other => panic!("expected interrupted, got {:?}", other),
        }
        let captured = events.lock().unwrap();
        assert!(captured.iter().any(|event| matches!(event, Event::PermissionAsked { .. })));
        assert!(!captured.iter().any(|event| matches!(event, Event::ToolStart { .. })));
    }

    #[test]
    fn loop_context_allows_after_permission_reply() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });
        let mut registry = ToolRegistry::new();
        registry.register("echo", Arc::new(|call| {
            Box::pin(async move { Ok(ToolOutput::text(format!("ok:{}", call.tool))) })
        }));
        let registry = Arc::new(registry);
        let gate = Arc::new(PermissionSession::new(PermissionPolicy::new(vec![PermissionRule::new(
            PermissionDecision::Ask,
            vec!["tool:echo".to_string()],
        )])));

        let node = LoopNode::with_tools_and_gate(
            "loop",
            Arc::clone(&registry),
            gate,
            |state: LoopState, ctx| async move {
                ctx.reply_permission("tool:echo", crate::langgraph::event::PermissionReply::Once);
                ctx.run_tool(ToolCall::new("echo", "call-ok", serde_json::json!({})))
                    .await?;
                Ok(state)
            },
        );

        let result = block_on(node.run(LoopState::default(), sink));
        assert!(result.is_ok());
        let captured = events.lock().unwrap();
        assert!(captured
            .iter()
            .any(|event| matches!(event, Event::PermissionReplied { .. })));
        assert!(captured
            .iter()
            .any(|event| matches!(event, Event::ToolResult { .. })));
    }

    #[test]
    fn loop_context_resumes_permission_from_command() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });
        let mut registry = ToolRegistry::new();
        registry.register("echo", Arc::new(|call| {
            Box::pin(async move { Ok(ToolOutput::text(format!("ok:{}", call.tool))) })
        }));
        let registry = Arc::new(registry);
        let gate = Arc::new(PermissionSession::new(PermissionPolicy::new(vec![PermissionRule::new(
            PermissionDecision::Ask,
            vec!["tool:echo".to_string()],
        )])));
        let resume = ResumeCommand::new("once");

        let node = LoopNode::with_tools_and_gate(
            "loop",
            Arc::clone(&registry),
            gate,
            move |state: LoopState, ctx| {
                let resume = resume.clone();
                async move {
                    ctx.resume_permission("tool:echo", &resume);
                ctx.run_tool(ToolCall::new("echo", "call-resume", serde_json::json!({})))
                    .await?;
                Ok(state)
                }
            },
        );

        let result = block_on(node.run(LoopState::default(), sink));
        assert!(result.is_ok());
        let captured = events.lock().unwrap();
        assert!(captured
            .iter()
            .any(|event| matches!(event, Event::PermissionReplied { .. })));
    }
}
