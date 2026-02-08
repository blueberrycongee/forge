# Forge

Language: [English README](README.md) | 中文

[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-active%20development-yellow.svg)](#项目状态)

Forge 是一个面向 Rust 的 Agent 编排运行时，适合构建长时运行、有状态的工作流。
它关注可恢复的图执行、事件流协议、工具生命周期、权限治理，以及基于检查点的中断/恢复。

## 概览

Forge 提供的是底层运行时能力，不预设提示词风格、Agent 架构或工具命名方式。

当你需要以下能力时，可以选择 Forge：

- 在图节点间进行显式状态迁移
- 用结构化事件支持 UI/CLI 流式输出和审计日志
- 对工具调用应用权限与附件策略
- 在 human-in-the-loop 与失败恢复场景中继续执行

## 核心能力

- 可恢复执行：将状态图编译为可断点恢复的执行计划。
- 事件流协议：统一输出文本、工具、权限、压缩与运行生命周期事件。
- 工具优先运行循环：以生命周期元数据编排 LLM 到工具的交互。
- 权限系统：支持 allow/ask/deny 与会话级决策持久化。
- 会话模型：快照、回放追踪与运行元数据，便于调试和审计。
- Provider 适配：内置 OpenAI `ChatModel` 适配器（`runtime::provider::openai`）。

## 安装

Forge 目前尚未发布到 crates.io，请使用 Git 依赖并固定 commit：

```toml
[dependencies]
forge = { git = "https://github.com/blueberrycongee/forge", rev = "<commit>" }
```

## 快速开始

### 1) 构建 StateGraph

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

### 2) 使用权限控制运行工具循环

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

### 3) 使用 OpenAI Provider 适配器

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

## 架构模块

| 模块 | 责任 |
| --- | --- |
| `runtime::graph` | 构建并编译状态图（`StateGraph -> CompiledGraph`） |
| `runtime::executor` | 运行生命周期、检查点、恢复命令与流式执行 |
| `runtime::loop` | 工具循环抽象（`LoopNode`、`LoopContext`） |
| `runtime::tool` | 工具契约、注册、元数据、附件与状态 |
| `runtime::permission` | 权限策略与会话决策 |
| `runtime::event` | 运行时事件协议与序列化 |
| `runtime::session_state` | 事件到状态的归约与运行元数据 |
| `runtime::session` | 会话快照与持久化辅助 |
| `runtime::provider` | 外部模型适配（当前为 OpenAI chat） |

## 事件协议

Forge 通过 `EventSink` / `EventRecordSink` 输出结构化事件。
内置流输出：

- `JsonLineEventSink` 与 `JsonLineEventRecordSink`
- `SseEventSink` 与 `SseEventRecordSink`

主要事件族包括：

- 运行生命周期：`RunStarted`、`RunPaused`、`RunResumed`、`RunCompleted`、`RunFailed`
- 文本与消息：`TextDelta`、`TextFinal`、`Attachment`、`Error`
- 工具生命周期：`ToolStart`、`ToolUpdate`、`ToolResult`、`ToolError`、`ToolStatus`
- 权限流：`PermissionAsked`、`PermissionReplied`
- 会话控制：压缩与 phase 迁移事件

## 示例

可运行示例位于 `/examples`：

- `cargo run --example core_workflow`
- `cargo run --example multi_agent_graph`
- `cargo run --example tool_context`

## 项目状态

Forge 目前处于 pre-1.0 的活跃开发阶段。

- 公开 API 与兼容性治理规则已文档化，并在 PR 审查中执行。
- pre-1.0 阶段仍可能有破坏性变更，但必须附带迁移说明。
- 发布节奏按里程碑推进，不按固定时间频率。
- 生产使用建议固定 commit，并做兼容性回归测试。

## 开发

```bash
cargo test
cargo clippy
```

## 文档

- 运行评测说明：[EVALUATION.md](EVALUATION.md)
- 进度记录：[PROGRESS.md](PROGRESS.md)
- 贡献指南：[CONTRIBUTING.md](CONTRIBUTING.md)
- 安全策略：[SECURITY.md](SECURITY.md)
- API 兼容策略：[docs/api-compatibility-policy.md](docs/api-compatibility-policy.md)
- 弃用策略：[docs/deprecation-policy.md](docs/deprecation-policy.md)
- 升级指南：[docs/upgrading.md](docs/upgrading.md)
- 变更日志：[CHANGELOG.md](CHANGELOG.md)
- 1.0 合同与一致性矩阵：[docs/forge-1.0-contracts-and-conformance.md](docs/forge-1.0-contracts-and-conformance.md)

## 贡献

欢迎贡献。提交前请阅读 `CONTRIBUTING.md` 并遵守 `CODE_OF_CONDUCT.md`。

## 许可证

MIT，详见 [LICENSE](LICENSE)。
