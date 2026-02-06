//! Trace data structures for runtime replay.

use crate::runtime::error::GraphResult;
use serde::{Deserialize, Serialize};

/// A trace event capturing high-level execution activity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TraceEvent {
    NodeStart {
        node: String,
    },
    NodeFinish {
        node: String,
    },
    Compacted {
        summary: String,
        truncated_before: usize,
    },
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
    const RECORD_LOG_VERSION: u32 = 1;

    pub fn replay(trace: &ExecutionTrace) -> Vec<TraceEvent> {
        trace.events.clone()
    }

    pub fn replay_to_sink(
        trace: &ExecutionTrace,
        sink: &dyn crate::runtime::event::EventSink,
    ) -> GraphResult<()> {
        for event in &trace.events {
            let runtime_event = map_trace_event(event);
            sink.emit(runtime_event)?;
        }
        Ok(())
    }

    pub fn replay_to_record_sink(
        trace: &ExecutionTrace,
        sink: &dyn crate::runtime::event::EventRecordSink,
    ) -> GraphResult<()> {
        let sequencer = crate::runtime::event::EventSequencer::new();
        for event in &trace.events {
            let runtime_event = map_trace_event(event);
            let record = sequencer.record(runtime_event);
            sink.emit_record(record)?;
        }
        Ok(())
    }

    pub fn replay_to_record_sink_with_start_seq(
        trace: &ExecutionTrace,
        sink: &dyn crate::runtime::event::EventRecordSink,
        start_seq: u64,
    ) -> GraphResult<()> {
        let sequencer = crate::runtime::event::EventSequencer::with_start_seq(start_seq);
        for event in &trace.events {
            let runtime_event = map_trace_event(event);
            let record = sequencer.record(runtime_event);
            sink.emit_record(record)?;
        }
        Ok(())
    }

    pub fn replay_to_record_sink_with_existing(
        trace: &ExecutionTrace,
        sink: &dyn crate::runtime::event::EventRecordSink,
        existing: &[crate::runtime::event::EventRecord],
    ) -> GraphResult<()> {
        let start_seq = crate::runtime::event::max_record_seq(existing).unwrap_or(0);
        let sequencer = crate::runtime::event::EventSequencer::with_start_seq(start_seq);
        for event in &trace.events {
            let runtime_event = map_trace_event(event);
            let record = sequencer.record(runtime_event);
            sink.emit_record(record)?;
        }
        Ok(())
    }

    pub fn replay_to_record_sink_with_record_log(
        trace: &ExecutionTrace,
        sink: &dyn crate::runtime::event::EventRecordSink,
        path: impl AsRef<std::path::Path>,
    ) -> std::io::Result<()> {
        let existing = Self::read_audit_log_records(path)?;
        Self::replay_to_record_sink_with_existing(trace, sink, &existing)
            .map_err(std::io::Error::other)?;
        Ok(())
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
        let json = serde_json::json!({
            "version": Self::RECORD_LOG_VERSION,
            "records": Self::replay_to_record_json(trace),
        });
        let data = serde_json::to_string_pretty(&json)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, data)?;
        Ok(())
    }

    pub fn read_audit_log_records(
        path: impl AsRef<std::path::Path>,
    ) -> std::io::Result<Vec<crate::runtime::event::EventRecord>> {
        let contents = std::fs::read_to_string(path)?;
        let value: serde_json::Value = serde_json::from_str(&contents)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        match value {
            serde_json::Value::Array(_) => {
                let mut records: Vec<crate::runtime::event::EventRecord> =
                    serde_json::from_value(value)
                        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
                crate::runtime::event::sort_records_by_meta(&mut records);
                Ok(records)
            }
            serde_json::Value::Object(mut obj) => {
                let version = obj
                    .remove("version")
                    .and_then(|value| value.as_u64())
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "missing record log version",
                        )
                    })?;
                if version != Self::RECORD_LOG_VERSION as u64 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "unsupported record log version",
                    ));
                }
                let records_value = obj.remove("records").ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "missing record log records",
                    )
                })?;
                let mut records: Vec<crate::runtime::event::EventRecord> =
                    serde_json::from_value(records_value)
                        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
                crate::runtime::event::sort_records_by_meta(&mut records);
                Ok(records)
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid record log format",
            )),
        }
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
        fn emit(&self, event: Event) -> crate::runtime::error::GraphResult<()> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
    }

    #[derive(Debug)]
    struct CaptureRecordSink {
        records: Arc<Mutex<Vec<EventRecord>>>,
    }

    impl EventRecordSink for CaptureRecordSink {
        fn emit_record(&self, record: EventRecord) -> crate::runtime::error::GraphResult<()> {
            self.records.lock().unwrap().push(record);
            Ok(())
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

        TraceReplay::replay_to_sink(&trace, &sink).expect("replay");

        let events = events.lock().unwrap();
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::StepStart { .. })));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::SessionCompacted { .. })));
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

        TraceReplay::replay_to_record_sink(&trace, &sink).expect("replay");

        let captured = records.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert!(captured
            .iter()
            .all(|record| !record.meta.event_id.is_empty()));
        assert!(captured.iter().all(|record| record.meta.timestamp_ms > 0));
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

        TraceReplay::replay_to_record_sink_with_start_seq(&trace, &sink, 99).expect("replay");

        let captured = records.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].meta.seq, 100);
    }

    #[test]
    fn trace_replay_records_with_existing_offset() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(TraceEvent::NodeStart {
            node: "a".to_string(),
        });

        let existing = vec![
            EventRecord::with_meta(
                Event::StepStart {
                    session_id: "s1".to_string(),
                },
                crate::runtime::event::EventMeta {
                    event_id: "e1".to_string(),
                    timestamp_ms: 1,
                    seq: 7,
                },
            ),
            EventRecord::with_meta(
                Event::StepStart {
                    session_id: "s1".to_string(),
                },
                crate::runtime::event::EventMeta {
                    event_id: "e2".to_string(),
                    timestamp_ms: 2,
                    seq: 9,
                },
            ),
        ];

        let records = Arc::new(Mutex::new(Vec::new()));
        let sink = CaptureRecordSink {
            records: Arc::clone(&records),
        };

        TraceReplay::replay_to_record_sink_with_existing(&trace, &sink, &existing).expect("replay");

        let captured = records.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].meta.seq, 10);
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
        assert!(
            array[1]["meta"]["seq"].as_u64().unwrap() > array[0]["meta"]["seq"].as_u64().unwrap()
        );
    }

    #[test]
    fn trace_replay_write_audit_log() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(TraceEvent::NodeStart {
            node: "a".to_string(),
        });
        let path = std::env::temp_dir().join(format!("forge-audit-{}.json", uuid::Uuid::new_v4()));
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
        let path =
            std::env::temp_dir().join(format!("forge-audit-records-{}.json", uuid::Uuid::new_v4()));
        TraceReplay::write_audit_log_records(&trace, &path).expect("write");
        let contents = std::fs::read_to_string(path).expect("read");
        assert!(contents.contains("\"meta\""));
        assert!(contents.contains("\"event_id\""));
    }

    #[test]
    fn trace_replay_read_audit_log_records_round_trip() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(TraceEvent::NodeStart {
            node: "a".to_string(),
        });
        let path = std::env::temp_dir().join(format!(
            "forge-audit-records-read-{}.json",
            uuid::Uuid::new_v4()
        ));
        TraceReplay::write_audit_log_records(&trace, &path).expect("write");

        let records = TraceReplay::read_audit_log_records(&path).expect("read");

        assert_eq!(records.len(), 1);
        assert!(matches!(records[0].event, Event::StepStart { .. }));
        assert!(!records[0].meta.event_id.is_empty());
    }

    #[test]
    fn trace_replay_read_audit_log_records_supports_legacy_array() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(TraceEvent::NodeStart {
            node: "a".to_string(),
        });
        let path = std::env::temp_dir().join(format!(
            "forge-audit-records-legacy-{}.json",
            uuid::Uuid::new_v4()
        ));
        let legacy = TraceReplay::replay_to_record_json(&trace);
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&legacy).expect("serialize"),
        )
        .expect("write");

        let records = TraceReplay::read_audit_log_records(&path).expect("read");

        assert_eq!(records.len(), 1);
        assert!(matches!(records[0].event, Event::StepStart { .. }));
    }

    #[test]
    fn trace_replay_read_audit_log_records_rejects_unknown_version() {
        let path = std::env::temp_dir().join(format!(
            "forge-audit-records-badver-{}.json",
            uuid::Uuid::new_v4()
        ));
        let value = serde_json::json!({
            "version": 2,
            "records": []
        });
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&value).expect("serialize"),
        )
        .expect("write");

        let err = TraceReplay::read_audit_log_records(&path).expect_err("error");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn trace_replay_records_with_record_log_file() {
        let mut trace = ExecutionTrace::new();
        trace.record_event(TraceEvent::NodeStart {
            node: "a".to_string(),
        });
        let path = std::env::temp_dir().join(format!(
            "forge-audit-records-replay-{}.json",
            uuid::Uuid::new_v4()
        ));
        TraceReplay::write_audit_log_records(&trace, &path).expect("write");

        let records = Arc::new(Mutex::new(Vec::new()));
        let sink = CaptureRecordSink {
            records: Arc::clone(&records),
        };

        TraceReplay::replay_to_record_sink_with_record_log(&trace, &sink, &path).expect("replay");

        let captured = records.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].meta.seq, 2);
    }

    #[test]
    fn trace_replay_read_audit_log_records_sorts_by_seq() {
        let path = std::env::temp_dir().join(format!(
            "forge-audit-records-sort-{}.json",
            uuid::Uuid::new_v4()
        ));
        let records = vec![
            EventRecord::with_meta(
                Event::StepStart {
                    session_id: "s1".to_string(),
                },
                crate::runtime::event::EventMeta {
                    event_id: "b".to_string(),
                    timestamp_ms: 2,
                    seq: 2,
                },
            ),
            EventRecord::with_meta(
                Event::StepStart {
                    session_id: "s1".to_string(),
                },
                crate::runtime::event::EventMeta {
                    event_id: "a".to_string(),
                    timestamp_ms: 1,
                    seq: 1,
                },
            ),
        ];
        let value = serde_json::json!({
            "version": 1,
            "records": records,
        });
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&value).expect("serialize"),
        )
        .expect("write");

        let read = TraceReplay::read_audit_log_records(&path).expect("read");

        assert_eq!(read.len(), 2);
        assert_eq!(read[0].meta.seq, 1);
        assert_eq!(read[1].meta.seq, 2);
    }
}
