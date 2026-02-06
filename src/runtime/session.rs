//! Session snapshot structures for export/import.

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::runtime::compaction::CompactionResult;
use crate::runtime::error::{GraphError, Interrupt};
use crate::runtime::event::EventRecord;
use crate::runtime::executor::Checkpoint;
use crate::runtime::tool::{AttachmentStore, ToolAttachment};
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

    pub fn to_message(&self) -> Option<crate::runtime::message::Message> {
        let role = crate::runtime::message::MessageRole::parse(&self.role)?;
        let mut message = crate::runtime::message::Message::new(role);
        if !self.content.is_empty() {
            message.parts.push(crate::runtime::message::Part::TextFinal {
                text: self.content.clone(),
            });
        }
        Some(message)
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
        let entry = SessionMessage::from_message(message);
        if !entry.content.is_empty() {
            self.messages.push(entry);
        }
    }

    pub fn to_messages(&self) -> Vec<crate::runtime::message::Message> {
        self.messages
            .iter()
            .filter_map(SessionMessage::to_message)
            .collect()
    }
}


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CheckpointRecord {
    pub run_id: String,
    pub checkpoint_id: String,
    pub created_at: String,
    pub state: serde_json::Value,
    pub next_node: String,
    pub iterations: usize,
    pub pending_interrupts: Vec<Interrupt>,
    pub resume_values: HashMap<String, serde_json::Value>,
}

impl CheckpointRecord {
    pub fn new(
        run_id: impl Into<String>,
        checkpoint_id: impl Into<String>,
        state: serde_json::Value,
        next_node: impl Into<String>,
        iterations: usize,
        pending_interrupts: Vec<Interrupt>,
        resume_values: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            checkpoint_id: checkpoint_id.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            state,
            next_node: next_node.into(),
            iterations,
            pending_interrupts,
            resume_values,
        }
    }

    pub fn from_checkpoint<S: Serialize>(
        run_id: impl Into<String>,
        checkpoint_id: impl Into<String>,
        checkpoint: &Checkpoint<S>,
    ) -> Result<Self, serde_json::Error> {
        let state = serde_json::to_value(&checkpoint.state)?;
        Ok(Self {
            run_id: run_id.into(),
            checkpoint_id: checkpoint_id.into(),
            created_at: checkpoint.created_at.clone(),
            state,
            next_node: checkpoint.next_node.clone(),
            iterations: checkpoint.iterations,
            pending_interrupts: checkpoint.pending_interrupts.clone(),
            resume_values: checkpoint.resume_values.clone(),
        })
    }

    pub fn to_checkpoint<S: for<'de> Deserialize<'de>>(
        &self,
    ) -> Result<Checkpoint<S>, serde_json::Error> {
        let state = serde_json::from_value(self.state.clone())?;
        Ok(Checkpoint {
            run_id: self.run_id.clone(),
            checkpoint_id: self.checkpoint_id.clone(),
            created_at: self.created_at.clone(),
            state,
            next_node: self.next_node.clone(),
            pending_interrupts: self.pending_interrupts.clone(),
            iterations: self.iterations,
            resume_values: self.resume_values.clone(),
        })
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

    pub fn load_messages(
        &self,
        session_id: &str,
    ) -> std::io::Result<Vec<crate::runtime::message::Message>> {
        let snapshot = self.load(session_id)?;
        Ok(snapshot.to_messages())
    }
}


/// Append-only run log store (JSONL).
pub struct RunLogStore {
    root: std::path::PathBuf,
}

impl RunLogStore {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn run_dir(&self, run_id: &str) -> std::path::PathBuf {
        self.root.join(run_id)
    }

    fn log_path(&self, run_id: &str) -> std::path::PathBuf {
        self.run_dir(run_id).join("events.jsonl")
    }

    pub fn append(&self, run_id: &str, record: &EventRecord) -> std::io::Result<()> {
        let dir = self.run_dir(run_id);
        std::fs::create_dir_all(&dir)?;
        let path = self.log_path(run_id);
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let line = serde_json::to_string(record)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    pub fn load(&self, run_id: &str) -> std::io::Result<Vec<EventRecord>> {
        let path = self.log_path(run_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let contents = std::fs::read_to_string(path)?;
        let mut records = Vec::new();
        for line in contents.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let record: EventRecord = serde_json::from_str(line)
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
            records.push(record);
        }
        Ok(records)
    }
}

/// File-backed checkpoint store for resumable runs.
pub struct CheckpointStore {
    root: std::path::PathBuf,
}

impl CheckpointStore {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn run_dir(&self, run_id: &str) -> std::path::PathBuf {
        self.root.join(run_id)
    }

    fn checkpoint_dir(&self, run_id: &str) -> std::path::PathBuf {
        self.run_dir(run_id).join("checkpoints")
    }

    fn checkpoint_path(&self, run_id: &str, checkpoint_id: &str) -> std::path::PathBuf {
        self.checkpoint_dir(run_id)
            .join(format!("{}.json", checkpoint_id))
    }

    pub fn save(&self, record: &CheckpointRecord) -> std::io::Result<()> {
        let dir = self.checkpoint_dir(&record.run_id);
        std::fs::create_dir_all(&dir)?;
        let path = self.checkpoint_path(&record.run_id, &record.checkpoint_id);
        let payload = serde_json::to_string_pretty(record)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        std::fs::write(path, payload)?;
        Ok(())
    }

    pub fn load(&self, run_id: &str, checkpoint_id: &str) -> std::io::Result<CheckpointRecord> {
        let path = self.checkpoint_path(run_id, checkpoint_id);
        let data = std::fs::read_to_string(path)?;
        let record = serde_json::from_str(&data)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        Ok(record)
    }

    pub fn list(&self, run_id: &str) -> std::io::Result<Vec<String>> {
        let dir = self.checkpoint_dir(run_id);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                    entries.push(stem.to_string());
                }
            }
        }
        entries.sort();
        Ok(entries)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AttachmentRecord {
    pub attachment_id: String,
    pub created_at: String,
    pub attachment: ToolAttachment,
}

impl AttachmentRecord {
    pub fn new(attachment_id: impl Into<String>, attachment: ToolAttachment) -> Self {
        Self {
            attachment_id: attachment_id.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            attachment,
        }
    }
}

/// File-backed attachment store for reference payloads.
pub struct FileAttachmentStore {
    root: PathBuf,
}

impl FileAttachmentStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn attachments_dir(&self) -> PathBuf {
        self.root.join("attachments")
    }

    fn attachment_path(&self, attachment_id: &str) -> PathBuf {
        self.attachments_dir().join(format!("{}.json", attachment_id))
    }

    pub fn save(&self, record: &AttachmentRecord) -> std::io::Result<()> {
        let dir = self.attachments_dir();
        std::fs::create_dir_all(&dir)?;
        let path = self.attachment_path(&record.attachment_id);
        let payload = serde_json::to_string_pretty(record)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        std::fs::write(path, payload)?;
        Ok(())
    }

    pub fn load(&self, attachment_id: &str) -> std::io::Result<AttachmentRecord> {
        let path = self.attachment_path(attachment_id);
        let data = std::fs::read_to_string(path)?;
        let record = serde_json::from_str(&data)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        Ok(record)
    }
}

impl AttachmentStore for FileAttachmentStore {
    fn store(&self, attachment: &ToolAttachment) -> Result<String, GraphError> {
        let attachment_id = uuid::Uuid::new_v4().to_string();
        let record = AttachmentRecord::new(attachment_id.clone(), attachment.clone());
        self.save(&record).map_err(|err| {
            GraphError::Other(format!("attachment store error: {}", err))
        })?;
        Ok(format!("attachment://{}", attachment_id))
    }
}

/// Resolves attachment references persisted by `FileAttachmentStore`.
pub struct AttachmentResolver {
    store: FileAttachmentStore,
}

impl AttachmentResolver {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            store: FileAttachmentStore::new(root),
        }
    }

    pub fn resolve_reference(&self, reference: &str) -> std::io::Result<AttachmentRecord> {
        let attachment_id = Self::parse_reference(reference).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid attachment reference",
            )
        })?;
        self.store.load(attachment_id)
    }

    pub fn resolve_id(&self, attachment_id: &str) -> std::io::Result<AttachmentRecord> {
        let attachment_id = Self::validate_id(attachment_id).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid attachment id",
            )
        })?;
        self.store.load(attachment_id)
    }

    pub fn parse_reference(reference: &str) -> Option<&str> {
        let attachment_id = reference.strip_prefix("attachment://")?;
        Self::validate_id(attachment_id)
    }

    fn validate_id(attachment_id: &str) -> Option<&str> {
        if attachment_id.is_empty() {
            return None;
        }
        let mut components = std::path::Path::new(attachment_id).components();
        match components.next() {
            Some(std::path::Component::Normal(_)) => {
                if components.next().is_none() {
                    Some(attachment_id)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AttachmentResolver, SessionMessage, SessionSnapshot, SessionSnapshotIo, SessionStore};
    use crate::runtime::compaction::CompactionResult;
    use crate::runtime::message::{Message, MessageRole, Part};
    use crate::runtime::trace::{ExecutionTrace, TraceEvent};
    use crate::runtime::tool::{AttachmentStore, ToolAttachment, ToolOutput};

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
    fn session_store_load_messages_restores_text_parts() {
        let temp = std::env::temp_dir().join(format!("forge-session-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(temp);
        let snapshot = SessionSnapshot {
            version: 1,
            session_id: "s1".to_string(),
            messages: vec![
                SessionMessage {
                    role: "user".to_string(),
                    content: "hi".to_string(),
                },
                SessionMessage {
                    role: "assistant".to_string(),
                    content: "hello".to_string(),
                },
                SessionMessage {
                    role: "unknown".to_string(),
                    content: "skip".to_string(),
                },
            ],
            trace: ExecutionTrace::new(),
            compactions: Vec::new(),
        };

        store.save(&snapshot).expect("save");
        let messages = store.load_messages("s1").expect("load messages");

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(
            messages[0].parts,
            vec![Part::TextFinal {
                text: "hi".to_string()
            }]
        );
        assert_eq!(messages[1].role, MessageRole::Assistant);
        assert_eq!(
            messages[1].parts,
            vec![Part::TextFinal {
                text: "hello".to_string()
            }]
        );
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

    #[test]
    fn session_message_to_message_builds_text_final_parts() {
        let session_message = SessionMessage {
            role: "assistant".to_string(),
            content: "hello".to_string(),
        };

        let message = session_message.to_message().expect("message");
        assert_eq!(message.role, MessageRole::Assistant);
        assert_eq!(
            message.parts,
            vec![Part::TextFinal {
                text: "hello".to_string()
            }]
        );
    }

    #[test]
    fn session_message_to_message_skips_empty_content() {
        let session_message = SessionMessage {
            role: "user".to_string(),
            content: "".to_string(),
        };

        let message = session_message.to_message().expect("message");
        assert_eq!(message.role, MessageRole::User);
        assert!(message.parts.is_empty());
    }

    #[test]
    fn session_snapshot_to_messages_filters_unknown_roles() {
        let snapshot = SessionSnapshot {
            version: 1,
            session_id: "s1".to_string(),
            messages: vec![
                SessionMessage {
                    role: "user".to_string(),
                    content: "hi".to_string(),
                },
                SessionMessage {
                    role: "weird".to_string(),
                    content: "skip".to_string(),
                },
            ],
            trace: ExecutionTrace::new(),
            compactions: Vec::new(),
        };

        let messages = snapshot.to_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(
            messages[0].parts,
            vec![Part::TextFinal {
                text: "hi".to_string()
            }]
        );
    }

    #[test]
    fn session_snapshot_push_message_skips_empty_content() {
        let mut message = Message::new(MessageRole::Tool);
        message.parts.push(Part::ToolResult {
            tool: "read".to_string(),
            call_id: "c1".to_string(),
            output: ToolOutput::text("ok"),
        });

        let mut snapshot = SessionSnapshot::new("s1");
        snapshot.push_message(&message);

        assert!(snapshot.messages.is_empty());
    }

    #[test]
    fn attachment_resolver_loads_reference_payload() {
        let temp = std::env::temp_dir().join(format!("forge-attach-{}", uuid::Uuid::new_v4()));
        let store = super::FileAttachmentStore::new(temp.clone());
        let attachment = ToolAttachment::inline(
            "notes.txt",
            "text/plain",
            serde_json::json!({"ok": true}),
        );
        let reference = store.store(&attachment).expect("store");

        let resolver = AttachmentResolver::new(temp);
        let record = resolver.resolve_reference(&reference).expect("resolve");

        assert_eq!(record.attachment, attachment);
        assert_eq!(AttachmentResolver::parse_reference(&reference), Some(record.attachment_id.as_str()));
    }

    #[test]
    fn attachment_resolver_rejects_invalid_references() {
        assert!(AttachmentResolver::parse_reference("attachment://").is_none());
        assert!(AttachmentResolver::parse_reference("attachment://../bad").is_none());
        assert!(AttachmentResolver::parse_reference("http://bad").is_none());
    }
}
