# Forge 1.0 Contracts and Conformance Matrix

Last updated: 2026-02-06

## Goal

Define a 1.0-stable contract surface for Forge and map each contract to executable conformance tests.
This document is the baseline for reaching enterprise-grade stability.

## 1.0 Contract Scope

The following surfaces should be treated as semver-protected in `1.x`:

1. Runtime event protocol in `src/runtime/event.rs`
2. Checkpoint and resume semantics in `src/runtime/executor.rs`
3. Session state machine semantics in `src/runtime/session_state.rs`
4. Tool lifecycle and attachment behavior in `src/runtime/tool.rs`
5. Permission decision semantics in `src/runtime/permission.rs`
6. Component interfaces (`ChatModel`/`EmbeddingModel`/`Retriever`) in `src/runtime/component.rs`
7. Session snapshot and run log formats in `src/runtime/session.rs`

## Contract Checklist (File/Type Level)

## A. Event Protocol Contract

- `Event` enum variants and required fields are stable.
- `EventMeta { event_id, timestamp_ms, seq }` semantics are stable.
- `EventRecord` ordering (`cmp_meta`, `sort_records_by_meta`) is stable.
- `PermissionReply` and `ToolUpdate` wire shapes are stable.

## B. Checkpoint/Resume Contract

- `Checkpoint<S>` field set is stable:
  - `run_id`, `checkpoint_id`, `created_at`, `state`, `next_node`
  - `pending_interrupts`, `iterations`, `resume_values`
- `ExecutionResult::{Complete, Interrupted}` semantics are stable.
- Resume value injection key format (`resume:{node}`) is stable.

## C. Session State Machine Contract

- `SessionPhase` states and transition rules are stable.
- `SessionRouting::{Next, Complete, Interrupt}` semantics are stable.
- Tool call tracking (`ToolCallRecord`) and permission decision recording are stable.
- Message finalization behavior for `pending_parts` is stable.

## D. Tool Lifecycle Contract

- `ToolCall`/`ToolDefinition` shapes are stable.
- `ToolState` transition semantics are stable (`Pending -> Running -> Completed/Error`).
- `ToolOutput` attachment emission and metadata behavior are stable.
- `AttachmentPolicy` inline/reference conversion semantics are stable.

## E. Permission Contract

- Rule ordering and wildcard matching behavior are stable.
- `PermissionSession` override precedence is stable:
  - `reject` > `always` > `once` > base policy
- `PermissionRequest` event payload shape is stable.
- Resume command mapping to permission reply is stable.

## F. Component Contract

- `ChatRequest`/`ChatResponse` field semantics are stable.
- `ChatModel::generate` and `ChatModel::stream` behavior contracts are stable.
- `EmbeddingModel` and `Retriever` trait behavior contracts are stable.

## G. Persistence Contract

- `SessionSnapshot` JSON shape and `version` behavior are stable.
- `CheckpointRecord` JSON shape compatibility is stable.
- `RunLogStore` JSONL record format is stable.

## Conformance Matrix (Current State)

| Contract Area | Core Invariant | Current Coverage | Status | Next Action | Priority |
| --- | --- | --- | --- | --- | --- |
| Event protocol | Ordered metadata + stable event shapes | `src/runtime/event.rs` unit tests + `tests/contract/serialization_contract.rs` | Good | Add broader fixture set for additional event variants | P1 |
| Checkpoint/resume | Pause/resume preserves state and emits lifecycle events | `tests/integration/pause_resume.rs` | Good | Add interrupt-id targeted resume cases | P1 |
| Permission flow | ask/allow/deny semantics with replies | `tests/integration/permission_flow.rs`, `tests/integration/tool_context_permission.rs`, `src/runtime/permission.rs` table tests | Good | Add fuzz/property tests for pattern matcher | P1 |
| Tool lifecycle | tool start/update/result/error sequencing | `tests/unit/tool_interface.rs` | Good | Add cancellation-order conformance tests | P1 |
| Attachment policy | large payload conversion + store behavior | `tests/unit/attachment_policy.rs`, `tests/unit/tool_context_edge_cases.rs` | Good | Add size/mime backward compatibility tests | P1 |
| Session phase machine | phase transitions and rejection events | `src/runtime/session_state.rs` unit tests | Partial | Add event replay to phase consistency tests | P0 |
| Snapshot/log format | serialize/deserialize compatibility | `src/runtime/session.rs` unit tests + `tests/contract/serialization_contract.rs` | Good | Add explicit migration fixture pairs for future versions | P1 |
| Component interfaces | chat/retrieval/embedding baseline behavior | `src/runtime/component.rs` unit tests | Partial | Add provider-neutral contract tests across adapters | P1 |
| OpenAI adapter | request/response/error mapping | `src/runtime/provider/openai.rs` unit tests | Partial | Add record/replay HTTP fixture conformance tests | P1 |
| External contract spec | tool-context API endpoints documented | `tests/contract/tool_context_contract.rs` + `specs/001-tool-context/contracts/tool-context.openapi.yaml` | Basic | Expand schema-level validation and examples | P1 |

## Test Discovery Baseline

As of this update, suite entry points are enabled so Cargo runs subdirectory tests:

- `tests/integration_suite.rs`
- `tests/unit_suite.rs`
- `tests/contract_suite.rs`

Without these entry points, files under `tests/integration`, `tests/unit`, and `tests/contract` are not auto-discovered by `cargo test`.

## Priority Roadmap

## Phase 0 (Now, 1-2 weeks): Contract Freeze Draft

1. Mark all P0 contracts as `proposed-frozen` in this doc.
2. Add golden fixtures for event, checkpoint, and snapshot serialization. (Done)
3. Enforce CI gate: conformance suite must pass before merge. (Done in `.github/workflows/ci.yml`)

## Phase 1 (2-4 weeks): Conformance Expansion

1. Add table-driven permission and state-machine edge case tests. (Permission table tests done)
2. Add tool lifecycle ordering tests (including cancel/abort paths). (Done for success/error/attachment terminal paths)
3. Add regression fixtures for prior serialized records.

## Phase 2 (4-8 weeks): Enterprise Reliability

1. Add deterministic replay tests from run logs to final state.
2. Add fault-injection tests: sink failure, tool failure, attachment store failure.
3. Add performance baselines for event throughput and resume latency.

## Phase 3 (8+ weeks): Release Hardening

1. Publish compatibility statement for 1.0 contract surfaces.
2. Add migration notes and deprecation policy.
3. Cut release candidates with frozen conformance reports.

## 1.0 Definition of Done

1. All P0 conformance tests are implemented and green in CI.
2. Serialization compatibility fixtures are stable across two consecutive release candidates.
3. No undocumented breaking change in the 1.0 contract scope.
4. Public upgrade and compatibility policy is documented.
