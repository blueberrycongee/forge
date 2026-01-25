# Forge

Language: English | [中文 README](README.zh.md)

Forge is a Rust framework for building stateful, event-driven agent runtimes.
It focuses on streaming execution, tool lifecycles, permission gating, and
audit-ready observability.

## Status

Active development. APIs may evolve.

## Why Forge

- **Streaming by default**: structured runtime events for text, tool progress,
  permissions, compaction, and run lifecycle.
- **Tool-first orchestration**: a loop abstraction designed for LLM ↔ tools
  workflows, without prescribing tool names.
- **Safety + control**: permission policies and resume flows to keep humans in
  the loop.
- **Traceable sessions**: snapshots, checkpoints, and replay for audits and
  debugging.

## Core Concepts

- **StateGraph / CompiledGraph**: build stateful workflows as graphs of async
  nodes.
- **LoopNode / LoopContext**: a streaming loop that can call tools, emit events,
  and resume after interruptions.
- **ToolRegistry / ToolDefinition / ToolOutput**: tool contracts, lifecycle
  events, and structured outputs with attachments.
- **PermissionPolicy / PermissionSession**: allow/ask/deny decisions with
  explicit resumes.
- **Event sinks**: JSONL/SSE sinks for CLI or UI streaming.
- **SessionState / Trace**: persistent run state, token usage, and replay.

## Installation

Forge is not published on crates.io yet. Use a git dependency:

```toml
[dependencies]
forge = { git = "https://github.com/blueberrycongee/forge", rev = "<commit>" }
```

Pin `rev` for reproducible builds. When tags or crates.io releases are
available, prefer those.

## Quickstart: StateGraph

```rust
use forge::runtime::constants::START;
use forge::runtime::prelude::{GraphError, StateGraph, END};
use forge::runtime::state::GraphState;

#[derive(Clone, Default)]
struct State {
    count: i32,
}

impl GraphState for State {}

async fn inc(mut state: State) -> Result<State, GraphError> {
    state.count += 1;
    Ok(state)
}

# async fn run() -> Result<(), GraphError> {
let mut graph = StateGraph::<State>::new();
graph.add_node("inc", inc);
graph.add_edge(START, "inc");
graph.add_edge("inc", END);

let compiled = graph.compile()?;
let result = compiled.invoke(State::default()).await?;
assert_eq!(result.count, 1);
# Ok(())
# }
```

## Quickstart: Tool + LoopNode

```rust
use forge::runtime::permission::{PermissionPolicy, PermissionSession};
use forge::runtime::r#loop::LoopNode;
use forge::runtime::state::GraphState;
use forge::runtime::tool::{ToolCall, ToolDefinition, ToolOutput, ToolRegistry};
use forge::runtime::prelude::{GraphError, StateGraph, END};
use forge::runtime::constants::START;
use serde_json::json;
use std::sync::Arc;

#[derive(Clone, Default)]
struct State {
    next: Option<String>,
}

impl GraphState for State {
    fn get_next(&self) -> Option<&str> { self.next.as_deref() }
    fn set_next(&mut self, next: Option<String>) { self.next = next; }
}

fn build_tools() -> Arc<ToolRegistry> {
    let mut registry = ToolRegistry::new();
    let definition = ToolDefinition::new("echo", "Echo input")
        .with_input_schema(json!({
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"]
        }));

    registry.register_with_definition(
        definition,
        Arc::new(|call, _ctx| {
            let text = call.input.get("text").cloned().unwrap_or_default();
            Box::pin(async move { Ok(ToolOutput::new(text)) })
        }),
    );

    Arc::new(registry)
}

# async fn run() -> Result<(), GraphError> {
let tools = build_tools();
let gate = Arc::new(PermissionSession::new(PermissionPolicy::default()));
let loop_node = LoopNode::with_tools_and_gate("agent_loop", tools, gate, |state, ctx| async move {
    let call = ToolCall::new("echo", "call-1", json!({ "text": "hello" }));
    let _ = ctx.run_tool(call).await?;
    Ok(state)
});

let mut graph = StateGraph::<State>::new();
graph.add_node_spec(loop_node.into_node());
graph.add_edge(START, "agent_loop");
graph.add_edge("agent_loop", END);

let compiled = graph.compile()?;
let _ = compiled.invoke(State::default()).await?;
# Ok(())
# }
```

## Event Streaming

Forge emits structured events through an `EventSink`. You can use built-in
sinks for CLI/UI streaming:

- `JsonLineEventSink` / `JsonLineEventRecordSink`
- `SseEventSink` / `SseEventRecordSink`

These live in `forge::runtime::output`.

## Repository Layout

```text
src/
  runtime/
    graph.rs          # StateGraph, edges, routing
    executor.rs       # CompiledGraph execution, checkpoints
    loop.rs           # LoopNode + LoopContext
    tool.rs           # Tool registry, lifecycle, attachments
    permission.rs     # Permission policy + session
    event.rs          # Event protocol
    session_state.rs  # Run metadata + reducer
    session.rs        # Snapshots and attachment store
    trace.rs          # Trace/replay
tests/
```

## Development

```bash
cargo test
cargo clippy
```

## Contributing

See `CONTRIBUTING.md`. Please follow `CODE_OF_CONDUCT.md`. Security issues go to
`SECURITY.md`.

## License

MIT. See `LICENSE`.
