//! Session snapshot structures for export/import.

use serde::{Deserialize, Serialize};

use crate::langgraph::compaction::CompactionResult;
use crate::langgraph::trace::ExecutionTrace;

/// Minimal message payload for snapshots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
}

/// Session snapshot containing messages, trace, and compaction summaries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub session_id: String,
    pub messages: Vec<SessionMessage>,
    pub trace: ExecutionTrace,
    pub compactions: Vec<CompactionResult>,
}

impl SessionSnapshot {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            messages: Vec::new(),
            trace: ExecutionTrace::new(),
            compactions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionMessage, SessionSnapshot};
    use crate::langgraph::compaction::CompactionResult;
    use crate::langgraph::trace::TraceEvent;

    #[test]
    fn session_snapshot_roundtrip() {
        let mut snapshot = SessionSnapshot::new("s1");
        snapshot.messages.push(SessionMessage {
            role: "user".to_string(),
            content: "hi".to_string(),
        });
        snapshot.trace.record_event(TraceEvent::NodeStart {
            node: "n1".to_string(),
        });
        snapshot.compactions.push(CompactionResult::new("summary", 1));

        let json = serde_json::to_value(&snapshot).expect("serialize");
        let decoded: SessionSnapshot = serde_json::from_value(json).expect("deserialize");
        assert_eq!(snapshot, decoded);
    }
}
