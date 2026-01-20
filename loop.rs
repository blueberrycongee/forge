use std::sync::Arc;

use crate::langgraph::error::GraphResult;
use crate::langgraph::event::EventSink;
use crate::langgraph::node::NodeSpec;
use crate::langgraph::state::GraphState;

/// LoopNode is the OpenCode-style streaming loop abstraction.
///
/// It is intentionally minimal in Phase 2: a callable unit that emits events
/// and returns updated state, and can be converted into a stream-capable node.
pub struct LoopNode<S: GraphState> {
    name: String,
    handler: Arc<dyn Fn(S, Arc<dyn EventSink>) -> crate::langgraph::node::BoxFuture<'static, GraphResult<S>> + Send + Sync>,
}

impl<S: GraphState> LoopNode<S> {
    pub fn new<F, Fut>(name: impl Into<String>, handler: F) -> Self
    where
        F: Fn(S, Arc<dyn EventSink>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = GraphResult<S>> + Send + 'static,
    {
        Self {
            name: name.into(),
            handler: Arc::new(move |state, sink| Box::pin(handler(state, sink))),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn run(&self, state: S, sink: Arc<dyn EventSink>) -> crate::langgraph::node::BoxFuture<'static, GraphResult<S>> {
        (self.handler)(state, sink)
    }

    pub fn into_node(self) -> NodeSpec<S> {
        NodeSpec::new_stream(self.name, move |state, sink| (self.handler)(state, sink))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::langgraph::event::{Event, EventSink};
    use crate::langgraph::state::GraphState;
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

        let node = LoopNode::new("loop", |mut state: LoopState, sink| async move {
            sink.emit(Event::TextDelta {
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
}
