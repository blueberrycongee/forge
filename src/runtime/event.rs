//! Event protocol for streaming execution.
//!
//! This is the foundation for tool-driven runtime events
//! (text/tool/step/permission) so clients can consume a single stream.

use std::cmp::Ordering;
use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::runtime::session_state::{RunStatus, SessionPhase};
use crate::runtime::tool::{ToolAttachment, ToolMetadata, ToolOutput, ToolState};

/// Token usage breakdown (input/output/reasoning/cache).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub reasoning: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

/// Permission reply outcomes.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PermissionReply {
    Once,
    Always,
    Reject,
}

/// Tool update payload for streaming tool output or metadata.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ToolUpdate {
    /// Incremental tool output (stdout/stderr/etc).
    OutputDelta {
        delta: String,
        stream: Option<String>,
    },
    /// Preview payload when output is truncated.
    OutputPreview {
        preview: serde_json::Value,
        truncated: bool,
    },
    /// Tool metadata update (full payload).
    Metadata {
        metadata: ToolMetadata,
    },
    /// Progress updates for long-running tools.
    Progress {
        current: u64,
        total: Option<u64>,
        unit: Option<String>,
        message: Option<String>,
    },
    /// Custom tool update payloads.
    Custom {
        data: serde_json::Value,
    },
}

/// Event metadata for protocol-level fields.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EventMeta {
    pub event_id: String,
    pub timestamp_ms: u64,
    pub seq: u64,
}

impl EventMeta {
    pub fn new(seq: u64) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp_ms: now_ms(),
            seq,
        }
    }
}

/// Event record with protocol metadata.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EventRecord {
    pub meta: EventMeta,
    pub event: Event,
}

impl EventRecord {
    pub fn new(event: Event, seq: u64) -> Self {
        Self {
            meta: EventMeta::new(seq),
            event,
        }
    }

    pub fn with_meta(event: Event, meta: EventMeta) -> Self {
        Self { meta, event }
    }

    pub fn cmp_meta(a: &Self, b: &Self) -> Ordering {
        a.meta
            .seq
            .cmp(&b.meta.seq)
            .then_with(|| a.meta.timestamp_ms.cmp(&b.meta.timestamp_ms))
            .then_with(|| a.meta.event_id.cmp(&b.meta.event_id))
    }
}

pub fn sort_records_by_meta(records: &mut [EventRecord]) {
    records.sort_by(EventRecord::cmp_meta);
}

pub fn max_record_seq(records: &[EventRecord]) -> Option<u64> {
    records.iter().map(|record| record.meta.seq).max()
}

/// Sequencer for assigning event ids, timestamps, and sequence numbers.
#[derive(Debug, Default)]
pub struct EventSequencer {
    next_seq: AtomicU64,
}

impl EventSequencer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_start_seq(start: u64) -> Self {
        Self {
            next_seq: AtomicU64::new(start),
        }
    }

    pub fn record(&self, event: Event) -> EventRecord {
        let seq = self.next_seq.fetch_add(1, AtomicOrdering::Relaxed) + 1;
        EventRecord::new(event, seq)
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Runtime events emitted during execution.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Event {
    RunStarted {
        run_id: String,
        status: RunStatus,
    },
    RunPaused {
        run_id: String,
        checkpoint_id: String,
    },
    RunResumed {
        run_id: String,
        checkpoint_id: String,
    },
    RunCompleted {
        run_id: String,
        status: RunStatus,
    },
    RunFailed {
        run_id: String,
        error: String,
    },
    RunAborted {
        run_id: String,
        reason: String,
    },
    TextDelta {
        session_id: String,
        message_id: String,
        delta: String,
    },
    TextFinal {
        session_id: String,
        message_id: String,
        text: String,
    },
    Attachment {
        session_id: String,
        message_id: String,
        name: String,
        mime_type: String,
        data: serde_json::Value,
    },
    Error {
        session_id: String,
        message_id: String,
        message: String,
    },
    ToolStart {
        tool: String,
        call_id: String,
        input: serde_json::Value,
    },
    ToolUpdate {
        tool: String,
        call_id: String,
        update: ToolUpdate,
    },
    ToolResult {
        tool: String,
        call_id: String,
        output: ToolOutput,
    },
    ToolAttachment {
        tool: String,
        call_id: String,
        attachment: ToolAttachment,
    },
    ToolError {
        tool: String,
        call_id: String,
        error: String,
    },
    ToolStatus {
        tool: String,
        call_id: String,
        state: ToolState,
    },
    StepStart {
        session_id: String,
    },
    StepFinish {
        session_id: String,
        tokens: TokenUsage,
        cost: f64,
    },
    PermissionAsked {
        permission: String,
        patterns: Vec<String>,
        #[serde(default)]
        metadata: serde_json::Map<String, serde_json::Value>,
        #[serde(default)]
        always: Vec<String>,
    },
    PermissionReplied {
        permission: String,
        reply: PermissionReply,
    },
    SessionCompacted {
        session_id: String,
        summary: String,
        truncated_before: usize,
    },
    SessionCompactionRequested {
        session_id: String,
        message_count: usize,
        tokens: TokenUsage,
        context_window: Option<u64>,
        threshold_ratio: Option<f64>,
    },
    SessionPhaseChanged {
        session_id: String,
        message_id: String,
        from: SessionPhase,
        to: SessionPhase,
    },
    SessionPhaseTransitionRejected {
        session_id: String,
        message_id: String,
        from: SessionPhase,
        to: SessionPhase,
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::{
        max_record_seq,
        sort_records_by_meta,
        Event,
        EventMeta,
        EventRecord,
        EventSequencer,
        TokenUsage,
    };
    use crate::runtime::tool::ToolState;

    #[test]
    fn tool_status_event_can_be_emitted() {
        let event = Event::ToolStatus {
            tool: "grep".to_string(),
            call_id: "call-1".to_string(),
            state: ToolState::Running,
        };

        match event {
            Event::ToolStatus { state, .. } => assert_eq!(state, ToolState::Running),
            _ => panic!("expected tool status event"),
        }
    }

    #[test]
    fn tool_update_event_can_be_emitted() {
        let event = Event::ToolUpdate {
            tool: "bash".to_string(),
            call_id: "call-2".to_string(),
            update: super::ToolUpdate::OutputDelta {
                delta: "line".to_string(),
                stream: Some("stdout".to_string()),
            },
        };

        match event {
            Event::ToolUpdate { update, .. } => match update {
                super::ToolUpdate::OutputDelta { delta, stream } => {
                    assert_eq!(delta, "line");
                    assert_eq!(stream.as_deref(), Some("stdout"));
                }
                _ => panic!("expected output delta"),
            },
            _ => panic!("expected tool update event"),
        }
    }

    #[test]
    fn event_sequencer_assigns_metadata() {
        let sequencer = EventSequencer::default();

        let first = sequencer.record(Event::StepStart {
            session_id: "s1".to_string(),
        });
        let second = sequencer.record(Event::StepFinish {
            session_id: "s1".to_string(),
            tokens: TokenUsage::default(),
            cost: 0.0,
        });

        assert!(!first.meta.event_id.is_empty());
        assert!(!second.meta.event_id.is_empty());
        assert_ne!(first.meta.event_id, second.meta.event_id);
        assert!(first.meta.seq < second.meta.seq);
        assert!(first.meta.timestamp_ms > 0);
    }

    #[test]
    fn event_sequencer_starts_after_provided_base() {
        let sequencer = EventSequencer::with_start_seq(41);

        let first = sequencer.record(Event::StepStart {
            session_id: "s1".to_string(),
        });
        let second = sequencer.record(Event::StepStart {
            session_id: "s1".to_string(),
        });

        assert_eq!(first.meta.seq, 42);
        assert_eq!(second.meta.seq, 43);
    }

    #[test]
    fn event_record_holds_meta_and_payload() {
        let meta = EventMeta {
            event_id: "e1".to_string(),
            timestamp_ms: 42,
            seq: 7,
        };
        let record = EventRecord::with_meta(
            Event::StepStart {
                session_id: "s1".to_string(),
            },
            meta.clone(),
        );

        assert_eq!(record.meta, meta);
        assert!(matches!(record.event, Event::StepStart { .. }));
    }

    #[test]
    fn event_record_cmp_orders_by_seq_then_timestamp_then_id() {
        let base = Event::StepStart {
            session_id: "s1".to_string(),
        };
        let record_a = EventRecord::with_meta(
            base.clone(),
            EventMeta {
                event_id: "a".to_string(),
                timestamp_ms: 10,
                seq: 1,
            },
        );
        let record_b = EventRecord::with_meta(
            base.clone(),
            EventMeta {
                event_id: "b".to_string(),
                timestamp_ms: 20,
                seq: 1,
            },
        );
        let record_c = EventRecord::with_meta(
            base,
            EventMeta {
                event_id: "c".to_string(),
                timestamp_ms: 5,
                seq: 2,
            },
        );

        assert!(EventRecord::cmp_meta(&record_a, &record_b).is_lt());
        assert!(EventRecord::cmp_meta(&record_b, &record_c).is_lt());
    }

    #[test]
    fn sort_records_by_meta_orders_records() {
        let base = Event::StepStart {
            session_id: "s1".to_string(),
        };
        let mut records = vec![
            EventRecord::with_meta(
                base.clone(),
                EventMeta {
                    event_id: "b".to_string(),
                    timestamp_ms: 10,
                    seq: 1,
                },
            ),
            EventRecord::with_meta(
                base.clone(),
                EventMeta {
                    event_id: "a".to_string(),
                    timestamp_ms: 10,
                    seq: 1,
                },
            ),
            EventRecord::with_meta(
                base,
                EventMeta {
                    event_id: "c".to_string(),
                    timestamp_ms: 5,
                    seq: 0,
                },
            ),
        ];

        sort_records_by_meta(&mut records);

        let ids: Vec<&str> = records.iter().map(|record| record.meta.event_id.as_str()).collect();
        assert_eq!(ids, vec!["c", "a", "b"]);
    }

    #[test]
    fn max_record_seq_returns_none_for_empty() {
        let records: Vec<EventRecord> = Vec::new();
        assert_eq!(max_record_seq(&records), None);
    }

    #[test]
    fn max_record_seq_returns_max_seq() {
        let base = Event::StepStart {
            session_id: "s1".to_string(),
        };
        let records = vec![
            EventRecord::with_meta(
                base.clone(),
                EventMeta {
                    event_id: "a".to_string(),
                    timestamp_ms: 1,
                    seq: 5,
                },
            ),
            EventRecord::with_meta(
                base,
                EventMeta {
                    event_id: "b".to_string(),
                    timestamp_ms: 2,
                    seq: 9,
                },
            ),
        ];

        assert_eq!(max_record_seq(&records), Some(9));
    }
}

/// Event sink for streaming runtime events to UI/CLI/SSE/etc.
pub trait EventSink: Send + Sync {
    fn emit(&self, event: Event);
}

/// A no-op event sink for tests or silent execution.
pub struct NoopEventSink;

impl EventSink for NoopEventSink {
    fn emit(&self, _event: Event) {}
}

/// Event sink for protocol records (event_id/timestamp/seq).
pub trait EventRecordSink: Send + Sync + Debug {
    fn emit_record(&self, record: EventRecord);
}

/// A no-op event record sink for tests or silent execution.
#[derive(Debug)]
pub struct NoopEventRecordSink;

impl EventRecordSink for NoopEventRecordSink {
    fn emit_record(&self, _record: EventRecord) {}
}
