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
            let runtime_event = map_trace_event(event);
            sink.emit(runtime_event);
        }
    }

    pub fn replay_to_record_sink(
        trace: &ExecutionTrace,
        sink: &dyn crate::runtime::event::EventRecordSink,
    ) {
        let sequencer = crate::runtime::event::EventSequencer::new();
        for event in &trace.events {
            let runtime_event = map_trace_event(event);
            let record = sequencer.record(runtime_event);
            sink.emit_record(record);
        }
    }

    pub fn replay_to_record_sink_with_start_seq(
        trace: &ExecutionTrace,
        sink: &dyn crate::runtime::event::EventRecordSink,
        start_seq: u64,
    ) {
        let sequencer = crate::runtime::event::EventSequencer::with_start_seq(start_seq);
        for event in &trace.events {
            let runtime_event = map_trace_event(event);
            let record = sequencer.record(runtime_event);
            sink.emit_record(record);
        }
    }

    pub fn replay_to_json(trace: &ExecutionTrace) -> serde_json::Value {
        let mut events = Vec::new();
        for event in &trace.events {
            let runtime_event = map_trace_event(event);
            events.push(serde_json::to_value(runtime_event).expect("serialize"));
        }
        serde_json::Value::Array(events)
    }

    pub fn replay_to_record_json(trace: &ExecutionTrace) -> serde_json::Value {
        let sequencer = crate::runtime::event::EventSequencer::new();
        let mut records = Vec::new();
        for event in &trace.events {
            let runtime_event = map_trace_event(event);
            let record = sequencer.record(runtime_event);
            records.push(serde_json::to_value(record).expect("serialize"));
        }
        serde_json::Value::Array(records)
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

    pub fn write_audit_log_records(
        trace: &ExecutionTrace,
        path: impl AsRef<std::path::Path>,
    ) -> std::io::Result<()> {
        let json = Self::replay_to_record_json(trace);
        let data = serde_json::to_string_pretty(&json)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, data)?;
        Ok(())
    }
}

fn map_trace_event(event: &TraceEvent) -> crate::runtime::event::Event {
    match event {
        TraceEvent::NodeStart { node } => crate::runtime::event::Event::StepStart {
            session_id: node.clone(),
        },
        TraceEvent::NodeFinish { node } => crate::runtime::event::Event::StepFinish {
            session_id: node.clone(),
            tokens: crate::runtime::event::TokenUsage::default(),
            cost: 0.0,
        },
        TraceEvent::Compacted {
            summary,
            truncated_before,
        } => crate::runtime::event::Event::SessionCompacted {
            session_id: "replay".to_string(),
            summary: summary.clone(),
            truncated_before: *truncated_before,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{ExecutionTrace, TraceEvent, TraceReplay, TraceSpan};
    use crate::runtime::event::{Event, EventRecord, EventRecordSink, EventSink};
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

    #[derive(Debug)]
    struct CaptureRecordSink {
        records: Arc<Mutex<Vec<EventRecord>>>,
    }

    impl EventRecordSink for CaptureRecordSink {
        fn emit_record(&self, record: EventRecord) {
            self.records.lock().unwrap().push(record);
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
    fn trace_replay_emits_event_records_with_metadata() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(TraceEvent::NodeStart {
            node: "a".to_string(),
        });
        trace.record_event(TraceEvent::NodeFinish {
            node: "a".to_string(),
        });

        let records = Arc::new(Mutex::new(Vec::new()));
        let sink = CaptureRecordSink {
            records: Arc::clone(&records),
        };

        TraceReplay::replay_to_record_sink(&trace, &sink);

        let captured = records.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert!(captured
            .iter()
            .all(|record| !record.meta.event_id.is_empty()));
        assert!(captured
            .iter()
            .all(|record| record.meta.timestamp_ms > 0));
        assert!(captured[0].meta.seq < captured[1].meta.seq);
        assert!(matches!(captured[0].event, Event::StepStart { .. }));
        assert!(matches!(captured[1].event, Event::StepFinish { .. }));
    }

    #[test]
    fn trace_replay_records_with_start_seq_offset() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(TraceEvent::NodeStart {
            node: "a".to_string(),
        });

        let records = Arc::new(Mutex::new(Vec::new()));
        let sink = CaptureRecordSink {
            records: Arc::clone(&records),
        };

        TraceReplay::replay_to_record_sink_with_start_seq(&trace, &sink, 99);

        let captured = records.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].meta.seq, 100);
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
    fn trace_replay_to_record_json_emits_array_with_metadata() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(TraceEvent::NodeStart {
            node: "a".to_string(),
        });
        trace.record_event(TraceEvent::NodeFinish {
            node: "a".to_string(),
        });

        let json = TraceReplay::replay_to_record_json(&trace);
        let array = json.as_array().expect("array");
        assert_eq!(array.len(), 2);
        assert!(array[0].get("meta").is_some());
        assert!(array[0]["meta"].get("event_id").is_some());
        assert!(array[0]["meta"].get("timestamp_ms").is_some());
        assert!(array[0]["meta"].get("seq").is_some());
        assert!(array[1]["meta"]["seq"].as_u64().unwrap() > array[0]["meta"]["seq"].as_u64().unwrap());
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

    #[test]
    fn trace_replay_write_audit_log_records() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(TraceEvent::NodeStart {
            node: "a".to_string(),
        });
        let path = std::env::temp_dir().join(format!(
            "forge-audit-records-{}.json",
            uuid::Uuid::new_v4()
        ));
        TraceReplay::write_audit_log_records(&trace, &path).expect("write");
        let contents = std::fs::read_to_string(path).expect("read");
        assert!(contents.contains("\"meta\""));
        assert!(contents.contains("\"event_id\""));
    }
}
