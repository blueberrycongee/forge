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

---

## Note — Timestamp Policy (effective 2026-01-21 02:12:01)

- From this point forward, all PROGRESS entries must use the local machine time with second-level precision.
- Prior entries remain unchanged by request.

## 2026-01-21 02:13:51 — LoopNode Skeleton (Phase 2)

- Date: 2026-01-21 02:13:51
- Scope: Phase 2 LoopNode skeleton (OpenCode-style loop abstraction)
- Summary: Added LoopNode with event-emitting handler and tests; integrated into module exports.
- Changes:
  - Added LoopNode abstraction with 
un and into_node methods.
  - Added loop_node_emits_events test (TDD) to verify event emission and state update.
  - Exported LoopNode in module prelude (module name escaped as 
#loop).
- Files touched:
  - D:\Desktop\opencode\forge\loop.rs
  - D:\Desktop\opencode\forge\mod.rs
- Known gaps / simplifications:
  - LoopNode is a thin wrapper around a stream-capable handler (no actual LLM/tool loop yet).
  - No structured loop state (session/message IDs, tool lifecycle) at this stage.
- Validation:
  - C:\Users\10758\.cargo\bin\cargo.exe test
- Next steps:
  - Define minimal LoopState structure for session/message metadata.
  - Introduce PermissionGate and ToolLifecycle traits for LoopNode integration.

## 2026-01-21 02:16:24 LoopState Struct (Phase 2)

- Date: 2026-01-21 02:16:24
- Scope: Phase 2 LoopState foundation (session/message metadata)
- Summary: Added a minimal LoopState with session/message identifiers, step counter, and routing/complete flags.
- Changes:
  - Introduced `LoopState` with `session_id`, `message_id`, `step`, `next`, and `complete`.
  - Implemented `GraphState` for LoopState to support routing and completion semantics.
  - Added `advance_step` helper for loop iteration tracking.
  - Added `loop_state_tracks_session_and_routing` test (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\state.rs`
- Known gaps / simplifications:
  - LoopState does not yet model messages, tool calls, or structured parts.
  - No persistence/serialization strategy for LoopState.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Introduce PermissionGate trait (allow/ask/deny) with minimal rule matching.
  - Define ToolLifecycle types (pending/running/completed/error) and events.

## 2026-01-21 02:20:19 PermissionGate (Phase 2)

- Date: 2026-01-21 02:20:19
- Scope: Phase 2 permission gating primitives
- Summary: Added minimal permission policy with ordered rule matching and wildcard support.
- Changes:
  - Added `PermissionDecision`, `PermissionRule`, and `PermissionPolicy` types.
  - Implemented `PermissionGate` trait with default policy adapter.
  - Implemented minimal wildcard matching (`*` and prefix `prefix*`).
  - Added unit tests covering first-match order, wildcard prefix, and default allow (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\permission.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - Matching only supports exact or prefix wildcard; no full glob/regex.
  - Default decision is `Allow` when no rule matches (may be tightened later).
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Define ToolLifecycle types (pending/running/completed/error) and link to `Event`.
  - Add a minimal tool execution facade to LoopNode to emit lifecycle events.

## 2026-01-21 02:24:39 ToolLifecycle Types (Phase 2)

- Date: 2026-01-21 02:24:39
- Scope: Phase 2 tool lifecycle primitives
- Summary: Added minimal ToolLifecycle state types and linked lifecycle to Event stream.
- Changes:
  - Added `ToolState` enum with pending/running/completed/error.
  - Added `Event::ToolStatus` variant to emit lifecycle transitions.
  - Exported `ToolState` via module prelude.
  - Added unit tests for ToolState and ToolStatus event (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\tool.rs`
  - `D:\Desktop\opencode\forge\event.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - No structured tool call input/output model yet (only state).
  - ToolStatus does not carry timestamps or payloads.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Add a minimal tool execution facade to LoopNode that emits ToolStatus.
  - Define ToolCall input/output structures for richer lifecycle events.

## 2026-01-21 02:29:54 ToolRunner Facade (Phase 2)

- Date: 2026-01-21 02:29:54
- Scope: Phase 2 tool execution facade
- Summary: Added ToolRunner with lifecycle + start/result events for tool execution.
- Changes:
  - Added `ToolCall` struct to capture tool name, call ID, and input payload.
  - Implemented `ToolRunner::run_with_events` to emit ToolStatus/ToolStart/ToolResult/ToolError.
  - Exported `ToolCall`/`ToolRunner` via prelude.
  - Added unit test covering event order and output (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\tool.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - Tool output is a `String`; no structured output type yet.
  - No timestamps or token usage tied to tool lifecycle.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Introduce structured ToolOutput (JSON) + optional metadata.
  - Add a simple ToolRegistry to dispatch by name and wire into LoopNode.

## 2026-01-21 02:38:15 ToolRegistry Dispatch (Phase 2)

- Date: 2026-01-21 02:38:15
- Scope: Phase 2 tool dispatch registry
- Summary: Added ToolRegistry for name-based dispatch with lifecycle event emission.
- Changes:
  - Added `ToolHandler` alias and `ToolRegistry` with register/has/run_with_events.
  - Registry delegates to ToolRunner for lifecycle events and error emission.
  - Added tests for registry dispatch and missing-tool error (TDD).
  - Exported `ToolRegistry` via prelude.
- Files touched:
  - `D:\Desktop\opencode\forge\tool.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - Registry uses `String` output (no structured ToolOutput yet).
  - No tool metadata/permissions integration yet.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Introduce structured ToolOutput (JSON) + optional metadata.
  - Wire ToolRegistry into LoopNode to drive tool calls from the loop.

## 2026-01-21 02:42:03 LoopNode Tool Integration (Phase 2)

- Date: 2026-01-21 02:42:03
- Scope: Phase 2 LoopNode + ToolRegistry integration
- Summary: Wired ToolRegistry into LoopNode via LoopContext, enabling tool calls inside loop handlers.
- Changes:
  - Added `LoopContext` carrying `EventSink` + `ToolRegistry`.
  - Updated LoopNode handler signature to accept LoopContext.
  - Added `LoopNode::with_tools` constructor and default registry for `new`.
  - Added `LoopContext::run_tool` to emit ToolStatus/ToolStart/ToolResult through registry.
  - Added integration test `loop_node_runs_tools_via_registry` (TDD).
  - Exported LoopContext in prelude.
- Files touched:
  - `D:\Desktop\opencode\forge\loop.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - LoopContext does not expose permission gating yet.
  - Tool outputs are still `String` (no structured ToolOutput).
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Introduce structured ToolOutput (JSON) + optional metadata.
  - Add PermissionGate integration in LoopContext (allow/ask/deny per tool).

## 2026-01-21 02:44:53 LoopContext Permission Gate (Phase 2)

- Date: 2026-01-21 02:44:53
- Scope: Phase 2 permission gating in loop context
- Summary: Integrated PermissionGate into LoopContext to block or ask before tool execution.
- Changes:
  - Added PermissionGate to LoopContext and LoopNode construction.
  - Added LoopNode::with_tools_and_gate for explicit gate wiring.
  - Implemented permission checks in LoopContext::run_tool (allow/ask/deny).
  - Emitted PermissionAsked event on Ask decisions.
  - Added unit test `loop_context_asks_permission_for_tool` (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\loop.rs`
- Known gaps / simplifications:
  - Ask/deny currently return ExecutionError; no resume flow yet.
  - PermissionAsked event uses the requested permission as fallback patterns.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Introduce structured ToolOutput (JSON) + optional metadata.
  - Add permission reply/resume flow (PermissionReplied + Interrupt handling).

## 2026-01-21 02:48:02 Structured ToolOutput (Phase 2)

- Date: 2026-01-21 02:48:02
- Scope: Phase 2 structured tool output model
- Summary: Added ToolOutput with JSON payload + optional metadata and propagated through tool execution flow.
- Changes:
  - Added `ToolOutput` (content + metadata) with helpers `new`/`with_metadata`/`text`.
  - Updated ToolRunner/ToolRegistry to return ToolOutput instead of String.
  - Updated ToolResult event payload to carry ToolOutput.
  - Updated LoopContext tool integration tests to work with structured output (TDD).
  - Exported ToolOutput in prelude.
- Files touched:
  - `D:\Desktop\opencode\forge\tool.rs`
  - `D:\Desktop\opencode\forge\event.rs`
  - `D:\Desktop\opencode\forge\loop.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - ToolOutput metadata is unstructured JSON (no schema yet).
  - ToolResult does not carry timestamps or token usage.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Add permission reply/resume flow (PermissionReplied + Interrupt handling).
  - Introduce ToolOutput schema/metadata helpers (typed fields).

## 2026-01-21 02:51:24 Permission Reply Flow (Phase 2)

- Date: 2026-01-21 02:51:24
- Scope: Phase 2 permission reply handling
- Summary: Added PermissionSession with runtime overrides and wired replies into LoopContext.
- Changes:
  - Added `PermissionSession` with override sets for once/always/reject replies.
  - Implemented `PermissionSession::apply_reply` and override-aware `decide`.
  - Added `LoopContext::reply_permission` to emit PermissionReplied events.
  - Updated LoopNode/LoopContext to use PermissionSession.
  - Added tests for once/always/reject overrides and for reply-based tool execution (TDD).
  - Exported PermissionSession in prelude.
- Files touched:
  - `D:\Desktop\opencode\forge\permission.rs`
  - `D:\Desktop\opencode\forge\loop.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - Ask/deny still return ExecutionError; no Interrupt resume command yet.
  - PermissionAsked includes only direct permission string as pattern.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Add interrupt-based resume flow (PermissionReply via ResumeCommand).
  - Add ToolOutput schema/metadata helpers (typed fields).

## 2026-01-21 03:07:00 Permission Interrupt Resume (Phase 2)

- Date: 2026-01-21 03:07:00
- Scope: Phase 2 permission interrupt/resume flow
- Summary: Permission ask now raises Interrupt with request payload; ResumeCommand can approve via PermissionSession.
- Changes:
  - Added `PermissionRequest` payload (serde) for interrupt value.
  - Added `PermissionSession::apply_resume` to parse ResumeCommand values.
  - Added `LoopContext::resume_permission` to emit PermissionReplied on resume.
  - Updated LoopContext tool execution to return GraphError::Interrupted on ask.
  - Added tests for resume parsing, interrupt payload, and resume-based tool execution (TDD).
  - Derived PartialEq for PermissionReply to simplify assertions.
- Files touched:
  - `D:\Desktop\opencode\forge\permission.rs`
  - `D:\Desktop\opencode\forge\loop.rs`
  - `D:\Desktop\opencode\forge\event.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - Interrupt resume is local-only (no session persistence yet).
  - PermissionAsked patterns still mirror the requested permission string.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Add ToolOutput schema/metadata helpers (typed fields).
  - Add persistence hooks for permission sessions (store/resume by session id).

## 2026-01-21 03:08:45 ToolOutput Typed Metadata (Phase 2)

- Date: 2026-01-21 03:08:45
- Scope: Phase 2 ToolOutput schema/metadata helpers
- Summary: Added typed ToolMetadata and helper methods for structured tool output annotations.
- Changes:
  - Added `ToolMetadata` with mime_type/schema/source/attributes (serde-ready).
  - Updated ToolOutput metadata to use ToolMetadata instead of raw JSON.
  - Added helper methods to set mime_type/schema/source/custom attributes.
  - Added tests for metadata helpers and ToolOutput::with_metadata (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\tool.rs`
- Known gaps / simplifications:
  - No standardized schemas or enums for common tool outputs yet.
  - ToolResult does not carry timestamps or token usage.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Add persistence hooks for permission sessions (store/resume by session id).
  - Introduce a ToolOutput schema registry for common tools (grep, read, ls).

## 2026-01-21 03:21:47 Permission Session Snapshot (Phase 2)

- Date: 2026-01-21 03:21:47
- Scope: Phase 2 permission session persistence hooks
- Summary: Added snapshot/restore for permission sessions to enable persistence and resume.
- Changes:
  - Added `PermissionSnapshot` (serde) capturing once/always/reject overrides.
  - Added `PermissionSession::snapshot` and `PermissionSession::restore`.
  - Added tests for snapshot roundtrip and restore behavior (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\permission.rs`
- Known gaps / simplifications:
  - Snapshot does not include base policy rules (only runtime overrides).
  - No IO/storage adapter yet; persistence integration is left to caller.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Introduce ToolOutput schema registry for common tools (grep, read, ls).
  - Add persistence adapter trait (load/save by session id).

## 2026-01-21 03:23:44 ToolOutput Schema Registry (Phase 2)

- Date: 2026-01-21 03:23:44
- Scope: Phase 2 ToolOutput schema registry
- Summary: Added ToolSchemaRegistry with common tool schemas and output annotation helpers.
- Changes:
  - Added `ToolSchemaRegistry` with register/get and default common schemas.
  - Added `annotate_output` to fill missing ToolOutput metadata.
  - Added tests for default entries and annotation behavior (TDD).
  - Exported ToolSchemaRegistry in prelude.
- Files touched:
  - `D:\Desktop\opencode\forge\tool.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - Schemas are string identifiers only; no formal validation layer yet.
  - Common schemas are placeholders (read/grep/ls v1).
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Add persistence adapter trait (load/save by session id).
  - Introduce tool-specific output structs (ReadOutput, GrepOutput, LsOutput).

## 2026-01-21 03:25:22 Permission Store Adapter (Phase 2)

- Date: 2026-01-21 03:25:22
- Scope: Phase 2 permission persistence adapter
- Summary: Added PermissionStore trait and in-memory implementation for session snapshots.
- Changes:
  - Added `PermissionStore` trait with load/save by session id.
  - Added `InMemoryPermissionStore` for tests/local use.
  - Added tests for store roundtrip (TDD).
  - Exported PermissionStore/InMemoryPermissionStore in prelude.
- Files touched:
  - `D:\Desktop\opencode\forge\permission.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - No filesystem or database adapter yet.
  - Store is not wired into LoopContext by default.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Introduce tool-specific output structs (ReadOutput, GrepOutput, LsOutput).
  - Wire PermissionStore into a higher-level session manager (future).

## 2026-01-21 03:26:22 Phase 2 Complete (MVP-1)

- Date: 2026-01-21 03:26:22
- Scope: Phase 2 closeout
- Summary: Phase 2 core goals completed (LoopNode + permissions + tool lifecycle + structured output + persistence hooks).
- Changes:
  - Marked Phase 2 as complete.
- Files touched:
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Optional: tool-specific output structs (ReadOutput/GrepOutput/LsOutput).
  - Optional: wire PermissionStore into a higher-level session manager.
  - Optional: stricter schema validation or official schema list.
- Validation:
  - N/A (status update only).
- Next steps:
  - Begin Phase 3 (compaction/prune + trace/replay).

## 2026-01-21 03:28:07 Phase 3 Round 1 (Compaction Model)

- Date: 2026-01-21 03:28:07
- Scope: Phase 3 round 1 - compaction policy/result + event
- Summary: Added compaction policy/result types and session compaction event.
- Changes:
  - Added `CompactionPolicy` and `CompactionResult` with basic helpers.
  - Added `Event::SessionCompacted` for compaction notifications.
  - Exported compaction types in prelude.
  - Added unit tests for policy threshold and result structure (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\compaction.rs`
  - `D:\Desktop\opencode\forge\event.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - No executor integration yet (compaction not triggered).
  - CompactionResult does not include summary prompt or token usage.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Add compaction hook trait and wiring points in executor/loop.

## 2026-01-21 03:29:15 Phase 3 Round 2 (Compaction Hooks)

- Date: 2026-01-21 03:29:15
- Scope: Phase 3 round 2 - compaction hooks
- Summary: Added compaction hook trait and no-op implementation.
- Changes:
  - Added `CompactionHook` trait with before/after callbacks.
  - Added `NoopCompactionHook` default implementation.
  - Added unit test for default hook behavior (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\compaction.rs`
- Known gaps / simplifications:
  - Hooks are not wired into executor/loop yet.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Wire compaction hooks into executor/loop.

## 2026-01-21 03:31:40 Phase 3 Round 3 (Compaction Hook Wiring)

- Date: 2026-01-21 03:31:40
- Scope: Phase 3 round 3 - hook wiring in executor stream_events
- Summary: Wired compaction hook into stream_events and emit SessionCompacted events.
- Changes:
  - Added compaction hook to ExecutionConfig with setter.
  - Invoked hook in stream_events and emitted SessionCompacted event.
  - Added resolve_session_id helper (state.get("session_id") fallback).
  - Added test asserting compaction event emission (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\executor.rs`
- Known gaps / simplifications:
  - Hook uses empty message list (no message extraction yet).
  - Only stream_events path emits compaction events.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Add prune policy + pruning implementation.

## 2026-01-21 03:33:22 Phase 3 Round 4 (Prune Policy)

- Date: 2026-01-21 03:33:22
- Scope: Phase 3 round 4 - pruning old tool events
- Summary: Added prune policy and helper to remove old tool events.
- Changes:
  - Added `PrunePolicy`, `PruneResult`, and `prune_tool_events` helper.
  - Implemented tool-event detection and retention of most recent N tool events.
  - Added unit tests for pruning behavior and disabled policy (TDD).
  - Exported prune types in prelude.
- Files touched:
  - `D:\Desktop\opencode\forge\prune.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - Pruning operates on event lists, not on message/part history yet.
  - No executor integration to apply pruning during runs.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Integrate pruning into executor/loop (apply to stored event history).

## 2026-01-21 03:35:41 Phase 3 Round 5 (Prune Integration)

- Date: 2026-01-21 03:35:41
- Scope: Phase 3 round 5 - pruning integration in stream_events
- Summary: Integrated prune policy into stream_events with recording sink + history buffer.
- Changes:
  - Added prune policy + event history buffer to ExecutionConfig.
  - Added RecordingSink to capture emitted events into history.
  - Applied prune_tool_events after each node when enabled.
  - Added test verifying old tool events are pruned (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\executor.rs`
- Known gaps / simplifications:
  - Pruning only applies to stream_events history; invoke/invoke_with_metrics unchanged.
  - History buffer is optional and not persisted by default.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Add trace/replay data structures (TraceEvent/TraceSpan).

## 2026-01-21 03:37:03 Phase 3 Round 6 (Trace Structures)

- Date: 2026-01-21 03:37:03
- Scope: Phase 3 round 6 - trace data structures
- Summary: Added trace event/span data structures with serde support.
- Changes:
  - Added `TraceEvent`, `TraceSpan`, and `ExecutionTrace`.
  - Added trace record helpers and roundtrip tests (TDD).
  - Exported trace types in prelude.
- Files touched:
  - `D:\Desktop\opencode\forge\trace.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - Trace is not yet wired into executor or replay engine.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Implement TraceReplay to reconstruct event sequence.

## 2026-01-21 03:38:14 Phase 3 Round 7 (Trace Replay)

- Date: 2026-01-21 03:38:14
- Scope: Phase 3 round 7 - trace replay
- Summary: Added TraceReplay helper to replay recorded trace events.
- Changes:
  - Added `TraceReplay` with `replay` method.
  - Added unit test for replay order (TDD).
  - Exported TraceReplay in prelude.
- Files touched:
  - `D:\Desktop\opencode\forge\trace.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - Replay only returns events; no reconstruction into state yet.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Add SessionSnapshot (messages + trace + compaction summaries).

## 2026-01-21 03:39:50 Phase 3 Round 8 (Session Snapshot)

- Date: 2026-01-21 03:39:50
- Scope: Phase 3 round 8 - session snapshot export/import model
- Summary: Added SessionSnapshot with messages, trace, and compaction summaries.
- Changes:
  - Added `SessionMessage` and `SessionSnapshot` (serde).
  - Added roundtrip serialization test (TDD).
  - Exported session snapshot types in prelude.
  - Added serde derives to CompactionResult for snapshot support.
- Files touched:
  - `D:\Desktop\opencode\forge\session.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
  - `D:\Desktop\opencode\forge\compaction.rs`
- Known gaps / simplifications:
  - Messages are minimal (role + content only).
  - Snapshot does not capture tool events or permissions yet.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Integrate trace + snapshot emission in executor.

## 2026-01-21 03:41:35 Phase 3 Round 9 (Trace Wiring)

- Date: 2026-01-21 03:41:35
- Scope: Phase 3 round 9 - trace wiring in executor
- Summary: Wired trace collection into stream_events and added snapshot builder hook.
- Changes:
  - Added trace collector to ExecutionConfig and recorded NodeStart/NodeFinish events.
  - Recorded Compacted events when compaction hook fires.
  - Added build_snapshot helper on CompiledGraph.
  - Added test verifying trace recording (TDD).
  - Removed legacy ExecutionTrace struct from executor to use shared trace module.
- Files touched:
  - `D:\Desktop\opencode\forge\executor.rs`
- Known gaps / simplifications:
  - Snapshot builder does not include messages or compaction list yet.
  - Trace not connected to invoke/invoke_with_metrics paths.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Wire SessionSnapshot emission to include messages and compactions.

## 2026-01-21 03:45:34 Phase 3 Round 10 (Snapshot Integration)

- Date: 2026-01-21 03:45:34
- Scope: Phase 3 round 10 - session snapshot integration
- Summary: Integrated SessionSnapshot updates in stream_events and exposed builder helper.
- Changes:
  - Added session snapshot collector to ExecutionConfig.
  - stream_events now appends synthetic node messages and compaction results.
  - build_snapshot now returns collected messages + compactions + trace.
  - Added test validating snapshot updates (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\executor.rs`
- Known gaps / simplifications:
  - Messages are synthetic “node executed” entries, not real chat history.
  - Compactions only recorded when compaction hook triggers.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Phase 3 closeout: document remaining optional items or mark complete.

## 2026-01-21 03:51:12 Phase 4 Round 1 (Compaction Auto Trigger)

- Date: 2026-01-21 03:51:12
- Scope: Phase 4 round 1 - compaction auto trigger policy
- Summary: Added compaction policy enforcement based on message counts with hook inputs.
- Changes:
  - Added compaction policy to ExecutionConfig with setter.
  - stream_events now checks message count before triggering compaction hook.
  - Hook now receives message content when snapshot is attached.
  - Added helpers to resolve message count and collect messages.
  - Updated compaction event test to use policy (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\executor.rs`
- Known gaps / simplifications:
  - Message count is derived from snapshot messages or event history length.
  - Compaction still only fires in stream_events path.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Phase 4 Round 2: compaction prompt/summary slots via hook context object.

## 2026-01-21 03:52:40 Phase 4 Round 2 (Compaction Hook Context)

- Date: 2026-01-21 03:52:40
- Scope: Phase 4 round 2 - compaction hook context
- Summary: Added CompactionContext with prompt hint support and updated hook signature.
- Changes:
  - Added `CompactionContext` carrying messages and prompt_hint.
  - Updated CompactionHook to accept CompactionContext.
  - Updated executor compaction call site to pass context.
  - Added tests for context and updated hook test (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\compaction.rs`
  - `D:\Desktop\opencode\forge\executor.rs`
- Known gaps / simplifications:
  - prompt_hint is optional and not used by executor yet.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Phase 4 Round 3: unify prune/compaction scheduling.

## 2026-01-21 03:54:18 Phase 4 Round 3 (Prune/Compaction Ordering)

- Date: 2026-01-21 03:54:18
- Scope: Phase 4 round 3 - prune/compaction scheduling
- Summary: Added ordering control between prune and compaction in stream_events.
- Changes:
  - Added `prune_before_compaction` flag to ExecutionConfig.
  - Applied pruning before or after compaction based on flag.
  - Added unit test to verify prune-before-compaction ordering (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\executor.rs`
- Known gaps / simplifications:
  - Ordering only applies to stream_events path.
  - Prune order does not yet affect compaction prompt content.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Phase 4 Round 4: trace replay to sink / audit log export.

## 2026-01-21 03:55:35 Phase 4 Round 4 (Trace Replay to Sink)

- Date: 2026-01-21 03:55:35
- Scope: Phase 4 round 4 - replay trace into runtime event sink
- Summary: Added trace replay to emit runtime events for audit/logging.
- Changes:
  - Added `TraceReplay::replay_to_sink` mapping TraceEvent to runtime Event stream.
  - Added unit test for replay emission (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\trace.rs`
- Known gaps / simplifications:
  - Replay uses synthetic session ids and default token usage.
  - Mapping is minimal (NodeStart/Finish/Compacted only).
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Phase 4 Round 5: snapshot export/import IO + versioning.

## 2026-01-21 03:57:06 Phase 4 Round 5 (Snapshot IO + Version)

- Date: 2026-01-21 03:57:06
- Scope: Phase 4 round 5 - snapshot IO + versioning
- Summary: Added snapshot version field and IO helpers for JSON export/import.
- Changes:
  - Added `version` to SessionSnapshot (default 1).
  - Added `SessionSnapshotIo` helpers for to/from JSON.
  - Updated build_snapshot to set version.
  - Added unit test for IO helper roundtrip (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\session.rs`
  - `D:\Desktop\opencode\forge\executor.rs`
  - `D:\Desktop\opencode\forge\mod.rs`
- Known gaps / simplifications:
  - No file system adapter yet (JSON value only).
  - No version migration strategy.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Phase 4 Round 6: session history storage layer (filesystem adapter).

## 2026-01-21 03:58:37 Phase 4 Round 6 (Session Store)

- Date: 2026-01-21 03:58:37
- Scope: Phase 4 round 6 - session storage adapter
- Summary: Added SessionStore filesystem adapter with JSON read/write.
- Changes:
  - Added SessionSnapshotIo string helpers.
  - Added SessionStore with save/load to snapshot.json under session directory.
  - Added unit test for store roundtrip (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\session.rs`
- Known gaps / simplifications:
  - No locking or concurrency strategy for store writes.
  - No version migration handling.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Phase 4 Round 7: event replay/audit log export wiring.

## 2026-01-21 04:00:32 Phase 4 Round 7 (Audit Log Export)

- Date: 2026-01-21 04:00:32
- Scope: Phase 4 round 7 - audit log export from trace replay
- Summary: Added trace replay JSON export and enabled serde for runtime events.
- Changes:
  - Added `TraceReplay::replay_to_json` for audit-log export.
  - Added serde derives for Event/TokenUsage/PermissionReply/ToolState.
  - Added unit test for replay JSON output (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\trace.rs`
  - `D:\Desktop\opencode\forge\event.rs`
  - `D:\Desktop\opencode\forge\tool.rs`
- Known gaps / simplifications:
  - Audit log uses minimal event mapping with synthetic session ids.
  - No file IO helper for audit log yet.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Phase 4 Round 8: closeout / optional file IO for audit log.

## 2026-01-21 04:01:47 Phase 4 Round 8 (Audit Log IO)

- Date: 2026-01-21 04:01:47
- Scope: Phase 4 round 8 - audit log file IO
- Summary: Added audit log writer for replayed trace events.
- Changes:
  - Added `TraceReplay::write_audit_log` to write JSON audit logs to disk.
  - Added unit test for audit log write (TDD).
- Files touched:
  - `D:\Desktop\opencode\forge\trace.rs`
- Known gaps / simplifications:
  - Audit log path handling is minimal (writes snapshot.json only).
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Phase 4 closeout: mark completion and list optional gaps.

## 2026-01-21 04:02:36 Phase 4 Complete (MVP-3)

- Date: 2026-01-21 04:02:36
- Scope: Phase 4 closeout
- Summary: Phase 4 core goals completed (compaction scheduling, audit replay/export, session storage).
- Changes:
  - Marked Phase 4 as complete.
- Files touched:
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / optional follow-ups:
  - Integrate audit log IO with higher-level session manager.
  - Add version migration for snapshots/audit logs.
  - Extend trace mapping to cover more runtime events.
- Validation:
  - N/A (status update only).
- Next steps:
  - Decide Phase 5 scope or pick optional follow-ups.

## 2026-01-21 04:21:07 Repo Restructure (Standard Library Layout)

- Date: 2026-01-21 04:21:07
- Scope: Repository structure refactor
- Summary: Reorganized Forge into standard Rust crate layout with archived legacy docs.
- Changes:
  - Moved core modules into `src/runtime/` and updated `src/lib.rs`.
  - Archived previous README to `docs/README-legacy.md`.
  - Rewrote root `README.md` with concise project overview.
- Files touched:
  - `D:\Desktop\opencode\forge\src\lib.rs`
  - `D:\Desktop\opencode\forge\src\\runtime\*.rs`
  - `D:\Desktop\opencode\forge\docs\README-legacy.md`
  - `D:\Desktop\opencode\forge\README.md`
- Known gaps / simplifications:
  - Older PROGRESS entries reference pre-move file paths.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Add docs index and examples directory (optional).

## 2026-01-21 03:47:01 Phase 3 Complete (MVP-2)

- Date: 2026-01-21 03:47:01
- Scope: Phase 3 closeout
- Summary: Phase 3 core goals completed (compaction/prune + trace/replay + snapshot model).
- Changes:
  - Marked Phase 3 as complete.
- Files touched:
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / optional follow-ups:
  - Replace synthetic node messages with real conversation history.
  - Wire trace/compaction/snapshot to invoke/invoke_with_metrics paths.
  - Add richer snapshot contents (tool events, permissions).
- Validation:
  - N/A (status update only).
- Next steps:
  - Begin Phase 4 or select optional follow-up items.

## 2026-01-21 04:28:13 Runtime Module Rename (langgraph -> runtime)

- Date: 2026-01-21 04:28:13
- Scope: Repository module rename to align with Forge runtime naming
- Summary: Renamed internal module path from `langgraph` to `runtime` and updated references.
- Changes:
  - Renamed `src/langgraph/` to `src/runtime/` and updated `src/lib.rs` exports.
  - Replaced crate/module references to `langgraph` with `runtime` across code and docs.
  - Updated README examples to use `forge::prelude::*` and `runtime` paths.
- Files touched:
  - `D:\Desktop\opencode\forge\src\lib.rs`
  - `D:\Desktop\opencode\forge\src\runtime\*.rs`
  - `D:\Desktop\opencode\forge\README.md`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Archived docs may still reference legacy names (intentional).
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Run tests and commit the rename changes.

## 2026-01-21 05:21:56 Message/Part Model (Phase 5)

- Date: 2026-01-21 05:21:56
- Scope: Phase 5 core message/part model for session runtime
- Summary: Added structured Message/Part types with Event-to-Part mapping and tests.
- Changes:
  - Added `message.rs` with `MessageRole`, `Message`, and `Part` enums.
  - Implemented `Part::from_event` mapping for TextDelta, ToolStart/Result/Error, and TokenUsage.
  - Exported message types via runtime prelude.
  - Added tests covering mapping behavior and default message initialization (TDD).
  - Added `PartialEq` for `TokenUsage` to support comparisons.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\message.rs`
  - `D:\Desktop\opencode\forge\src\runtime\event.rs`
  - `D:\Desktop\opencode\forge\src\runtime\mod.rs`
- Known gaps / simplifications:
  - No integration with SessionSnapshot yet (still uses `SessionMessage`).
  - WSL distro not installed, so WSL lint/CI not run yet.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Install/enable WSL distro and run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` in WSL.
  - Extend SessionSnapshot to use Message/Part or add conversion helpers.

## 2026-01-21 05:36:07 SessionSnapshot Message Conversion (Phase 5)

- Date: 2026-01-21 05:36:07
- Scope: Phase 5 incremental bridge between Message/Part and SessionSnapshot
- Summary: Added conversion helpers to map structured Message/Part into legacy snapshot messages.
- Changes:
  - Added `MessageRole::as_str` helper for stable role serialization.
  - Added `SessionMessage::from_message` to flatten text parts into snapshot content.
  - Added `SessionSnapshot::push_message` to append converted messages.
  - Added TDD coverage for text-part ordering, non-text filtering, and snapshot append.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\message.rs`
  - `D:\Desktop\opencode\forge\src\runtime\session.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Non-text parts are ignored in snapshot conversion (tool outputs/attachments not serialized).
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Use `SessionSnapshot::push_message` (or a richer mapping) in executor/session capture.
  - Extend conversion to include tool results or structured parts if needed.

## 2026-01-21 05:40:25 Snapshot Replay Helpers (Phase 5)

- Date: 2026-01-21 05:40:25
- Scope: Phase 5 snapshot replay helpers for structured messages
- Summary: Added reverse conversion from snapshot messages back to structured Message/Part.
- Changes:
  - Added `MessageRole::from_str` for role parsing (case-insensitive).
  - Added `SessionMessage::to_message` and `SessionSnapshot::to_messages` helpers.
  - Added TDD coverage for role parsing, empty content handling, and unknown role filtering.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\message.rs`
  - `D:\Desktop\opencode\forge\src\runtime\session.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Snapshot replay only restores text content (non-text parts remain unsupported).
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Wire snapshot replay into executor/session restore paths.
  - Extend conversion to include tool results/attachments when needed.

## 2026-01-21 05:46:36 Snapshot Message Filtering (Phase 5)

- Date: 2026-01-21 05:46:36
- Scope: Phase 5 snapshot message hygiene + executor usage
- Summary: Skip empty snapshot entries and standardize executor snapshot writes through Message/Part conversion.
- Changes:
  - `SessionSnapshot::push_message` now ignores messages without text content.
  - `stream_events` builds a structured `Message` and uses snapshot conversion helpers.
  - Added TDD coverage for skipping empty snapshot messages.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\session.rs`
  - `D:\Desktop\opencode\forge\src\runtime\executor.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Snapshot still only stores flattened text; tool outputs/attachments are not preserved.
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Wire snapshot replay (`SessionSnapshot::to_messages`) into restore paths.
  - Extend snapshot conversion to include tool results/attachments when needed.

## 2026-01-21 05:48:41 SessionStore Message Restore (Phase 5)

- Date: 2026-01-21 05:48:41
- Scope: Phase 5 snapshot restore helper
- Summary: Added SessionStore helper to rehydrate structured messages from persisted snapshots.
- Changes:
  - Added `SessionStore::load_messages` to load snapshots and convert to structured messages.
  - Added TDD coverage for restored messages and unknown role filtering.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\session.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Restored messages only contain text parts; tool outputs/attachments are still omitted.
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Use `SessionStore::load_messages` in session restore flows (loop/session manager).
  - Extend snapshot conversion to include tool results/attachments when needed.

## 2026-01-21 05:51:01 ExecutionConfig Snapshot Seeding (Phase 5)

- Date: 2026-01-21 05:51:01
- Scope: Phase 5 snapshot restore wiring helper
- Summary: Added ExecutionConfig helper to seed session snapshots from structured messages.
- Changes:
  - Added `ExecutionConfig::with_snapshot_messages` for snapshot seeding.
  - Added TDD coverage for seeding behavior and empty-message filtering.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\executor.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Snapshot still only stores flattened text; tool outputs/attachments are not preserved.
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Use `with_snapshot_messages` when restoring sessions in loop/session manager.
  - Extend snapshot conversion to include tool results/attachments when needed.

## 2026-01-21 05:53:37 SessionState Skeleton (Phase 5)

- Date: 2026-01-21 05:53:37
- Scope: Phase 5 session state core model
- Summary: Added a minimal SessionState model with routing and tool-call tracking.
- Changes:
  - Added `SessionState`, `SessionRouting`, `ToolCallRecord`, and `ToolCallStatus`.
  - Added helpers for routing transitions, step advance, and tool call updates.
  - Exported session state types via runtime module prelude.
  - Added TDD coverage for initialization, routing transitions, and tool call tracking.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\session_state.rs`
  - `D:\Desktop\opencode\forge\src\runtime\mod.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - SessionState is not wired into LoopNode or executor yet.
  - Tool call records do not track payloads or timestamps yet.
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Integrate SessionState into LoopNode/loop execution flow.
  - Add message/part merge helpers for pending_parts and finalized messages.

## 2026-01-21 05:55:06 SessionState Part Merge (Phase 5)

- Date: 2026-01-21 05:55:06
- Scope: Phase 5 session state merge helpers
- Summary: Added pending_parts merge/finalize helper on SessionState with TDD coverage.
- Changes:
  - Added `SessionState::finalize_message` to merge pending parts into a finalized Message.
  - Added tests covering merge order, pending_parts clearing, and empty pending handling.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\session_state.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - finalize_message does not handle message_id updates or tool-call correlation yet.
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Integrate SessionState finalize flow into LoopNode/loop execution.
  - Add helpers to append pending parts by event (TextDelta/ToolResult).

## 2026-01-21 05:56:31 SessionState Event Mapping (Phase 5)

- Date: 2026-01-21 05:56:31
- Scope: Phase 5 session state event ingestion
- Summary: Added SessionState helper to map runtime events into pending parts and tool call status.
- Changes:
  - Added `SessionState::apply_event` to capture TextDelta/ToolStart/ToolResult/ToolError.
  - Added TDD coverage for pending part updates and tool lifecycle tracking.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\session_state.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Event mapping ignores TextFinal/TokenUsage/Attachment for now.
  - Tool call tracking stores minimal status only (no timestamps).
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Integrate SessionState apply_event/finalize flow into LoopNode/loop execution.
  - Expand event mapping to include TextFinal and TokenUsage parts.

## 2026-01-21 05:58:24 SessionState TextFinal/TokenUsage (Phase 5)

- Date: 2026-01-21 05:58:24
- Scope: Phase 5 session state event ingestion expansion
- Summary: Added TextFinal and TokenUsage handling to SessionState event mapping.
- Changes:
  - Added `Event::TextFinal` variant and mapping to `Part::TextFinal`.
  - Extended `SessionState::apply_event` to handle TextFinal and StepFinish token usage.
  - Added TDD coverage for text final and token usage ingestion.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\event.rs`
  - `D:\Desktop\opencode\forge\src\runtime\message.rs`
  - `D:\Desktop\opencode\forge\src\runtime\session_state.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - apply_event still ignores attachments and errors beyond tool lifecycle.
  - Tool call tracking stores minimal status only (no timestamps).
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Integrate SessionState apply_event/finalize flow into LoopNode/loop execution.
  - Add attachment/error handling in event ingestion.

## 2026-01-21 06:00:26 SessionState Attachment/Error (Phase 5)

- Date: 2026-01-21 06:00:26
- Scope: Phase 5 event ingestion expansion
- Summary: Added Attachment/Error events and SessionState mapping for them.
- Changes:
  - Added `Event::Attachment` and `Event::Error` variants.
  - Extended `Part::from_event` and `SessionState::apply_event` to map attachments/errors.
  - Added TDD coverage for attachment/error mapping in message and session state tests.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\event.rs`
  - `D:\Desktop\opencode\forge\src\runtime\message.rs`
  - `D:\Desktop\opencode\forge\src\runtime\session_state.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Attachment payload is untyped JSON without schema enforcement.
  - Tool call tracking stores minimal status only (no timestamps).
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Integrate SessionState apply_event/finalize flow into LoopNode/loop execution.
  - Add schema/metadata helpers for attachments if needed.

## 2026-01-21 06:04:18 LoopNode SessionState Wiring (Phase 5)

- Date: 2026-01-21 06:04:18
- Scope: Phase 5 loop integration
- Summary: Added LoopNode helper to update SessionState from emitted events.
- Changes:
  - Added `LoopNode::run_with_session_state` with an event sink adapter.
  - Added SessionState-aware sink to apply events before forwarding.
  - Added TDD coverage for LoopNode event ingestion updating SessionState.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\loop.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - SessionState finalize flow is not automatically triggered after runs.
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Wire SessionState finalize flow (pending_parts -> messages) in loop execution.
  - Add session state restoration into loop/session manager flows.

## 2026-01-21 17:07:52 LoopNode SessionState Finalize (Phase 5)

- Date: 2026-01-21 17:07:52
- Scope: Phase 5 loop finalize integration
- Summary: Added LoopNode helper to finalize SessionState pending parts after execution.
- Changes:
  - Added `LoopNode::run_with_session_state_and_finalize` to finalize pending parts into a Message.
  - Added SessionState finalize tests for message creation and no-op when empty.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\loop.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Finalization role is provided by caller; no automatic role inference yet.
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Add session state restoration into loop/session manager flows.
  - Consider automatic role selection or per-event role tracking.

## 2026-01-21 17:16:14 SessionPhase State Machine (Phase 5)

- Date: 2026-01-21 17:16:14
- Scope: Phase 5 session state machine scaffolding
- Summary: Added SessionPhase enum and transition helpers on SessionState.
- Changes:
  - Added `SessionPhase` with core phases (UserInput/Thinking/Streaming/Tool/Finalize/Completed/Interrupted/Resumed).
  - Added phase transition helpers on SessionState and defaulted new state to UserInput.
  - Exported SessionPhase in runtime prelude.
  - Added TDD coverage for phase initialization and transitions.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\session_state.rs`
  - `D:\Desktop\opencode\forge\src\runtime\mod.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Phase transitions are not enforced/validated yet.
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Validate phase transitions (guard illegal transitions) and emit state transition events.
  - Integrate phase updates into LoopNode/loop execution flow.

## 2026-01-21 17:18:34 SessionPhase Transition Guards (Phase 5)

- Date: 2026-01-21 17:18:34
- Scope: Phase 5 session phase validation
- Summary: Added transition guard helpers for SessionPhase with TDD coverage.
- Changes:
  - Added `can_transition` and `try_transition` on SessionState with a minimal allowed path.
  - Added tests for happy-path transitions and invalid transition rejection.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\session_state.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Transition rules are minimal and not yet enforced by mark_* helpers.
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\\cargo.exe test`
- Next steps:
  - Wire phase transitions into LoopNode/SessionState flows (use try_transition).
  - Emit explicit state transition events for replay/audit.

## 2026-01-21 17:20:44 SessionState Event Phase Updates (Phase 5)

- Date: 2026-01-21 17:20:44
- Scope: Phase 5 session phase + event ingestion
- Summary: SessionState now advances phase when ingesting core events.
- Changes:
  - apply_event now advances phase for TextDelta/TextFinal/ToolStart/ToolResult/ToolError/StepFinish.
  - Added TDD coverage for phase changes on TextDelta, ToolStart, ToolResult, and StepFinish.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\session_state.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Phase updates are best-effort (try_transition failure is ignored).
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\\cargo.exe test`
- Next steps:
  - Enforce phase transitions (surface errors) and emit state transition events.
  - Hook phase updates into LoopNode finalize flow (use try_transition).

## 2026-01-21 17:23:24 SessionPhase Transition Events (Phase 5)

- Date: 2026-01-21 17:23:24
- Scope: Phase 5 state transition events
- Summary: Added SessionPhase transition event emission helpers and Event PartialEq.
- Changes:
  - Added `Event::SessionPhaseChanged` and derived `PartialEq` for Event.
  - Added `SessionState::try_transition_with_event` returning transition events.
  - Added TDD coverage for transition events and no-op same-phase transitions.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\event.rs`
  - `D:\Desktop\opencode\forge\src\runtime\session_state.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Transition events are not yet emitted by LoopNode/SessionState flows.
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\\cargo.exe test`
- Next steps:
  - Emit SessionPhaseChanged from loop/session flows when transitions occur.
  - Convert apply_event phase updates to use try_transition_with_event and emit events.

## 2026-01-21 17:25:42 SessionState Event Transition Output (Phase 5)

- Date: 2026-01-21 17:25:42
- Scope: Phase 5 event-driven phase transitions
- Summary: Added apply_event_with_events to return phase change events on ingestion.
- Changes:
  - Added `SessionState::apply_event_with_events` to return transition events alongside handling.
  - Updated `apply_event` to delegate to the new helper.
  - Added TDD coverage for phase change events and tool phase steps.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\session_state.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Transition events are not yet emitted by LoopNode/SessionState sinks.
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\\cargo.exe test`
- Next steps:
  - Emit SessionPhaseChanged from loop/session flows (hook apply_event_with_events).
  - Consider surfacing invalid transitions instead of ignoring.

## 2026-01-21 18:03:47 Loop SessionPhase Emission (Phase 5)

- Date: 2026-01-21 18:03:47
- Scope: Phase 5 loop event emission
- Summary: Loop SessionState sink now emits phase change events during ingestion.
- Changes:
  - SessionStateSink emits `SessionPhaseChanged` events returned by `apply_event_with_events`.
  - Added TDD coverage to assert phase change events are emitted in LoopNode flows.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\loop.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Invalid transitions still ignored (no explicit error events).
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\\cargo.exe test`
- Next steps:
  - Emit explicit errors on invalid transitions.
  - Use phase transition events in audit/replay pipeline.

## 2026-01-21 18:09:45 Invalid Phase Transition Events (Phase 5)

- Date: 2026-01-21 18:09:45
- Scope: Phase 5 transition error signaling
- Summary: apply_event_with_events now emits explicit rejection events for invalid phase transitions.
- Changes:
  - Added `Event::SessionPhaseTransitionRejected` and emit it on invalid transitions.
  - apply_event_with_events now returns phase rejection events in addition to phase changes.
  - Added TDD coverage for invalid transition rejection event output.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\event.rs`
  - `D:\Desktop\opencode\forge\src\runtime\session_state.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Rejection events are not yet surfaced by LoopNode sink.
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\\cargo.exe test`
- Next steps:
  - Emit rejection events from SessionStateSink in loop flows.
  - Decide whether invalid transitions should hard-fail execution.

## 2026-01-21 21:36:49 Loop Phase Rejection Emission (Phase 5)

- Date: 2026-01-21 21:36:49
- Scope: Phase 5 loop event propagation
- Summary: Loop SessionState sink now emits phase rejection events as well.
- Changes:
  - SessionStateSink forwards `SessionPhaseTransitionRejected` events.
  - Added TDD coverage for rejection event emission in LoopNode.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\loop.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Invalid transitions still do not hard-fail; only emit rejection events.
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\\cargo.exe test`
- Next steps:
  - Decide whether rejection events should interrupt execution.
  - Propagate transition events into audit/replay pipeline.

## 2026-01-21 22:24:42 Event Metadata Records (Phase 5)

- Date: 2026-01-21 22:24:42
- Scope: Phase 5 event protocol metadata (history)
- Summary: Added event metadata records (event_id/timestamp/seq) for event history capture.
- Changes:
  - Added `EventMeta`, `EventRecord`, and `EventSequencer` in runtime events.
  - RecordingSink now stores `EventRecord` with metadata for history buffers.
  - Prune policy now operates on `EventRecord` while preserving tool event pruning.
  - Added TDD coverage for sequencer metadata and record wrapper.
  - Updated executor/prune tests to use history records.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\event.rs`
  - `D:\Desktop\opencode\forge\src\runtime\executor.rs`
  - `D:\Desktop\opencode\forge\src\runtime\prune.rs`
  - `D:\Desktop\opencode\forge\src\runtime\mod.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Metadata is captured only in history buffers; emitted events still lack event_id/timestamp/seq.
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Decide how to attach metadata to the emitted Event stream (SSE/CLI/IDE).
  - Add event_id/timestamp/seq to protocol output and replay ordering.

## 2026-01-21 22:30:11 Event Record Sink (Phase 5)

- Date: 2026-01-21 22:30:11
- Scope: Phase 5 protocol metadata output hook
- Summary: Added EventRecordSink to surface event metadata alongside emitted events.
- Changes:
  - Added `EventRecordSink` + `NoopEventRecordSink` to runtime events.
  - ExecutionConfig now accepts an optional event record sink.
  - RecordingSink forwards `EventRecord` metadata to the sink while emitting events.
  - Added TDD coverage for record sink emission with metadata.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\event.rs`
  - `D:\Desktop\opencode\forge\src\runtime\executor.rs`
  - `D:\Desktop\opencode\forge\src\runtime\mod.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Record sink is optional; existing SSE/CLI/IDE adapters still need wiring.
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Wire EventRecordSink into SSE/CLI/IDE adapters and event streaming output.
  - Define replay ordering guarantees using seq/timestamp.

## 2026-01-21 22:32:44 Trace Replay Record Sink (Phase 5)

- Date: 2026-01-21 22:32:44
- Scope: Phase 5 trace replay metadata
- Summary: Added trace replay path that emits EventRecord metadata with ordering.
- Changes:
  - Added `TraceReplay::replay_to_record_sink` using EventSequencer metadata.
  - Reused trace→runtime Event mapping to avoid duplicate logic.
  - Added TDD coverage for record sink emission ordering and metadata.
- Files touched:
  - `D:\Desktop\opencode\forge\src\runtime\trace.rs`
  - `D:\Desktop\opencode\forge\PROGRESS.md`
- Known gaps / simplifications:
  - Record replay uses fresh sequencing (not persisted).
  - WSL distro not available, so WSL lint/CI not run.
- Validation:
  - `C:\Users\10758\.cargo\bin\cargo.exe test`
- Next steps:
  - Decide whether trace replay should preserve original seq/timestamps.
  - Expose trace replay record sink through platform adapters.
