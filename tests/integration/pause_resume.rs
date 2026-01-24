use std::any::Any;
use std::sync::{Arc, Mutex};

use forge::runtime::constants::{END, START};
use forge::runtime::error::{interrupt, GraphError, ResumeCommand};
use forge::runtime::event::{Event, EventSink};
use forge::runtime::executor::{ExecutionConfig, ExecutionResult};
use forge::runtime::graph::StateGraph;
use forge::runtime::state::GraphState;
use futures::executor::block_on;

#[derive(Clone, Default)]
struct PauseState {
    steps: usize,
    resume: Option<serde_json::Value>,
}

impl GraphState for PauseState {
    fn get(&self, key: &str) -> Option<&dyn Any> {
        if key == "resume:pause" {
            return self.resume.as_ref().map(|value| value as &dyn Any);
        }
        None
    }

    fn set(&mut self, key: &str, value: Box<dyn Any + Send + Sync>) {
        if key == "resume:pause" {
            if let Ok(value) = value.downcast::<serde_json::Value>() {
                self.resume = Some(*value);
            }
        }
    }
}

async fn pause_node(state: PauseState) -> Result<PauseState, GraphError> {
    if state.resume.is_some() {
        return Ok(state);
    }
    interrupt("paused", "pause")
}

async fn finish_node(mut state: PauseState) -> Result<PauseState, GraphError> {
    state.steps += 1;
    Ok(state)
}

struct CaptureSink {
    events: Arc<Mutex<Vec<Event>>>,
}

impl EventSink for CaptureSink {
    fn emit(&self, event: Event) {
        self.events.lock().unwrap().push(event);
    }
}

#[test]
fn pause_and_resume_from_checkpoint() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink: Arc<dyn EventSink> = Arc::new(CaptureSink { events: events.clone() });

    let mut graph = StateGraph::<PauseState>::new();
    graph.add_node("pause", pause_node);
    graph.add_node("finish", finish_node);
    graph.add_edge(START, "pause");
    graph.add_edge("pause", "finish");
    graph.add_edge("finish", END);

    let compiled = graph
        .compile()
        .expect("compile")
        .with_config(ExecutionConfig::new().with_run_event_sink(Arc::clone(&sink)));

    let result = block_on(compiled.invoke_resumable(PauseState::default())).expect("run");
    let checkpoint = match result {
        ExecutionResult::Interrupted { checkpoint, .. } => checkpoint,
        _ => panic!("expected interrupt"),
    };

    let resumed = block_on(compiled.resume(checkpoint, ResumeCommand::new("continue")))
        .expect("resume");

    let final_state = match resumed {
        ExecutionResult::Complete(state) => state,
        _ => panic!("expected completion"),
    };

    assert_eq!(final_state.steps, 1);
    assert!(final_state.resume.is_some());

    let captured = events.lock().unwrap();
    assert!(captured.iter().any(|event| matches!(event, Event::RunStarted { .. })));
    assert!(captured.iter().any(|event| matches!(event, Event::RunPaused { .. })));
    assert!(captured.iter().any(|event| matches!(event, Event::RunResumed { .. })));
    assert!(captured.iter().any(|event| matches!(event, Event::RunCompleted { .. })));
}
