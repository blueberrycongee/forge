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
  - Added LoopNode abstraction with un and into_node methods.
  - Added loop_node_emits_events test (TDD) to verify event emission and state update.
  - Exported LoopNode in module prelude (module name escaped as #loop).
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
