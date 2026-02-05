//! Forge Runtime
//!
//! A Rust runtime for building stateful, event-driven agent applications\.
//!
//! ## Features
//!
//! - **State Graph**: Define nodes and edges for agent workflows
//! - **Conditional Routing**: Dynamic routing based on state
//! - **Interrupt/Resume**: Human-in-the-loop support
//! - **Metrics Collection**: Track latency, tokens, and success rates
//! - **Ablation Studies**: Analyze node contributions by masking
//! - **Evaluators**: Assess output quality with built-in or custom evaluators
//!
//! # Example
//! ```rust,no_run
//! use forge::runtime::constants::START;
//! use forge::runtime::prelude::{GraphError, StateGraph, END};
//! use forge::runtime::state::GraphState;
//!
//! #[derive(Clone, Default)]
//! struct MyState {
//!     messages: Vec<String>,
//!     next: Option<String>,
//! }
//!
//! impl GraphState for MyState {
//!     fn get_next(&self) -> Option<&str> { self.next.as_deref() }
//!     fn set_next(&mut self, next: Option<String>) { self.next = next; }
//! }
//!
//! async fn node_a(state: MyState) -> Result<MyState, GraphError> {
//!     let mut state = state;
//!     state.messages.push("Hello from A".to_string());
//!     Ok(state)
//! }
//!
//! # async fn run() -> Result<(), GraphError> {
//! let mut graph = StateGraph::<MyState>::new();
//! graph.add_node("a", node_a);
//! graph.add_edge(START, "a");
//! graph.add_edge("a", END);
//!
//! let compiled = graph.compile()?;
//! let _result = compiled.invoke(MyState::default()).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Ablation Study Example
//! ```rust,no_run
//! use forge::runtime::ablation::{AblationConfig, AblationReport};
//! use forge::runtime::metrics::MetricsCollector;
//!
//! // Create ablation configs
//! let configs = vec![
//!     AblationConfig::baseline("baseline"),
//!     AblationConfig::mask("without_planner", vec!["planner"]),
//!     AblationConfig::mask("without_researcher", vec!["researcher"]),
//! ];
//!
//! // Run study and generate report
//! let collector = MetricsCollector::new();
//! // ... run tests with different configs ...
//! let report = AblationReport::from_metrics(&collector, &configs);
//! println!("{}", report.to_markdown());
//! ```

// Core modules
pub mod constants;
pub mod error;
pub mod cancel;
pub mod state;
pub mod node;
pub mod branch;
pub mod graph;
pub mod executor;
pub mod channel;
pub mod event;
pub mod message;
pub mod component;
pub mod compaction;
pub mod permission;
pub mod prune;
pub mod tool;
pub mod trace;
pub mod output;
pub mod platform;
pub mod provider;
pub mod session;
pub mod session_state;
pub mod r#loop;
pub mod toolkit;

// Evaluation modules
pub mod metrics;
pub mod evaluator;
pub mod ablation;

/// Prelude - commonly used types
pub mod prelude {
    // Core types
    pub use crate::runtime::constants::END;
    pub use crate::runtime::error::GraphError;
    pub use crate::runtime::cancel::CancellationToken;
    
    
    
    pub use crate::runtime::graph::StateGraph;
    pub use crate::runtime::executor::CompiledGraph;
pub use crate::runtime::event::{
    Event,
    EventMeta,
    EventRecord,
    EventRecordSink,
    EventSequencer,
    EventSink,
    NoopEventRecordSink,
    NoopEventSink,
    PermissionReply,
    ToolUpdate,
    TokenUsage,
};
    pub use crate::runtime::message::{Message, MessageRole, Part};
    pub use crate::runtime::component::{
        register_retriever_tool,
        ChatModel,
        ChatRequest,
        ChatResponse,
        EmbeddingModel,
        HashEmbeddingModel,
        InMemoryRetriever,
        MockChatModel,
        RetrievedDocument,
        Retriever,
    };
    pub use crate::runtime::compaction::{CompactionPolicy, CompactionResult};
    pub use crate::runtime::prune::{PrunePolicy, PruneResult};
    pub use crate::runtime::trace::{ExecutionTrace, TraceEvent, TraceReplay, TraceSpan};
    pub use crate::runtime::output::{
        JsonLineEventRecordSink,
        JsonLineEventSink,
        SseEventRecordSink,
        SseEventSink,
    };
    pub use crate::runtime::platform::{
        PlatformOutputFormat,
        PlatformStreamMode,
        stream_to_writer,
        stream_cli_jsonl_events,
        stream_cli_jsonl_records,
        stream_sse_events,
        stream_sse_records,
    };
    pub use crate::runtime::provider::openai::{
        OpenAiChatModel,
        OpenAiChatModelConfig,
    };
    pub use crate::runtime::session::{
        AttachmentResolver,
        SessionMessage,
        SessionSnapshot,
        SessionSnapshotIo,
    };
    pub use crate::runtime::session_state::{
        RunMetadata,
        RunStatus,
        SessionPhase,
        SessionRouting,
        SessionState,
        ToolCallRecord,
        ToolCallStatus,
    };
    pub use crate::runtime::permission::{
        PermissionDecision,
        PermissionGate,
        PermissionPolicy,
        PermissionRequest,
        PermissionRule,
        PermissionSession,
        PermissionSnapshot,
        PermissionStore,
        InMemoryPermissionStore,
    };
    pub use crate::runtime::tool::{
        ToolCall,
        ToolDefinition,
        ToolMetadata,
        ToolOutput,
        ToolRegistry,
        ToolRunner,
        ToolSchemaRegistry,
        ToolState,
    };
    pub use crate::runtime::r#loop::{LoopContext, LoopNode};
    pub use crate::runtime::builtin_tool_registry;

    // Metrics and evaluation
    
    
    
}

pub fn builtin_tool_registry(root: impl Into<std::path::PathBuf>) -> tool::ToolRegistry {
    let mut registry = tool::ToolRegistry::new();
    toolkit::file_tools::register_file_tools(&mut registry, root);
    registry
}
