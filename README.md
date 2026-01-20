# Forge

Forge is a Rust framework for building stateful, event-driven agent runtimes.
It focuses on streaming execution, tool lifecycles, permissions, and audit-ready
observability.

## Status

- Active development
- Phase 1-4 complete (see PROGRESS)

## Features

- State graph execution with async nodes
- Streamed runtime events (text, tool, permissions, compaction)
- LoopNode runtime for OpenCode-style agent loops
- Permission gating with allow/ask/deny and resume flow
- Tool registry with lifecycle events and structured output
- Compaction + prune policies with hooks
- Trace and replay for audit logging
- Session snapshot export/import and filesystem store

## Quickstart

```rust
use forge::langgraph::prelude::*;

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

## Repo Layout

- `mod.rs` - module root and prelude
- `graph.rs` / `executor.rs` - graph construction and execution
- `event.rs` - runtime event protocol
- `loop.rs` - LoopNode runtime
- `tool.rs` - tool lifecycle and registry
- `permission.rs` - permissions and resume flow
- `compaction.rs` / `prune.rs` - session control policies
- `trace.rs` - trace/replay
- `session.rs` - snapshots and storage

## Docs

- `docs/README-legacy.md` - original long-form documentation
- `PROGRESS.md` - development log with timestamps
- `OPENCODE_RUNTIME_PLAN.md` - roadmap and mapping notes

## Development

Run tests:

```bash
C:\Users\10758\.cargo\bin\cargo.exe test
```

## License

MIT. See `LICENSE`.
