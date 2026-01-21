use std::sync::{Arc, Mutex};

use crate::runtime::event::{Event, EventSink};
use crate::runtime::error::{GraphError, GraphResult, Interrupt, ResumeCommand};
use crate::runtime::message::MessageRole;
use crate::runtime::permission::{
    PermissionDecision,
    PermissionGate,
    PermissionPolicy,
    PermissionRequest,
    PermissionSession,
};
use crate::runtime::node::NodeSpec;
use crate::runtime::session_state::SessionState;
use crate::runtime::state::GraphState;
use crate::runtime::tool::{ToolCall, ToolOutput, ToolRegistry};

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
        reply: crate::runtime::event::PermissionReply,
    ) {
        let permission = permission.into();
        self.gate.apply_reply(&permission, reply.clone());
        self.emit(Event::PermissionReplied { permission, reply });
    }

    pub fn resume_permission(
        &self,
        permission: impl Into<String>,
        command: &ResumeCommand,
    ) -> Option<crate::runtime::event::PermissionReply> {
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
        dyn Fn(S, LoopContext) -> crate::runtime::node::BoxFuture<'static, GraphResult<S>>
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

    pub fn run(&self, state: S, sink: Arc<dyn EventSink>) -> crate::runtime::node::BoxFuture<'static, GraphResult<S>> {
        let ctx = LoopContext::new_with_gate(
            sink,
            Arc::clone(&self.tools),
            Arc::clone(&self.gate),
        );
        (self.handler)(state, ctx)
    }

    pub fn run_with_session_state(
        &self,
        state: S,
        session_state: Arc<Mutex<SessionState>>,
        sink: Arc<dyn EventSink>,
    ) -> crate::runtime::node::BoxFuture<'static, GraphResult<S>> {
        let sink: Arc<dyn EventSink> = Arc::new(SessionStateSink::new(sink, session_state));
        self.run(state, sink)
    }

    pub fn run_with_session_state_and_finalize(
        &self,
        state: S,
        session_state: Arc<Mutex<SessionState>>,
        sink: Arc<dyn EventSink>,
        role: MessageRole,
    ) -> crate::runtime::node::BoxFuture<'static, GraphResult<S>> {
        let sink: Arc<dyn EventSink> = Arc::new(SessionStateSink::new(
            sink,
            Arc::clone(&session_state),
        ));
        let fut = self.run(state, sink);
        Box::pin(async move {
            let result = fut.await?;
            session_state.lock().unwrap().finalize_message(role);
            Ok(result)
        })
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

struct SessionStateSink {
    inner: Arc<dyn EventSink>,
    session_state: Arc<Mutex<SessionState>>,
}

impl SessionStateSink {
    fn new(inner: Arc<dyn EventSink>, session_state: Arc<Mutex<SessionState>>) -> Self {
        Self {
            inner,
            session_state,
        }
    }
}

impl EventSink for SessionStateSink {
    fn emit(&self, event: Event) {
        self.session_state.lock().unwrap().apply_event(&event);
        self.inner.emit(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::event::{Event, EventSink};
    use crate::runtime::permission::{PermissionDecision, PermissionPolicy, PermissionRule, PermissionSession};
    use crate::runtime::session_state::{SessionState, ToolCallStatus};
    use crate::runtime::state::GraphState;
    use crate::runtime::tool::{ToolCall, ToolOutput, ToolRegistry};
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
                ctx.reply_permission("tool:echo", crate::runtime::event::PermissionReply::Once);
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

    #[test]
    fn loop_node_updates_session_state_from_events() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });
        let session_state = Arc::new(Mutex::new(SessionState::new("s1", "m1")));

        let node = LoopNode::new("loop", |state: LoopState, ctx| async move {
            ctx.emit(Event::TextDelta {
                session_id: "s1".to_string(),
                message_id: "m1".to_string(),
                delta: "hi".to_string(),
            });
            ctx.emit(Event::ToolStart {
                tool: "read".to_string(),
                call_id: "c1".to_string(),
                input: serde_json::json!({"path": "file.txt"}),
            });
            ctx.emit(Event::ToolResult {
                tool: "read".to_string(),
                call_id: "c1".to_string(),
                output: ToolOutput::text("ok"),
            });
            Ok(state)
        });

        let result = block_on(node.run_with_session_state(
            LoopState::default(),
            Arc::clone(&session_state),
            sink,
        ));
        assert!(result.is_ok());

        let session_state = session_state.lock().unwrap();
        assert_eq!(session_state.pending_parts.len(), 3);
        assert_eq!(session_state.tool_calls.len(), 1);
        assert_eq!(session_state.tool_calls[0].status, ToolCallStatus::Completed);
    }

    #[test]
    fn loop_node_finalizes_session_state_message_after_run() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });
        let session_state = Arc::new(Mutex::new(SessionState::new("s1", "m1")));

        let node = LoopNode::new("loop", |state: LoopState, ctx| async move {
            ctx.emit(Event::TextDelta {
                session_id: "s1".to_string(),
                message_id: "m1".to_string(),
                delta: "he".to_string(),
            });
            ctx.emit(Event::TextFinal {
                session_id: "s1".to_string(),
                message_id: "m1".to_string(),
                text: "llo".to_string(),
            });
            Ok(state)
        });

        let result = block_on(node.run_with_session_state_and_finalize(
            LoopState::default(),
            Arc::clone(&session_state),
            sink,
            MessageRole::Assistant,
        ));
        assert!(result.is_ok());

        let session_state = session_state.lock().unwrap();
        assert!(session_state.pending_parts.is_empty());
        assert_eq!(session_state.messages.len(), 1);
        assert_eq!(session_state.messages[0].role, MessageRole::Assistant);
        assert_eq!(session_state.messages[0].parts.len(), 2);
    }

    #[test]
    fn loop_node_finalize_noop_when_no_pending_parts() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });
        let session_state = Arc::new(Mutex::new(SessionState::new("s1", "m1")));

        let node = LoopNode::new("loop", |state: LoopState, _ctx| async move { Ok(state) });

        let result = block_on(node.run_with_session_state_and_finalize(
            LoopState::default(),
            Arc::clone(&session_state),
            sink,
            MessageRole::Assistant,
        ));
        assert!(result.is_ok());

        let session_state = session_state.lock().unwrap();
        assert!(session_state.messages.is_empty());
    }
}
