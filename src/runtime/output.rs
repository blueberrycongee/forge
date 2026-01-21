//! Output adapters for runtime event streams.

use std::io::Write;
use std::sync::Mutex;

use crate::runtime::event::{Event, EventRecord, EventRecordSink, EventSink};

/// JSON Lines output for Event stream (CLI-friendly).
pub struct JsonLineEventSink<W: Write + Send> {
    writer: Mutex<W>,
}

impl<W: Write + Send> JsonLineEventSink<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer: Mutex::new(writer),
        }
    }

    pub fn into_inner(self) -> W {
        self.writer.into_inner().expect("poisoned writer")
    }
}

impl<W: Write + Send> EventSink for JsonLineEventSink<W> {
    fn emit(&self, event: Event) {
        if let Ok(json) = serde_json::to_string(&event) {
            let mut writer = self.writer.lock().expect("poisoned writer");
            let _ = writeln!(writer, "{json}");
        }
    }
}

/// JSON Lines output for EventRecord stream (with metadata).
pub struct JsonLineEventRecordSink<W: Write + Send> {
    writer: Mutex<W>,
}

impl<W: Write + Send> JsonLineEventRecordSink<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer: Mutex::new(writer),
        }
    }

    pub fn into_inner(self) -> W {
        self.writer.into_inner().expect("poisoned writer")
    }
}

impl<W: Write + Send> std::fmt::Debug for JsonLineEventRecordSink<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonLineEventRecordSink").finish()
    }
}

impl<W: Write + Send> EventRecordSink for JsonLineEventRecordSink<W> {
    fn emit_record(&self, record: EventRecord) {
        if let Ok(json) = serde_json::to_string(&record) {
            let mut writer = self.writer.lock().expect("poisoned writer");
            let _ = writeln!(writer, "{json}");
        }
    }
}

/// SSE output for Event stream (server-friendly).
pub struct SseEventSink<W: Write + Send> {
    writer: Mutex<W>,
}

impl<W: Write + Send> SseEventSink<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer: Mutex::new(writer),
        }
    }

    pub fn into_inner(self) -> W {
        self.writer.into_inner().expect("poisoned writer")
    }
}

impl<W: Write + Send> EventSink for SseEventSink<W> {
    fn emit(&self, event: Event) {
        if let Ok(json) = serde_json::to_string(&event) {
            let mut writer = self.writer.lock().expect("poisoned writer");
            let _ = write!(writer, "data: {json}\n\n");
        }
    }
}

/// SSE output for EventRecord stream (with metadata).
pub struct SseEventRecordSink<W: Write + Send> {
    writer: Mutex<W>,
}

impl<W: Write + Send> SseEventRecordSink<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer: Mutex::new(writer),
        }
    }

    pub fn into_inner(self) -> W {
        self.writer.into_inner().expect("poisoned writer")
    }
}

impl<W: Write + Send> std::fmt::Debug for SseEventRecordSink<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SseEventRecordSink").finish()
    }
}

impl<W: Write + Send> EventRecordSink for SseEventRecordSink<W> {
    fn emit_record(&self, record: EventRecord) {
        if let Ok(json) = serde_json::to_string(&record) {
            let mut writer = self.writer.lock().expect("poisoned writer");
            let _ = write!(writer, "data: {json}\n\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        JsonLineEventRecordSink,
        JsonLineEventSink,
        SseEventRecordSink,
        SseEventSink,
    };
    use crate::runtime::event::{Event, EventMeta, EventRecord, EventRecordSink, EventSink};

    #[test]
    fn json_line_event_sink_writes_lines() {
        let sink = JsonLineEventSink::new(Vec::new());
        sink.emit(Event::TextDelta {
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            delta: "hi".to_string(),
        });

        let output = String::from_utf8(sink.into_inner()).expect("utf8");
        assert!(output.contains("\"TextDelta\""));
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn json_line_event_record_sink_writes_meta() {
        let sink = JsonLineEventRecordSink::new(Vec::new());
        let record = EventRecord::with_meta(
            Event::StepStart {
                session_id: "s1".to_string(),
            },
            EventMeta {
                event_id: "e1".to_string(),
                timestamp_ms: 1,
                seq: 1,
            },
        );
        sink.emit_record(record);

        let output = String::from_utf8(sink.into_inner()).expect("utf8");
        assert!(output.contains("\"meta\""));
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn sse_event_sink_writes_data_frames() {
        let sink = SseEventSink::new(Vec::new());
        sink.emit(Event::StepStart {
            session_id: "s1".to_string(),
        });

        let output = String::from_utf8(sink.into_inner()).expect("utf8");
        assert!(output.starts_with("data: "));
        assert!(output.ends_with("\n\n"));
    }

    #[test]
    fn sse_event_record_sink_writes_data_frames() {
        let sink = SseEventRecordSink::new(Vec::new());
        let record = EventRecord::with_meta(
            Event::StepStart {
                session_id: "s1".to_string(),
            },
            EventMeta {
                event_id: "e1".to_string(),
                timestamp_ms: 1,
                seq: 1,
            },
        );
        sink.emit_record(record);

        let output = String::from_utf8(sink.into_inner()).expect("utf8");
        assert!(output.starts_with("data: "));
        assert!(output.ends_with("\n\n"));
    }
}
