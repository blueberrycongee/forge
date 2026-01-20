# PROGRESS

This log records atomic development progress for Forge. Each entry must be detailed and follow the template.

---

## Entry Template

- Date:
- Scope:
- Summary:
- Changes:
- Files touched:
- Known gaps / simplifications:
- Validation:
- Next steps:

---

## 2026-01-21 — Event Protocol Layer (Phase 1 kickoff)

- Date: 2026-01-21
- Scope: Runtime event protocol foundation (Phase 1)
- Summary: Added the core event protocol types and sink interface to enable streaming runtime events.
- Changes:
  - Introduced `Event` enum and supporting types (`TokenUsage`, `PermissionReply`).
  - Added `EventSink` trait for emitting events (UI/CLI/SSE compatibility).
  - Added `NoopEventSink` for silent execution/testing.
  - Exported new types via module prelude.
- Files touched:
  - `D:\Desktop\opencode\forge\event.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - No executor integration yet (no `stream_events`).
  - No `StreamNode` trait yet; only base event types exist.
  - No serialization strategy defined (future: serde for wire transport).
- Validation: Not run (type-only addition, no build/test harness present).
- Next steps:
  - Add `StreamNode` trait and extend `CompiledGraph` with `stream_events`.
  - Define an event serialization format for networked streaming.
  - Add a minimal demo node that emits `TextDelta` events.

## 2026-01-21 — Stream Events + Stream Nodes (Phase 1)

- Date: 2026-01-21
- Scope: Phase 1 runtime streaming (event protocol integration into executor)
- Summary: Added stream-capable nodes and a `stream_events` execution path; introduced Cargo crate scaffold and fixed doctests.
- Changes:
  - Added `Cargo.toml` + `src/lib.rs` to make Forge a Rust crate.
  - Added chrono/serde/uuid/futures deps to compile existing modules and tests.
  - Introduced `StreamNodeFn` support on `NodeSpec`, with optional stream function.
  - Added `StateGraph::add_stream_node` for stream-capable nodes.
  - Added `CompiledGraph::stream_events` to emit runtime events via `EventSink`.
  - Added unit test `stream_events_emits_from_stream_node` (TDD flow).
  - Replaced old `lumina_note_lib` doctest paths with `forge` for crate correctness.
- Files touched:
  - `D:\Desktop\opencode\forge\Cargo.toml`
  - `D:\Desktop\opencode\forge\src\lib.rs`
  - `D:\Desktop\opencode\forge\node.rs`
  - `D:\Desktop\opencode\forge\graph.rs`
  - `D:\Desktop\opencode\forge\executor.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
  - `D:\Desktop\opencode\forge\event.rs`
  - `D:\Desktop\opencode\forge\error.rs`
  - `D:\Desktop\opencode\forge\state.rs`
  - `D:\Desktop\opencode\forge\README.md`
- Known gaps / simplifications:
  - Stream functions accept `Arc<dyn EventSink>` instead of `&dyn EventSink` (simpler lifetimes).
  - Non-stream execution of stream nodes uses `NoopEventSink`.
  - No standardized wire serialization for events yet.
  - No per-node progress events beyond what stream nodes emit.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Define `Event` serialization format (serde + JSON schema).
  - Add `LoopNode` skeleton for OpenCode-style streaming loop.
  - Introduce PermissionGate + ToolLifecycle interfaces.
