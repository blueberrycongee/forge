//! Platform entry helpers for CLI/SSE output wiring.

use std::io::Write;
use std::sync::Arc;

use crate::runtime::error::GraphResult;
use crate::runtime::event::{EventRecordSink, EventSink, NoopEventSink};
use crate::runtime::executor::CompiledGraph;
use crate::runtime::output::{
    JsonLineEventRecordSink,
    JsonLineEventSink,
    SseEventRecordSink,
    SseEventSink,
};
use crate::runtime::state::GraphState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformOutputFormat {
    JsonLines,
    Sse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformStreamMode {
    Event,
    Record,
}

pub async fn stream_to_writer<S, W>(
    graph: &CompiledGraph<S>,
    state: S,
    format: PlatformOutputFormat,
    mode: PlatformStreamMode,
    writer: W,
) -> GraphResult<S>
where
    S: GraphState,
    W: Write + Send + 'static,
{
    match (format, mode) {
        (PlatformOutputFormat::JsonLines, PlatformStreamMode::Event) => {
            let sink: Arc<dyn EventSink> = Arc::new(JsonLineEventSink::new(writer));
            graph.stream_events(state, sink).await
        }
        (PlatformOutputFormat::JsonLines, PlatformStreamMode::Record) => {
            let record_sink: Arc<dyn EventRecordSink> =
                Arc::new(JsonLineEventRecordSink::new(writer));
            let config = graph
                .config()
                .clone()
                .with_event_record_sink(record_sink);
            let graph = graph.clone().with_config(config);
            let sink: Arc<dyn EventSink> = Arc::new(NoopEventSink);
            graph.stream_events(state, sink).await
        }
        (PlatformOutputFormat::Sse, PlatformStreamMode::Event) => {
            let sink: Arc<dyn EventSink> = Arc::new(SseEventSink::new(writer));
            graph.stream_events(state, sink).await
        }
        (PlatformOutputFormat::Sse, PlatformStreamMode::Record) => {
            let record_sink: Arc<dyn EventRecordSink> =
                Arc::new(SseEventRecordSink::new(writer));
            let config = graph
                .config()
                .clone()
                .with_event_record_sink(record_sink);
            let graph = graph.clone().with_config(config);
            let sink: Arc<dyn EventSink> = Arc::new(NoopEventSink);
            graph.stream_events(state, sink).await
        }
    }
}

pub async fn stream_cli_jsonl_events<S, W>(
    graph: &CompiledGraph<S>,
    state: S,
    writer: W,
) -> GraphResult<S>
where
    S: GraphState,
    W: Write + Send + 'static,
{
    stream_to_writer(
        graph,
        state,
        PlatformOutputFormat::JsonLines,
        PlatformStreamMode::Event,
        writer,
    )
    .await
}

pub async fn stream_cli_jsonl_records<S, W>(
    graph: &CompiledGraph<S>,
    state: S,
    writer: W,
) -> GraphResult<S>
where
    S: GraphState,
    W: Write + Send + 'static,
{
    stream_to_writer(
        graph,
        state,
        PlatformOutputFormat::JsonLines,
        PlatformStreamMode::Record,
        writer,
    )
    .await
}

pub async fn stream_sse_events<S, W>(
    graph: &CompiledGraph<S>,
    state: S,
    writer: W,
) -> GraphResult<S>
where
    S: GraphState,
    W: Write + Send + 'static,
{
    stream_to_writer(
        graph,
        state,
        PlatformOutputFormat::Sse,
        PlatformStreamMode::Event,
        writer,
    )
    .await
}

pub async fn stream_sse_records<S, W>(
    graph: &CompiledGraph<S>,
    state: S,
    writer: W,
) -> GraphResult<S>
where
    S: GraphState,
    W: Write + Send + 'static,
{
    stream_to_writer(
        graph,
        state,
        PlatformOutputFormat::Sse,
        PlatformStreamMode::Record,
        writer,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::constants::{END, START};
    use crate::runtime::event::Event;
    use crate::runtime::graph::StateGraph;
    use futures::executor::block_on;
    use std::io;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct StreamState;

    impl GraphState for StreamState {}

    #[derive(Clone, Default)]
    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl SharedBuffer {
        fn writer(&self) -> SharedWriter {
            SharedWriter(self.0.clone())
        }

        fn as_string(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).expect("utf8")
        }
    }

    struct SharedWriter(Arc<Mutex<Vec<u8>>>);

    impl io::Write for SharedWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn build_graph() -> CompiledGraph<StreamState> {
        let mut graph = StateGraph::<StreamState>::new();
        graph.add_stream_node("node", |state, sink| async move {
            sink.emit(Event::TextDelta {
                session_id: "s1".to_string(),
                message_id: "m1".to_string(),
                delta: "hello".to_string(),
            })?;
            Ok(state)
        });
        graph.add_edge(START, "node");
        graph.add_edge("node", END);
        graph.compile().expect("compile")
    }

    #[test]
    fn platform_streams_cli_jsonl_events() {
        let graph = build_graph();
        let buffer = SharedBuffer::default();
        let writer = buffer.writer();

        let _ = block_on(stream_cli_jsonl_events(&graph, StreamState, writer)).expect("run");

        let output = buffer.as_string();
        assert!(output.contains("\"TextDelta\""));
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn platform_streams_cli_jsonl_records() {
        let graph = build_graph();
        let buffer = SharedBuffer::default();
        let writer = buffer.writer();

        let _ = block_on(stream_cli_jsonl_records(&graph, StreamState, writer)).expect("run");

        let output = buffer.as_string();
        assert!(output.contains("\"meta\""));
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn platform_streams_sse_events() {
        let graph = build_graph();
        let buffer = SharedBuffer::default();
        let writer = buffer.writer();

        let _ = block_on(stream_sse_events(&graph, StreamState, writer)).expect("run");

        let output = buffer.as_string();
        assert!(output.starts_with("data: "));
        assert!(output.ends_with("\n\n"));
    }

    #[test]
    fn platform_streams_sse_records() {
        let graph = build_graph();
        let buffer = SharedBuffer::default();
        let writer = buffer.writer();

        let _ = block_on(stream_sse_records(&graph, StreamState, writer)).expect("run");

        let output = buffer.as_string();
        assert!(output.contains("\"meta\""));
        assert!(output.starts_with("data: "));
        assert!(output.ends_with("\n\n"));
    }
}
