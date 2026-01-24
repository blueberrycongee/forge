# Forge

Language: English | [中文 README](README.zh.md)

Forge is a Rust framework for building stateful, event-driven agent runtimes.
It focuses on streaming execution, tool lifecycles, permissions, and audit-ready
observability.

---

## English

### Overview

Forge provides a graph-based runtime with streaming events, tool lifecycle
tracking, permission gating, and session observability.

### Status

- Active development
- Phase 1-4 complete (see `PROGRESS.md`)

### Features

- State graph execution with async nodes
- Streamed runtime events (text, tool, permissions, compaction)
- LoopNode runtime for tool-driven agent loops
- Permission gating with allow/ask/deny and resume flow
- Tool registry with lifecycle events and structured output
- Compaction + prune policies with hooks
- Trace and replay for audit logging
- Session snapshot export/import and filesystem store

### Quickstart

```rust
use forge::prelude::*;

#[derive(Clone, Default)]
struct State {
    count: i32,
}

impl GraphState for State {}

async fn inc(mut state: State) -> GraphResult<State> {
    state.count += 1;
    Ok(state)
}

# async fn run() -> GraphResult<()> {
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

### Repository Layout

- `src/lib.rs` - module root and prelude
- `src/runtime/graph.rs` / `src/runtime/executor.rs` - graph construction + execution
- `src/runtime/event.rs` - runtime event protocol
- `src/runtime/loop.rs` - LoopNode runtime
- `src/runtime/tool.rs` - tool lifecycle and registry
- `src/runtime/permission.rs` - permissions and resume flow
- `src/runtime/compaction.rs` / `src/runtime/prune.rs` - session control policies
- `src/runtime/trace.rs` - trace/replay
- `src/runtime/session.rs` - snapshots and storage

### Links

- [Chinese README](README.zh.md)

### License

MIT. See `LICENSE`.
