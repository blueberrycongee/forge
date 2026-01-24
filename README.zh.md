# Forge

Language: [English README](README.md) | 中文

---

## 概述

Forge 是一个 Rust 框架，用于构建有状态、事件驱动的 Agent 运行时，
关注流式执行、工具生命周期、权限控制和可审计可观测性。

## 状态

- 处于活跃开发中
- Phase 1-4 已完成（见 `PROGRESS.md`）

## 特性

- 基于图的异步执行
- 运行时事件流（文本、工具、权限、压缩）
- Tool-driven LoopNode runtime
- allow/ask/deny 权限门禁与恢复流
- 工具注册表 + 生命周期事件 + 结构化输出
- compaction + prune 策略与 hook
- Trace/Replay 支持审计
- 会话快照导出/导入与文件存储

## 快速开始

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

## 目录结构

- `src/lib.rs` - 模块入口与 prelude
- `src/runtime/graph.rs` / `src/runtime/executor.rs` - 图构建与执行
- `src/runtime/event.rs` - 事件协议
- `src/runtime/loop.rs` - LoopNode 运行时
- `src/runtime/tool.rs` - 工具生命周期与注册表
- `src/runtime/permission.rs` - 权限与恢复流
- `src/runtime/compaction.rs` / `src/runtime/prune.rs` - 会话控制策略
- `src/runtime/trace.rs` - trace/replay
- `src/runtime/session.rs` - 快照与存储

## 文档

- `docs/README-legacy.md` - 归档的长文说明

## 许可证

MIT，详见 `LICENSE`。
