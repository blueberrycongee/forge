# Forge

Language: English | [中文 README](README.zh.md)

[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-active%20development-yellow.svg)](#project-status)

Forge is a Rust-first orchestration runtime for long-running, stateful agent workflows.
It focuses on durable graph execution, event-streaming, tool lifecycles, permission gating,
and checkpoint-based interrupt/resume.

## Overview

Forge provides low-level runtime primitives. It does not prescribe prompting style,
agent architecture, or tool naming conventions.

Use Forge when you need:

- explicit state transitions across graph nodes
- structured runtime events for UI/CLI streaming and audit logs
- controlled tool execution with permission and attachment policies
- resumable execution for human-in-the-loop and failure recovery

## Core Capabilities

- Durable execution: compile state graphs into resumable plans with checkpoints.
- Streaming event protocol: text, tool, permission, compaction, and run lifecycle events.
- Tool-first runtime loop: orchestrate LLM-to-tool interaction with lifecycle metadata.
- Permission system: allow/ask/deny decisions with persisted session state.
- Session model: snapshots, replay traces, and run metadata for debugging and audits.
- Provider adapters: built-in OpenAI `ChatModel` adapter via `runtime::provider::openai`.

## Installation

Forge is not published on crates.io yet. Use a Git dependency and pin a commit:

```toml
[dependencies]
forge = { git = "https://github.com/blueberrycongee/forge", rev = "<commit>" }
```

## Quick Start

### 1) Build a StateGraph

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

### 2) Run a tool loop with permissions

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

### 3) Use the OpenAI provider adapter

```rust
use forge::runtime::prelude::{ChatModel, ChatRequest, Message, MessageRole, OpenAiChatModel, OpenAiChatModelConfig, Part};
use futures::executor::block_on;

let model = OpenAiChatModel::new(OpenAiChatModelConfig::new("gpt-4o-mini"))?;
let mut msg = Message::new(MessageRole::User);
msg.parts.push(Part::TextFinal { text: "Say hi in one short sentence.".to_string() });
let req = ChatRequest::new("session-1", "message-1", vec![msg]);

let resp = block_on(model.generate(req))?;
println!("model={:?} text={:?}", resp.model, resp.text());
# Ok::<(), forge::runtime::error::GraphError>(())
```

## Architecture

| Module | Responsibility |
| --- | --- |
| `runtime::graph` | Build and compile state graphs (`StateGraph -> CompiledGraph`) |
| `runtime::executor` | Run lifecycle, checkpoints, resume commands, and streaming |
| `runtime::loop` | Tool loop abstraction (`LoopNode`, `LoopContext`) |
| `runtime::tool` | Tool contracts, registry, metadata, attachments, statuses |
| `runtime::permission` | Permission policies and session decisions |
| `runtime::event` | Runtime event protocol and sequencing |
| `runtime::session_state` | Event-to-state reduction and run metadata |
| `runtime::session` | Session snapshots and persistence helpers |
| `runtime::provider` | External model adapters (currently OpenAI chat) |

## Event Protocol

Forge emits structured events through `EventSink` / `EventRecordSink`.
Built-in stream outputs:

- `JsonLineEventSink` and `JsonLineEventRecordSink`
- `SseEventSink` and `SseEventRecordSink`

Event families include:

- Run lifecycle: `RunStarted`, `RunPaused`, `RunResumed`, `RunCompleted`, `RunFailed`
- Text and message updates: `TextDelta`, `TextFinal`, `Attachment`, `Error`
- Tool lifecycle: `ToolStart`, `ToolUpdate`, `ToolResult`, `ToolError`, `ToolStatus`
- Permission flow: `PermissionAsked`, `PermissionReplied`
- Session controls: compaction and phase transition events

## Examples

Runnable examples are in `/examples`:

- `cargo run --example core_workflow`
- `cargo run --example multi_agent_graph`
- `cargo run --example tool_context`

## Project Status

Forge is in active pre-1.0 development.

- Public API and compatibility governance are documented and enforced in PR review.
- Pre-1.0 changes may still be breaking, but must include migration notes.
- Release cadence is milestone-driven instead of time-driven.
- For production usage, pin a specific commit and run your own compatibility tests.

## Development

```bash
cargo test
cargo clippy
```

## Documentation

- Runtime evaluation notes: [EVALUATION.md](EVALUATION.md)
- Progress log: [PROGRESS.md](PROGRESS.md)
- Contribution guide: [CONTRIBUTING.md](CONTRIBUTING.md)
- Security policy: [SECURITY.md](SECURITY.md)
- API compatibility policy: [docs/api-compatibility-policy.md](docs/api-compatibility-policy.md)
- Deprecation policy: [docs/deprecation-policy.md](docs/deprecation-policy.md)
- Upgrade guide: [docs/upgrading.md](docs/upgrading.md)
- Changelog: [CHANGELOG.md](CHANGELOG.md)
- 1.0 contracts and conformance matrix: [docs/forge-1.0-contracts-and-conformance.md](docs/forge-1.0-contracts-and-conformance.md)

## Contributing

Contributions are welcome. Please read `CONTRIBUTING.md` and follow
`CODE_OF_CONDUCT.md`.

## License

MIT. See [LICENSE](LICENSE).
