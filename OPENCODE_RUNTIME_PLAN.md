# OpenCode Runtime/编排层对照清单（含验证与改造要点）

> 范围声明：仅讨论 runtime/编排层，不涉及模型能力或提示词写法。
> 证据来源：OpenCode 与 Claude Code 官方文档；对照引用第三方/用户报告。所有不确定处标注“推断”。

---

## A. 结论摘要（≤10条）

1) **Plan/Build 主代理 + allow/ask/deny 权限**：Plan 默认对文件编辑和 bash 设为 ask，把“先规划、后执行”变成系统机制，降低误改风险。证据：OpenCode Agents + Permissions。（见 Sources）
2) **Subagent 子会话 + 父/子导航 + General 并行 + Explore 只读**：子任务拆分更直接；子会话通过 task 工具创建 parentID 子会话，结果以 tool 输出回传父会话，并插入一条 synthetic user message 引导总结（源码：packages/opencode/src/tool/task.ts、packages/opencode/src/session/prompt.ts）。
3) **AGENTS.md + instructions + 优先级 + CLAUDE.md/skills 兼容**：上下文装配可复用且迁移成本低。证据：OpenCode Rules + Skills。
4) **compaction(auto)+prune + compaction hook**：长会话压缩可控且可注入关键状态。证据：OpenCode Config + Plugins（compaction hook）。
5) **Server/TUI 分离 + OpenAPI + SDK + ACP**：同一 runtime 可被 TUI/IDE/自动化客户端复用。证据：OpenCode Server + SDK + ACP。
6) **SSE 事件流 + CLI JSON 事件 + plugin 事件钩子**：可观测性与可追踪性更强。证据：OpenCode Server（/global/event SSE）、CLI（--format json）、Plugins（事件列表）。
7) **权限系统细粒度规则 + tool.execute.before hooks**：可在运行时阻断敏感读写。证据：OpenCode Permissions + Plugins（tool.execute.before）。
8) **MCP 支持且提示“上下文成本”**：外部工具组合更透明。证据：OpenCode MCP servers。
9) **grep/glob/list 基于 ripgrep 且尊重 .gitignore**：检索与选择更可预测。证据：OpenCode Tools。
10) **Claude Code 对照：Subagent 独立上下文 + Hooks（PreToolUse/PreCompact 等）**：OpenCode 的“会话导航 + 事件流”更偏平台化；Claude Code 更偏 hook 驱动（推断：需更多官方对照文档）。证据：Claude Code Subagents + Hooks 官方文档。

---

## B. OpenCode Runtime 架构图（ASCII）

+-------------------- Input Layer ---------------------+
| TUI | CLI | IDE | ACP | Web/Server API Clients       |
+------------------------+-----------------------------+
                         |
                 [OpenCode Server / OpenAPI]
                         |
        +----------------v----------------+
        | Orchestrator                    |
        | - Agent Router                  |
        | - Session Manager               |
        | - Context Builder               |
        | - Permission Gate               |
        | - Tool Runner                   |
        +----------------+----------------+
                         |
                      [LLM]
                         |
        +----------------v----------------+
        | Tool Layer                       |
        | fs(read/edit/write/patch)        |
        | bash | grep/glob/list            |
        | webfetch | MCP | custom tools    |
        +----------------+----------------+
                         |
        +----------------v----------------+
        | Event/Stream Layer              |
        | SSE / JSON events / plugin hooks|
        +----------------+----------------+
                         |
        +----------------v----------------+
        | Storage                         |
        | sessions/logs/summaries         |
        | rules/skills/config             |
        | share/export                    |
        +---------------------------------+

图中组件来自 OpenCode 的 Server/SDK/CLI/Agents/Tools/Rules/Plugins/MCP/ACP 文档描述与事件机制。（见 Sources）

---

## C. 关键状态机/序列图（文字）

### 普通 Build 流程
1) 用户输入来自 TUI/CLI/IDE/ACP；TUI/CLI 通过本地 server 工作，server 发布 OpenAPI。（OpenCode Server + ACP）
2) Context builder 读取 AGENTS.md/CLAUDE.md 等规则并按优先级合并 instructions；也可通过 CLI --file 附带文件。（OpenCode Rules + CLI）
3) Agent router 选择 Build（默认或用户指定）；Build 默认具备完整工具访问。（OpenCode Agents）
4) LLM 产出工具调用；权限系统 allow/ask/deny 决策后执行，插件可在 tool.execute.before 拦截。（OpenCode Permissions + Plugins）
5) 工具输出回写会话；prune=true 时删除旧工具输出以节省 token。（OpenCode Config）
6) 会话更新并产生事件；SSE/SDK/CLI JSON 事件与插件事件对外可订阅。（OpenCode Server + SDK + CLI + Plugins）
7) 可选：/share 或 export/import 进行会话分享与迁移。（OpenCode Share + CLI）

### Plan → Build 切换流程
1) 切换到 Plan 主代理；Plan 默认对 file edits 与 bash 设置为 ask。（OpenCode Agents）
2) 触发 edit/write/patch 等工具时会执行 PermissionNext 规则：匹配 deny 直接拒绝，匹配 ask 触发询问；Plan 默认对 edit 为 deny（仅允许 plan 文件），bash 未默认 deny（源码：packages/opencode/src/agent/agent.ts、packages/opencode/src/permission/next.ts、packages/opencode/src/tool/*）。
3) 用户切换到 Build 或调整权限为 allow 后，进入 Build 流程执行。（OpenCode Agents + Permissions）

### Subtask/子会话流程
1) 主代理自动或通过 @ 调用 subagent；subagent 产生子会话并可在父/子会话间导航。（OpenCode Agents）
2) General 具备全工具并用于并行任务；Explore 只读用于快速探索。（OpenCode Agents）
3) 子会话结果以 TaskTool 输出（summary + text + metadata）回传父会话，并在父会话插入 synthetic user message 提示继续（源码：packages/opencode/src/tool/task.ts、packages/opencode/src/session/prompt.ts、packages/opencode/src/session/message-v2.ts）。

### Compaction + Prune 流程
1) 上下文满且 compaction.auto=true 时触发 compaction；prune=true 时删除旧工具输出。（OpenCode Config）
2) compaction hook 在生成摘要前可注入上下文或替换 compaction prompt。（OpenCode Plugins）
3) compaction 生成 summary assistant message 并触发 session.compacted；后续构建上下文时通过 MessageV2.filterCompacted 在 compaction 边界截断历史（源码：packages/opencode/src/session/compaction.ts、packages/opencode/src/session/message-v2.ts）。

---

## D. 独特设计优点清单（≥7）

1) **Plan/Build + allow/ask/deny** → 先规划再执行更可控 → 适合高风险改动 → 代价：审批次数上升 → 指标：edit/bash 被拦截次数、审批耗时。（OpenCode Agents + Permissions）
2) **子会话 + 并行 General/只读 Explore** → 任务拆分与探索更高效 → 适合多模块排查 → 代价：结果以 tool 输出/摘要回灌，需要二次整合 → 指标：子会话数量、主会话 token 占比。（源码：packages/opencode/src/tool/task.ts、packages/opencode/src/session/prompt.ts）
3) **AGENTS.md + instructions + 优先级 + CLAUDE 兼容** → 团队规则可复用且迁移顺滑 → 适合多仓库协作 → 代价：规则过多占上下文 → 指标：指令文件数、规则 token 占比。（OpenCode Rules + Skills）
4) **compaction(auto)+prune + compaction hook** → 长会话可控压缩 → 适合长时调试 → 代价：摘要失真风险 → 指标：compaction 后任务续航成功率、摘要修订次数。（OpenCode Config + Plugins）
5) **SSE/SDK/CLI JSON 事件 + 插件事件** → 可观测与自动化能力强 → 适合审计/CI → 代价：事件噪声与运维成本 → 指标：事件覆盖率、误报率。（OpenCode Server + SDK + CLI + Plugins）
6) **Server/OpenAPI + ACP** → 统一 runtime 复用到多客户端 → 适合 IDE/远程运行 → 代价：服务暴露需安全配置 → 指标：接入时间、客户端数量。（OpenCode Server + ACP）
7) **MCP 支持且显式提示上下文成本** → 工具组合更透明 → 适合接入外部系统 → 代价：context 膨胀 → 指标：MCP 工具 token 占比、超限次数。（OpenCode MCP servers）
8) **ripgrep 驱动的 grep/glob/list** → 搜索一致性高、遵循 .gitignore → 代价：需要额外 .ignore 来覆盖 → 指标：搜索命中率、误漏率。（OpenCode Tools）

---

## E. 与 Claude Code 的“框架层差异矩阵”

| 维度 | OpenCode | Claude Code | 对用户体验影响 | 证据/依据 | 推断/待验证 |
| --- | --- | --- | --- | --- | --- |
| Agent 编排 | Primary+Subagent（Build/Plan/General/Explore），可并行与父/子会话导航 | Subagent 独立 context window，可自动委派 | 两者都有分工；OpenCode 提供父/子会话导航 | OpenCode Agents；Claude Subagents | Claude 内建导航/父子关系未见官方说明（推断） |
| 会话隔离 | 子会话与导航；支持 share/export/import | Subagent 独立 context window | OpenCode“可导航子会话”更可操作；Claude 偏“隐式隔离” | OpenCode Agents + Share + CLI；Claude Subagents | Claude 会话导航与合并策略待验证（推断） |
| 权限系统 | allow/ask/deny + 细粒度规则；Plan 默认 deny edit（仅允许 plan 文件） | Claude Code hooks 包含 PermissionRequest；SDK 有权限规则 | OpenCode 更偏工具粒度策略；Claude 偏 hooks/SDK 策略 | OpenCode Permissions + Agents；Claude Hooks + Agent SDK Permissions | Claude Code UI/权限模式细节待验证（推断） |
| 上下文装配 | AGENTS.md + instructions + 优先级 + CLAUDE 兼容 | .claude/agents + subagent prompt | OpenCode 对规则来源更显式 | OpenCode Rules + Skills；Claude Subagents | Claude “上下文装配”细节待验证（推断） |
| compaction/prune | compaction.auto/prune；compaction hook；session.compacted 事件 | Hooks 提供 PreCompact（可插入） | OpenCode 提供 hook + prune；Claude 更偏 hooks | OpenCode Config + Plugins；Claude Hooks | Claude compaction 策略细节待验证（推断） |
| 流式/事件 | SSE /global/event + SDK event.subscribe + CLI JSON + plugin events | Hooks 事件覆盖 PreToolUse/PreCompact/SessionStart 等 | OpenCode 更适合外部可观测/可回放 | OpenCode Server + SDK + CLI + Plugins；Claude Hooks | Claude SSE/SDK 事件流未见官方说明（推断） |
| 扩展性 | MCP + 自定义工具 + plugins + SDK + ACP | MCP + hooks + subagents | OpenCode 更偏“平台化接口”；Claude 更偏“内置生态/配置驱动” | OpenCode MCP + Plugins + SDK + ACP；Claude Subagents + Hooks | 需更多官方对照细节（推断） |

---

## 第三方/用户报告对照（仅作参考）

- 用户报告：Claude Code 子代理可能会自动 compact（非官方，需谨慎）。此类信息只用作趋势观察，不作为设计依据。（用户报告）

---

## Sources（官方文档/说明）

OpenCode 官方：
- OpenCode Agents
- OpenCode Permissions
- OpenCode Rules
- OpenCode Skills
- OpenCode Config（compaction/prune）
- OpenCode Plugins（事件与 compaction hook）
- OpenCode Tools（ripgrep/.gitignore）
- OpenCode Server（OpenAPI + SSE）
- OpenCode SDK（events）
- OpenCode CLI（--format json, export/import）
- OpenCode MCP servers（context cost）
- OpenCode ACP Support
- OpenCode Share

Claude Code 官方：
- Claude Code Subagents
- Claude Code Hooks（PreToolUse/PreCompact/SessionStart 等）
- Claude Agent SDK Permissions（用作对照）

第三方/用户报告：
- Claude Code subagent auto-compaction 讨论（用户报告）

---

## F. Verified by Code / External TBD

### Verified by OpenCode source (this repo)
- Plan 权限默认 **deny edit**（仅允许 plan 文件），不是 ask。来源：`packages/opencode/src/agent/agent.ts`
- 权限执行时序：`PermissionNext.ask` 基于规则直接 **deny / ask / allow**。来源：`packages/opencode/src/permission/next.ts`
- 子会话由 `task` 工具创建（带 parentID），结果以 tool 输出回传父会话，并插入 synthetic user message 引导总结。来源：`packages/opencode/src/tool/task.ts` + `packages/opencode/src/session/prompt.ts`
- Compaction 通过 `MessageV2.filterCompacted` 在构造上下文时 **截断历史**。来源：`packages/opencode/src/session/compaction.ts` + `packages/opencode/src/session/message-v2.ts`
- AGENTS/CLAUDE/instructions 装配路径与优先级。来源：`packages/opencode/src/session/system.ts`
- grep/glob/list 基于 ripgrep。来源：`packages/opencode/src/tool/grep.ts` + `glob.ts` + `ls.ts` + `file/ripgrep.ts`
- SSE 事件流 `/event` 与 CLI JSON 事件 `--format json`。来源：`packages/opencode/src/server/server.ts` + `packages/opencode/src/cli/cmd/run.ts`

### External TBD (needs official docs / external verification)
- Claude Code 会话导航/父子关系机制（官方未明确）
- Claude Code 权限 UI/模式与 SDK 权限规则映射细节
- Claude Code compaction 内部策略与替换逻辑
- Claude Code 是否提供 SSE/SDK 事件流
- “MCP 上下文成本提示”是否为官方文档明确说明

---

## G. OpenCode → A 框架映射表（核心抽象）

| OpenCode 概念 | 源码位置 | A 框架建议抽象 |
| --- | --- | --- |
| SessionProcessor (流式状态机) | `packages/opencode/src/session/processor.ts` | `LoopNode` / `LoopEngine` |
| Message/Part 事件 | `packages/opencode/src/session/message-v2.ts` | `Event` 枚举 + `EventSink` |
| PermissionNext | `packages/opencode/src/permission/next.ts` | `PermissionGate` |
| Tool 生命周期 | `packages/opencode/src/tool/*` | `ToolRegistry` + `ToolState` |
| Subtask / TaskTool | `packages/opencode/src/tool/task.ts` | `SubtaskNode` / `SubgraphNode` |
| Compaction | `packages/opencode/src/session/compaction.ts` | `CompactionPolicy` |
| SSE / CLI JSON events | `packages/opencode/src/server/server.ts` + `cli/cmd/run.ts` | `EventStream` |

---

## H. MVP 落地路线图（面向 A 框架）

### Phase 1 (MVP-0)
- 事件协议：定义 `Event` + `EventSink`
- 图执行器：新增 `stream_events`（节点可持续输出事件）
- 输出：能从一个节点持续输出 text/tool 事件

### Phase 2 (MVP-1)
- 引入 `LoopNode`：实现 OpenCode 式流式状态机
- 对接工具生命周期：pending/running/completed/error
- 输出：单节点可实现 OpenCode 样式的对话流

### Phase 3 (MVP-2)
- 权限模型：allow/ask/deny + 规则合并
- 子任务：TaskTool → SubtaskNode / 子图执行
- 输出：主/子会话分离 + 结果回灌

### Phase 4 (MVP-3)
- compaction + prune 基础实现
- graph-level trace / replay
- 输出：长会话稳定性 + 可观测性

---

## I. Rust API 草图（建议起步接口）

```rust
// 事件协议
pub enum Event {
    TextDelta { session_id: String, message_id: String, delta: String },
    ToolStart { tool: String, call_id: String, input: serde_json::Value },
    ToolResult { tool: String, call_id: String, output: String },
    ToolError { tool: String, call_id: String, error: String },
    StepStart { session_id: String },
    StepFinish { session_id: String, tokens: TokenUsage, cost: f64 },
    PermissionAsked { permission: String, patterns: Vec<String> },
    PermissionReplied { permission: String, reply: PermissionReply },
}

pub trait EventSink: Send + Sync {
    fn emit(&self, event: Event);
}

// LoopNode：OpenCode SessionProcessor 的等价物
pub struct LoopNode {
    // model, tools, permissions, etc.
}

impl LoopNode {
    pub async fn run(&self, state: AgentState, sink: &dyn EventSink) -> Result<AgentState, Error> {
        // streaming loop
        Ok(state)
    }
}

// 扩展执行器，允许节点流式输出事件
pub trait StreamNode<S> {
    fn name(&self) -> &str;
    fn execute_stream(&self, state: S, sink: &dyn EventSink) -> BoxFuture<'_, GraphResult<S>>;
}
```
