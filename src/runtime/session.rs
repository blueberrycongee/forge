//! Session snapshot structures for export/import.

use serde::{Deserialize, Serialize};

use crate::runtime::compaction::CompactionResult;
use crate::runtime::trace::ExecutionTrace;

/// Minimal message payload for snapshots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
}

impl SessionMessage {
    pub fn from_message(message: &crate::runtime::message::Message) -> Self {
        let mut content = String::new();
        for part in &message.parts {
            match part {
                crate::runtime::message::Part::TextDelta { delta } => content.push_str(delta),
                crate::runtime::message::Part::TextFinal { text } => content.push_str(text),
                _ => {}
            }
        }

        Self {
            role: message.role.as_str().to_string(),
            content,
        }
    }
}

/// Session snapshot containing messages, trace, and compaction summaries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub version: u32,
    pub session_id: String,
    pub messages: Vec<SessionMessage>,
    pub trace: ExecutionTrace,
    pub compactions: Vec<CompactionResult>,
}

impl SessionSnapshot {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            version: 1,
            session_id: session_id.into(),
            messages: Vec::new(),
            trace: ExecutionTrace::new(),
            compactions: Vec::new(),
        }
    }

    pub fn push_message(&mut self, message: &crate::runtime::message::Message) {
        self.messages.push(SessionMessage::from_message(message));
    }
}

/// Session snapshot IO helpers.
pub struct SessionSnapshotIo;

impl SessionSnapshotIo {
    pub fn to_json(snapshot: &SessionSnapshot) -> serde_json::Value {
        serde_json::to_value(snapshot).expect("serialize")
    }

    pub fn from_json(value: serde_json::Value) -> Result<SessionSnapshot, serde_json::Error> {
        serde_json::from_value(value)
    }

    pub fn to_string(snapshot: &SessionSnapshot) -> String {
        serde_json::to_string_pretty(snapshot).expect("serialize")
    }

    pub fn from_string(input: &str) -> Result<SessionSnapshot, serde_json::Error> {
        serde_json::from_str(input)
    }
}

/// Session snapshot persistence adapter.
pub struct SessionStore {
    root: std::path::PathBuf,
}

impl SessionStore {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn session_dir(&self, session_id: &str) -> std::path::PathBuf {
        self.root.join(session_id)
    }

    pub fn save(&self, snapshot: &SessionSnapshot) -> std::io::Result<()> {
        let dir = self.session_dir(&snapshot.session_id);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("snapshot.json");
        std::fs::write(path, SessionSnapshotIo::to_string(snapshot))?;
        Ok(())
    }

    pub fn load(&self, session_id: &str) -> std::io::Result<SessionSnapshot> {
        let path = self.session_dir(session_id).join("snapshot.json");
        let data = std::fs::read_to_string(path)?;
        let snapshot = SessionSnapshotIo::from_string(&data)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionMessage, SessionSnapshot, SessionSnapshotIo, SessionStore};
    use crate::runtime::compaction::CompactionResult;
    use crate::runtime::message::{Message, MessageRole, Part};
    use crate::runtime::trace::TraceEvent;
    use crate::runtime::tool::ToolOutput;

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

    #[test]
    fn session_snapshot_io_helpers_roundtrip() {
        let snapshot = SessionSnapshot::new("s1");
        let json = SessionSnapshotIo::to_json(&snapshot);
        let decoded = SessionSnapshotIo::from_json(json).expect("decode");
        assert_eq!(snapshot, decoded);
    }

    #[test]
    fn session_snapshot_store_roundtrip() {
        let temp = std::env::temp_dir().join(format!("forge-session-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(temp);
        let snapshot = SessionSnapshot::new("s1");

        store.save(&snapshot).expect("save");
        let loaded = store.load("s1").expect("load");
        assert_eq!(snapshot, loaded);
    }

    #[test]
    fn session_message_from_message_collects_text_parts_in_order() {
        let mut message = Message::new(MessageRole::Assistant);
        message.parts.push(Part::TextDelta {
            delta: "he".to_string(),
        });
        message.parts.push(Part::ToolCall {
            tool: "grep".to_string(),
            call_id: "c1".to_string(),
            input: serde_json::json!({ "q": "hi" }),
        });
        message.parts.push(Part::TextFinal {
            text: "llo".to_string(),
        });
        message.parts.push(Part::TextDelta {
            delta: "!".to_string(),
        });

        let session_message = SessionMessage::from_message(&message);
        assert_eq!(session_message.role, "assistant");
        assert_eq!(session_message.content, "hello!");
    }

    #[test]
    fn session_message_from_message_ignores_non_text_parts() {
        let mut message = Message::new(MessageRole::Tool);
        message.parts.push(Part::ToolResult {
            tool: "read".to_string(),
            call_id: "c1".to_string(),
            output: ToolOutput::text("ok"),
        });
        message.parts.push(Part::Attachment {
            name: "file.txt".to_string(),
            mime_type: "text/plain".to_string(),
            data: serde_json::json!({"size": 4}),
        });

        let session_message = SessionMessage::from_message(&message);
        assert_eq!(session_message.role, "tool");
        assert!(session_message.content.is_empty());
    }

    #[test]
    fn session_snapshot_push_message_appends_converted_entry() {
        let mut message = Message::new(MessageRole::User);
        message.parts.push(Part::TextFinal {
            text: "hi".to_string(),
        });

        let mut snapshot = SessionSnapshot::new("s1");
        snapshot.push_message(&message);

        assert_eq!(
            snapshot.messages,
            vec![SessionMessage {
                role: "user".to_string(),
                content: "hi".to_string(),
            }]
        );
    }
}
