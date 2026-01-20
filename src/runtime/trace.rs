//! Trace data structures for runtime replay.

use serde::{Deserialize, Serialize};

/// A trace event capturing high-level execution activity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TraceEvent {
    NodeStart { node: String },
    NodeFinish { node: String },
    Compacted { summary: String, truncated_before: usize },
}

/// Span covering a node execution window.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceSpan {
    pub node: String,
    pub start_ms: u64,
    pub duration_ms: u64,
}

/// Execution trace container.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub events: Vec<TraceEvent>,
    pub spans: Vec<TraceSpan>,
}

impl ExecutionTrace {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_event(&mut self, event: TraceEvent) {
        self.events.push(event);
    }

    pub fn record_span(&mut self, span: TraceSpan) {
        self.spans.push(span);
    }
}

/// Replay trace events in order.
#[derive(Clone, Debug, Default)]
pub struct TraceReplay;

impl TraceReplay {
    pub fn replay(trace: &ExecutionTrace) -> Vec<TraceEvent> {
        trace.events.clone()
    }

    pub fn replay_to_sink(
        trace: &ExecutionTrace,
        sink: &dyn crate::runtime::event::EventSink,
    ) {
        for event in &trace.events {
            let runtime_event = match event {
                TraceEvent::NodeStart { node } => crate::runtime::event::Event::StepStart {
                    session_id: node.clone(),
                },
                TraceEvent::NodeFinish { node } => crate::runtime::event::Event::StepFinish {
                    session_id: node.clone(),
                    tokens: crate::runtime::event::TokenUsage::default(),
                    cost: 0.0,
                },
                TraceEvent::Compacted { summary, truncated_before } => {
                    crate::runtime::event::Event::SessionCompacted {
                        session_id: "replay".to_string(),
                        summary: summary.clone(),
                        truncated_before: *truncated_before,
                    }
                }
            };
            sink.emit(runtime_event);
        }
    }

    pub fn replay_to_json(trace: &ExecutionTrace) -> serde_json::Value {
        let mut events = Vec::new();
        for event in &trace.events {
            let runtime_event = match event {
                TraceEvent::NodeStart { node } => crate::runtime::event::Event::StepStart {
                    session_id: node.clone(),
                },
                TraceEvent::NodeFinish { node } => crate::runtime::event::Event::StepFinish {
                    session_id: node.clone(),
                    tokens: crate::runtime::event::TokenUsage::default(),
                    cost: 0.0,
                },
                TraceEvent::Compacted { summary, truncated_before } => {
                    crate::runtime::event::Event::SessionCompacted {
                        session_id: "replay".to_string(),
                        summary: summary.clone(),
                        truncated_before: *truncated_before,
                    }
                }
            };
            events.push(serde_json::to_value(runtime_event).expect("serialize"));
        }
        serde_json::Value::Array(events)
    }

    pub fn write_audit_log(
        trace: &ExecutionTrace,
        path: impl AsRef<std::path::Path>,
    ) -> std::io::Result<()> {
        let json = Self::replay_to_json(trace);
        let data = serde_json::to_string_pretty(&json)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ExecutionTrace, TraceEvent, TraceReplay, TraceSpan};
    use crate::runtime::event::{Event, EventSink};
    use std::sync::{Arc, Mutex};

    #[test]
    fn trace_records_events_and_spans() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(TraceEvent::NodeStart {
            node: "n1".to_string(),
        });
        trace.record_span(TraceSpan {
            node: "n1".to_string(),
            start_ms: 10,
            duration_ms: 42,
        });

        assert_eq!(trace.events.len(), 1);
        assert_eq!(trace.spans.len(), 1);
    }

    #[test]
    fn trace_roundtrip() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(TraceEvent::Compacted {
            summary: "s".to_string(),
            truncated_before: 2,
        });
        let json = serde_json::to_value(&trace).expect("serialize");
        let decoded: ExecutionTrace = serde_json::from_value(json).expect("deserialize");
        assert_eq!(trace, decoded);
    }

    #[test]
    fn trace_replay_returns_events_in_order() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(TraceEvent::NodeStart {
            node: "a".to_string(),
        });
        trace.record_event(TraceEvent::NodeFinish {
            node: "a".to_string(),
        });
        let replayed = TraceReplay::replay(&trace);
        assert_eq!(replayed, trace.events);
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
    fn trace_replay_emits_events() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(TraceEvent::NodeStart {
            node: "a".to_string(),
        });
        trace.record_event(TraceEvent::Compacted {
            summary: "s".to_string(),
            truncated_before: 1,
        });

        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = CaptureSink {
            events: Arc::clone(&events),
        };

        TraceReplay::replay_to_sink(&trace, &sink);

        let events = events.lock().unwrap();
        assert!(events.iter().any(|event| matches!(event, Event::StepStart { .. })));
        assert!(events.iter().any(|event| matches!(event, Event::SessionCompacted { .. })));
    }

    #[test]
    fn trace_replay_to_json_emits_array() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(TraceEvent::NodeStart {
            node: "a".to_string(),
        });
        let json = TraceReplay::replay_to_json(&trace);
        assert!(json.is_array());
        assert_eq!(json.as_array().unwrap().len(), 1);
    }

    #[test]
    fn trace_replay_write_audit_log() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(TraceEvent::NodeStart {
            node: "a".to_string(),
        });
        let path = std::env::temp_dir().join(format!(
            "forge-audit-{}.json",
            uuid::Uuid::new_v4()
        ));
        TraceReplay::write_audit_log(&trace, &path).expect("write");
        let contents = std::fs::read_to_string(path).expect("read");
        assert!(contents.contains("StepStart"));
    }
}
