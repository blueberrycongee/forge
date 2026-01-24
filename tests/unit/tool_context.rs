use std::sync::{Arc, Mutex};

use forge::runtime::error::GraphError;
use forge::runtime::event::{Event, EventSink};
use forge::runtime::permission::{PermissionDecision, PermissionPolicy, PermissionRule, PermissionSession};
use forge::runtime::tool::{AttachmentPolicy, ToolContext};

struct CaptureSink {
    events: Arc<Mutex<Vec<Event>>>,
}

impl EventSink for CaptureSink {
    fn emit(&self, event: Event) {
        self.events.lock().unwrap().push(event);
    }
}

#[test]
fn tool_context_asks_permission_and_interrupts() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });
    let gate = Arc::new(PermissionSession::new(PermissionPolicy::new(vec![
        PermissionRule::new(PermissionDecision::Ask, vec!["perm:danger".to_string()]),
    ])));
    let ctx = ToolContext::new(
        Arc::clone(&sink),
        gate,
        AttachmentPolicy::default(),
        "danger",
        "call-1",
    );

    let result = ctx.ask_permission("perm:danger");
    match result {
        Err(GraphError::Interrupted(interrupts)) => {
            assert_eq!(interrupts.len(), 1);
        }
        other => panic!("expected interrupt, got {:?}", other),
    }

    let captured = events.lock().unwrap();
    assert!(captured.iter().any(|event| matches!(event, Event::PermissionAsked { .. })));
}
