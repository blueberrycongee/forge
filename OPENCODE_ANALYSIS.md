# OpenCode Value Abstraction and Transferable Advantages (for A Framework)

## Goal
This document explains which OpenCode details matter to understand, what can be abstracted, and what does NOT need to be copied, so A Framework can absorb the real advantages.

> Short answer: we need solid understanding, but not line-by-line cloning. The valuable parts are the abstractions: event protocol, session loop, tool lifecycle, permission model, and subtask mechanism.

---

## OpenCode Core Value (Abstracted Layers)

### 1) Unified Event Protocol (Session + Part + Event Stream)
- Normalizes LLM tokens, tools, patches, permissions, summaries into one event stream.
- CLI/TUI/IDE can consume the same protocol.
- Value: low coupling, consistent UX across clients.

### 2) Session State Machine (SessionProcessor)
- The core is a streaming loop, not a DAG.
- Handles streaming output + tool calls + permission denials + retry + compaction.
- Value: smooth interaction and predictable behavior.

### 3) Tool Lifecycle (Tool State Machine)
- tool.pending -> tool.running -> tool.completed / tool.error
- Structured input/output/error/metadata per tool call.
- Value: observability and replay.

### 4) Fine-Grained Permission Model
- Rules by tool/path/mode.
- Can interrupt or continue on deny.
- Value: safety and control.

### 5) Subtask / Subagent Mechanism
- Parent session spawns child sessions (Task Tool).
- Child outputs summary back to parent.
- Value: parallel exploration + reduced context pressure.

### 6) Provider Abstraction and Model Flexibility
- Multiple providers, variants, auth modes, custom providers.
- Value: real-world usability.

---

## OpenCode Limitations (Where A Framework Can Improve)

### 1) Weaker explicit orchestration
- Core relies on a single state loop.
- No explicit graph visualization or subgraph reuse.

### 2) Event protocol and execution logic are tightly coupled
- SessionProcessor is highly centralized.
- Hard to swap execution strategies or node semantics.

### 3) Subagent collaboration is implicit
- Subagent calls are hidden behind Task Tool.
- Harder to debug or orchestrate explicitly.

### 4) Extension cost is high
- New flow or mid-step often requires changing the core loop.

---

## Do we need to deeply understand OpenCode details?

**Yes, but selectively.**

Must fully understand (A Framework must absorb):
- Event types and lifecycle details (text/tool/step/patch/permission)
- Session loop decision logic (continue / stop / compact)
- Tool execution timing and state updates
- Permission evaluation and deny behavior

Can be deferred or simplified:
- Provider compatibility details
- UI/CLI specifics
- Full patch/diff system

---

## A Framework Upgrade Mapping (from OpenCode strengths)

### Must-have (framework level)
1. **Event Protocol Layer**
   - Event enum: TextDelta / ToolStart / ToolResult / StepStart / StepFinish / Patch / Permission / Status
   - EventSink trait (SSE / CLI / UI)

2. **Streaming Node Execution**
   - Nodes can emit event streams, not just final state
   - Add stream_events entry point

3. **LoopNode (SessionProcessor equivalent)**
   - A pluggable node that runs the OpenCode-style loop
   - Single node can reproduce OpenCode experience

4. **ToolLifecycle + Permission as first-class concepts**
   - Core engine understands tool states and permission checks

### Optional enhancements (go beyond LangGraph)
5. **SubGraph / Nested Graph**
   - Subagent can be a subgraph

6. **Graph-level Trace / Replay**
   - Unified event + node trace + replay

---

## Recommended Next Research (OpenCode deep dive)

1. SessionPrompt / SessionProcessor state transitions
2. ToolRegistry selection rules and model/permission coupling
3. Permission rule merging logic
4. Task Tool child session lifecycle and summary merge
5. Compaction and summary triggers

---

## Summary
- OpenCode value: event protocol + session loop + tool lifecycle + permissions + subtasking
- A Framework should make these abstractions reusable and pluggable
- Do not copy UI or provider details yet; replicate execution semantics first

---

If this direction is correct, next steps can include:
- Rust API sketch (Event / EventSink / LoopNode / ToolRegistry)
- A phased task list
- Minimal diffs to current langgraph executor
