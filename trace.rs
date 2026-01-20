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
}

#[cfg(test)]
mod tests {
    use super::{ExecutionTrace, TraceEvent, TraceReplay, TraceSpan};

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
}
