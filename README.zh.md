# Forge

Language: [English README](README.md) | 中文

Forge 是一个 Rust 框架，用于构建有状态、事件驱动的 Agent 运行时。
它强调流式执行、工具生命周期、权限控制，以及可审计的可观测性。

## 状态

处于活跃开发中，API 仍可能变化。

## 为什么是 Forge

- **默认流式**：文本、工具、权限、压缩、运行生命周期等事件统一输出。
- **工具驱动编排**：面向 LLM ↔ 工具的循环抽象，不强制工具命名。
- **安全可控**：权限策略与恢复流，便于 human-in-the-loop。
- **可追溯会话**：快照、检查点与回放，方便审计与调试。

## 核心概念

- **StateGraph / CompiledGraph**：以图节点编排异步流程。
- **LoopNode / LoopContext**：可调用工具、输出事件并支持中断恢复的循环。
- **ToolRegistry / ToolDefinition / ToolOutput**：工具契约、生命周期事件与结构化输出。
- **PermissionPolicy / PermissionSession**：allow/ask/deny 权限决策与恢复。
- **事件输出**：JSONL / SSE 等事件流输出。
- **SessionState / Trace**：运行状态、Token 统计与可回放追踪。

## 安装

Forge 目前尚未发布到 crates.io，请使用 git 依赖：

```toml
[dependencies]
forge = { git = "https://github.com/blueberrycongee/forge", rev = "<commit>" }
```

建议固定 `rev` 以确保构建可复现。待发布 tag 或 crates.io 版本后可切换。

## 快速开始：StateGraph

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

## 快速开始：工具 + LoopNode

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

## 事件流输出

Forge 通过 `EventSink` 输出结构化事件，内置：

- `JsonLineEventSink` / `JsonLineEventRecordSink`
- `SseEventSink` / `SseEventRecordSink`

位于 `forge::runtime::output`。

## 目录结构

```text
src/
  runtime/
    graph.rs          # StateGraph 与路由
    executor.rs       # 执行引擎与检查点
    loop.rs           # LoopNode / LoopContext
    tool.rs           # 工具与生命周期
    permission.rs     # 权限策略
    event.rs          # 事件协议
    session_state.rs  # 运行状态与 reducer
    session.rs        # 快照与附件存储
    trace.rs          # Trace / Replay
tests/
```

## 开发

```bash
cargo test
cargo clippy
```

## 贡献

请阅读 `CONTRIBUTING.md`，并遵守 `CODE_OF_CONDUCT.md`。安全问题请见
`SECURITY.md`。

## 许可证

MIT，详见 `LICENSE`。
