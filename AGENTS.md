# forge Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-01-25

## Active Technologies
- Files (run logs/checkpoints already file-backed; attachments may reference files) (001-tool-context)

- Rust 2021 (edition 2021) + serde, serde_json, uuid, chrono (dev: futures) (001-forge-opencode-orchestration)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test; cargo clippy

## Code Style

Rust 2021 (edition 2021): Follow standard conventions

## Recent Changes
- 001-tool-context: Added Rust 2021 (edition 2021) + serde, serde_json, uuid, chrono (dev: futures)

- 001-forge-opencode-orchestration: Added Rust 2021 (edition 2021) + serde, serde_json, uuid, chrono (dev: futures)

<!-- MANUAL ADDITIONS START -->
## Project Overview

Forge is a Rust framework for building agent orchestration runtimes. It provides
state-graph execution, event streaming, tool calls with lifecycle metadata,
permission gating, and interrupt/resume via checkpoints. The runtime is designed
to be async-friendly and structured around a consistent event protocol.

## Architecture Summary

- State graphs compile into executable plans (StateGraph -> CompiledGraph).
- Execution emits Event records (text/tool/permission/compaction/run lifecycle).
- LoopNode is a streaming loop abstraction that can invoke tools and emit events.
- Tools are registered in a ToolRegistry and run through a ToolExecutor.
- Permissions are governed by PermissionPolicy/PermissionSession.
- Interrupts return Checkpoints that can be resumed with ResumeCommand.

## Core Runtime Modules

- src/runtime/executor.rs: execution engine, routing, checkpoints, run events.
- src/runtime/graph.rs: StateGraph and edge routing.
- src/runtime/loop.rs: LoopNode and tool execution context.
- src/runtime/tool.rs: tool contracts, registry, schemas.
- src/runtime/permission.rs: permission rules, session, snapshot.
- src/runtime/event.rs: event protocol and sequencing.
- src/runtime/session_state.rs: event-to-state reducer and run metadata.
- src/runtime/session.rs: snapshots and checkpoint persistence.

## Event Protocol

- Permission flow uses PermissionAsked and PermissionReplied events.
- Run lifecycle events include RunStarted, RunPaused, RunResumed, RunCompleted, RunFailed.
- Tool events include ToolStart, ToolResult, ToolError, ToolStatus.
- Compaction and prune policies emit Compaction events when enabled.

## Checkpoints and Resume

- Checkpoint includes run_id, checkpoint_id, created_at, state, next_node,
  pending_interrupts, and resume_values.
- Resume values are injected into state under keys like resume:{node}.

## Tests

- Unit tests live under src/runtime/*.
- Integration tests live under tests/integration/.
- Primary commands: cargo test; cargo clippy.
<!-- MANUAL ADDITIONS END -->
