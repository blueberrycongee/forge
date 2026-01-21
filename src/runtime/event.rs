//! Event protocol for streaming execution.
//!
//! This is the foundation for OpenCode-style runtime events
//! (text/tool/step/permission) so clients can consume a single stream.

use std::cmp::Ordering;
use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::runtime::session_state::SessionPhase;
use crate::runtime::tool::{ToolOutput, ToolState};

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

/// Sequencer for assigning event ids, timestamps, and sequence numbers.
#[derive(Debug, Default)]
pub struct EventSequencer {
    next_seq: AtomicU64,
}

impl EventSequencer {
    pub fn new() -> Self {
        Self::default()
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
    ToolResult {
        tool: String,
        call_id: String,
        output: ToolOutput,
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
    use super::{sort_records_by_meta, Event, EventMeta, EventRecord, EventSequencer, TokenUsage};
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
