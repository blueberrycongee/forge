use std::any::Any;
use std::sync::{Arc, Mutex};

use forge::runtime::constants::{END, START};
use forge::runtime::error::{interrupt, GraphError, ResumeCommand};
use forge::runtime::event::{Event, EventSink};
use forge::runtime::executor::{CheckpointDurability, ExecutionConfig, ExecutionResult};
use forge::runtime::graph::StateGraph;
use forge::runtime::session::CheckpointStore;
use forge::runtime::state::GraphState;
use futures::executor::block_on;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
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
    fn emit(&self, event: Event) -> Result<(), GraphError> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct MultiPauseState {
    steps: usize,
    first_resume: Option<serde_json::Value>,
    second_resume: Option<serde_json::Value>,
    validate_resume: Option<serde_json::Value>,
}

impl GraphState for MultiPauseState {
    fn get(&self, key: &str) -> Option<&dyn Any> {
        match key {
            "resume:first" => self.first_resume.as_ref().map(|value| value as &dyn Any),
            "resume:second" => self.second_resume.as_ref().map(|value| value as &dyn Any),
            "resume:validate" => self.validate_resume.as_ref().map(|value| value as &dyn Any),
            _ => None,
        }
    }

    fn set(&mut self, key: &str, value: Box<dyn Any + Send + Sync>) {
        match key {
            "resume:first" => {
                if let Ok(value) = value.downcast::<serde_json::Value>() {
                    self.first_resume = Some(*value);
                }
            }
            "resume:second" => {
                if let Ok(value) = value.downcast::<serde_json::Value>() {
                    self.second_resume = Some(*value);
                }
            }
            "resume:validate" => {
                if let Ok(value) = value.downcast::<serde_json::Value>() {
                    self.validate_resume = Some(*value);
                }
            }
            _ => {}
        }
    }
}

async fn first_pause_node(state: MultiPauseState) -> Result<MultiPauseState, GraphError> {
    if state.first_resume.is_some() {
        return Ok(state);
    }
    interrupt("first pause", "first")
}

async fn second_pause_node(state: MultiPauseState) -> Result<MultiPauseState, GraphError> {
    if state.second_resume.is_some() {
        return Ok(state);
    }
    interrupt("second pause", "second")
}

async fn validate_resume_node(state: MultiPauseState) -> Result<MultiPauseState, GraphError> {
    let value = state
        .validate_resume
        .as_ref()
        .and_then(|value| value.as_str());
    if value == Some("approved") {
        return Ok(state);
    }
    interrupt("invalid resume value", "validate")
}

async fn finish_multi_node(mut state: MultiPauseState) -> Result<MultiPauseState, GraphError> {
    state.steps += 1;
    Ok(state)
}

#[test]
fn pause_and_resume_from_checkpoint() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink: Arc<dyn EventSink> = Arc::new(CaptureSink {
        events: events.clone(),
    });

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

    let resumed =
        block_on(compiled.resume(checkpoint, ResumeCommand::new("continue"))).expect("resume");

    let final_state = match resumed {
        ExecutionResult::Complete(state) => state,
        _ => panic!("expected completion"),
    };

    assert_eq!(final_state.steps, 1);
    assert!(final_state.resume.is_some());

    let captured = events.lock().unwrap();
    assert!(captured
        .iter()
        .any(|event| matches!(event, Event::RunStarted { .. })));
    assert!(captured
        .iter()
        .any(|event| matches!(event, Event::RunPaused { .. })));
    assert!(captured
        .iter()
        .any(|event| matches!(event, Event::RunResumed { .. })));
    assert!(captured
        .iter()
        .any(|event| matches!(event, Event::RunCompleted { .. })));
}

#[test]
fn pause_resume_handles_multiple_interrupt_nodes() {
    let mut graph = StateGraph::<MultiPauseState>::new();
    graph.add_node("first", first_pause_node);
    graph.add_node("second", second_pause_node);
    graph.add_node("finish", finish_multi_node);
    graph.add_edge(START, "first");
    graph.add_edge("first", "second");
    graph.add_edge("second", "finish");
    graph.add_edge("finish", END);

    let compiled = graph.compile().expect("compile");

    let first = block_on(compiled.invoke_resumable(MultiPauseState::default())).expect("run");
    let checkpoint_1 = match first {
        ExecutionResult::Interrupted { checkpoint, .. } => checkpoint,
        _ => panic!("expected first interrupt"),
    };
    assert_eq!(checkpoint_1.next_node, "first");

    let second = block_on(compiled.resume(checkpoint_1, ResumeCommand::new("continue-1")))
        .expect("resume first");
    let checkpoint_2 = match second {
        ExecutionResult::Interrupted { checkpoint, .. } => checkpoint,
        _ => panic!("expected second interrupt"),
    };
    assert_eq!(checkpoint_2.next_node, "second");

    let final_result = block_on(compiled.resume(checkpoint_2, ResumeCommand::new("continue-2")))
        .expect("resume second");
    let state = match final_result {
        ExecutionResult::Complete(state) => state,
        _ => panic!("expected completion"),
    };

    assert_eq!(state.steps, 1);
    assert!(state.first_resume.is_some());
    assert!(state.second_resume.is_some());
}

#[test]
fn pause_resume_reinterrupts_on_invalid_resume_value() {
    let mut graph = StateGraph::<MultiPauseState>::new();
    graph.add_node("validate", validate_resume_node);
    graph.add_node("finish", finish_multi_node);
    graph.add_edge(START, "validate");
    graph.add_edge("validate", "finish");
    graph.add_edge("finish", END);

    let compiled = graph.compile().expect("compile");

    let first = block_on(compiled.invoke_resumable(MultiPauseState::default())).expect("run");
    let checkpoint_1 = match first {
        ExecutionResult::Interrupted { checkpoint, .. } => checkpoint,
        _ => panic!("expected initial interrupt"),
    };

    let second = block_on(compiled.resume(checkpoint_1, ResumeCommand::new("invalid")))
        .expect("resume with invalid value");
    let checkpoint_2 = match second {
        ExecutionResult::Interrupted { checkpoint, .. } => checkpoint,
        _ => panic!("expected interrupt after invalid resume"),
    };

    let third = block_on(compiled.resume(checkpoint_2, ResumeCommand::new("approved")))
        .expect("resume with valid value");
    let state = match third {
        ExecutionResult::Complete(state) => state,
        _ => panic!("expected completion"),
    };
    assert_eq!(state.steps, 1);
}

#[test]
fn pause_resume_allows_reusing_same_checkpoint_deterministically() {
    let mut graph = StateGraph::<PauseState>::new();
    graph.add_node("pause", pause_node);
    graph.add_node("finish", finish_node);
    graph.add_edge(START, "pause");
    graph.add_edge("pause", "finish");
    graph.add_edge("finish", END);

    let compiled = graph.compile().expect("compile");

    let first = block_on(compiled.invoke_resumable(PauseState::default())).expect("run");
    let checkpoint = match first {
        ExecutionResult::Interrupted { checkpoint, .. } => checkpoint,
        _ => panic!("expected interrupt"),
    };

    let resumed_once =
        block_on(compiled.resume(checkpoint.clone(), ResumeCommand::new("continue")))
            .expect("resume once");
    let resumed_twice = block_on(compiled.resume(checkpoint, ResumeCommand::new("continue")))
        .expect("resume twice");

    let first_state = match resumed_once {
        ExecutionResult::Complete(state) => state,
        _ => panic!("expected first completion"),
    };
    let second_state = match resumed_twice {
        ExecutionResult::Complete(state) => state,
        _ => panic!("expected second completion"),
    };

    assert_eq!(first_state.steps, 1);
    assert_eq!(second_state.steps, 1);
}

#[test]
fn pause_resume_can_continue_from_persisted_checkpoint_after_restart() {
    let build_graph = || {
        let mut graph = StateGraph::<PauseState>::new();
        graph.add_node("pause", pause_node);
        graph.add_node("finish", finish_node);
        graph.add_edge(START, "pause");
        graph.add_edge("pause", "finish");
        graph.add_edge("finish", END);
        graph
    };

    let store_root =
        std::env::temp_dir().join(format!("forge-checkpoint-resume-{}", uuid::Uuid::new_v4()));
    let store = Arc::new(CheckpointStore::new(store_root));
    let config = ExecutionConfig::new()
        .with_checkpoint_store(Arc::clone(&store))
        .with_checkpoint_durability(CheckpointDurability::Sync);

    let compiled_1 = build_graph()
        .compile()
        .expect("compile")
        .with_config(config.clone());
    let first = block_on(compiled_1.invoke_resumable(PauseState::default())).expect("run");
    let checkpoint = match first {
        ExecutionResult::Interrupted { checkpoint, .. } => checkpoint,
        _ => panic!("expected interrupt"),
    };

    let checkpoints = store.list(&checkpoint.run_id).expect("list checkpoints");
    assert!(checkpoints.contains(&checkpoint.checkpoint_id));

    let compiled_2 = build_graph()
        .compile()
        .expect("compile")
        .with_config(config);
    let resumed = block_on(compiled_2.resume_from_store(
        &checkpoint.run_id,
        &checkpoint.checkpoint_id,
        Some(ResumeCommand::new("continue")),
    ))
    .expect("resume from store");

    let final_state = match resumed {
        ExecutionResult::Complete(state) => state,
        _ => panic!("expected completion"),
    };
    assert_eq!(final_state.steps, 1);
}
