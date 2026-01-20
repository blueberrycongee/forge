//! Graph executor - runs the compiled graph
//!
//! Supports:
//! - Interrupt/resume for human-in-the-loop workflows
//! - Node masking for ablation studies
//! - Metrics collection for performance analysis

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use serde::{Serialize, Deserialize};

use crate::langgraph::constants::{START, END, MAX_ITERATIONS};
use crate::langgraph::error::{GraphError, GraphResult, Interrupt, ResumeCommand};
use crate::langgraph::state::GraphState;
use crate::langgraph::graph::{StateGraph, Edge};
use crate::langgraph::event::{Event, EventSink};
use crate::langgraph::compaction::{
    CompactionContext,
    CompactionHook,
    CompactionPolicy,
    NoopCompactionHook,
    CompactionResult,
};
use crate::langgraph::prune::{PrunePolicy, prune_tool_events};
use crate::langgraph::trace::{ExecutionTrace, TraceEvent};
use crate::langgraph::session::{SessionSnapshot, SessionMessage};
use crate::langgraph::node::{Node, NodeSpec};
use crate::langgraph::branch::{Branch, BranchSpec};
use crate::langgraph::metrics::{MetricsCollector, RunMetrics, RunMetricsBuilder};
use crate::langgraph::ablation::NodeOverride;

/// Configuration for graph execution
#[derive(Clone, Debug)]
pub struct ExecutionConfig {
    /// Maximum number of iterations
    pub max_iterations: usize,
    /// Enable debug logging
    pub debug: bool,
    /// Recursion limit
    pub recursion_limit: usize,
    /// Nodes to skip (for ablation studies)
    pub masked_nodes: HashSet<String>,
    /// Node overrides (replace behavior)
    pub node_overrides: HashMap<String, NodeOverride>,
    /// Configuration ID for metrics grouping
    pub config_id: String,
    /// Enable metrics collection
    pub collect_metrics: bool,
    /// Compaction hook for streaming execution
    pub compaction_hook: Arc<dyn CompactionHook>,
    /// Compaction policy for auto-triggering
    pub compaction_policy: CompactionPolicy,
    /// Prune policy for event history
    pub prune_policy: PrunePolicy,
    /// Optional event history buffer
    pub event_history: Option<Arc<std::sync::Mutex<Vec<Event>>>>,
    /// Optional trace collector
    pub trace: Option<Arc<std::sync::Mutex<ExecutionTrace>>>,
    /// Optional session snapshot collector
    pub session_snapshot: Option<Arc<std::sync::Mutex<SessionSnapshot>>>,
}

impl ExecutionConfig {
    pub fn new() -> Self {
        Self {
            max_iterations: MAX_ITERATIONS,
            debug: false,
            recursion_limit: 25,
            masked_nodes: HashSet::new(),
            node_overrides: HashMap::new(),
            config_id: "default".to_string(),
            collect_metrics: false,
            compaction_hook: Arc::new(NoopCompactionHook),
            compaction_policy: CompactionPolicy::default(),
            prune_policy: PrunePolicy::default(),
            event_history: None,
            trace: None,
            session_snapshot: None,
        }
    }

    /// Create config for ablation study
    pub fn for_ablation(config_id: impl Into<String>, masked: HashSet<String>) -> Self {
        Self {
            max_iterations: MAX_ITERATIONS,
            debug: false,
            recursion_limit: 25,
            masked_nodes: masked,
            node_overrides: HashMap::new(),
            config_id: config_id.into(),
            collect_metrics: true,
            compaction_hook: Arc::new(NoopCompactionHook),
            compaction_policy: CompactionPolicy::default(),
            prune_policy: PrunePolicy::default(),
            event_history: None,
            trace: None,
            session_snapshot: None,
        }
    }

    /// Add a masked node
    pub fn mask_node(mut self, node: impl Into<String>) -> Self {
        self.masked_nodes.insert(node.into());
        self
    }

    /// Add multiple masked nodes
    pub fn mask_nodes(mut self, nodes: Vec<&str>) -> Self {
        for node in nodes {
            self.masked_nodes.insert(node.to_string());
        }
        self
    }

    /// Set config ID
    pub fn with_config_id(mut self, id: impl Into<String>) -> Self {
        self.config_id = id.into();
        self
    }

    /// Enable metrics collection
    pub fn with_metrics(mut self) -> Self {
        self.collect_metrics = true;
        self
    }

    /// Set compaction hook for streaming execution
    pub fn with_compaction_hook(mut self, hook: Arc<dyn CompactionHook>) -> Self {
        self.compaction_hook = hook;
        self
    }

    /// Set compaction policy
    pub fn with_compaction_policy(mut self, policy: CompactionPolicy) -> Self {
        self.compaction_policy = policy;
        self
    }

    /// Set prune policy for event history
    pub fn with_prune_policy(mut self, policy: PrunePolicy) -> Self {
        self.prune_policy = policy;
        self
    }

    /// Attach an event history buffer for stream_events
    pub fn with_event_history(mut self, history: Arc<std::sync::Mutex<Vec<Event>>>) -> Self {
        self.event_history = Some(history);
        self
    }

    /// Attach trace collector for node events
    pub fn with_trace(mut self, trace: Arc<std::sync::Mutex<ExecutionTrace>>) -> Self {
        self.trace = Some(trace);
        self
    }

    /// Attach session snapshot collector
    pub fn with_session_snapshot(
        mut self,
        snapshot: Arc<std::sync::Mutex<SessionSnapshot>>,
    ) -> Self {
        self.session_snapshot = Some(snapshot);
        self
    }

    /// Check if a node is masked
    pub fn is_masked(&self, node: &str) -> bool {
        self.masked_nodes.contains(node)
    }
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Checkpoint - saves execution state at interrupt
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Checkpoint<S> {
    /// Current state
    pub state: S,
    /// Next node to execute
    pub next_node: String,
    /// Pending interrupts
    pub pending_interrupts: Vec<Interrupt>,
    /// Completed iterations
    pub iterations: usize,
    /// Resume values (from user input)
    #[serde(default)]
    pub resume_values: HashMap<String, serde_json::Value>,
}

/// Execution result - may complete or be interrupted
#[derive(Debug)]
pub enum ExecutionResult<S> {
    /// Execution completed successfully
    Complete(S),
    /// Execution interrupted, needs human input
    Interrupted {
        checkpoint: Checkpoint<S>,
        interrupts: Vec<Interrupt>,
    },
}

/// Result of execution with metrics
#[derive(Debug)]
pub struct ExecutionResultWithMetrics<S> {
    /// The execution result
    pub result: ExecutionResult<S>,
    /// Collected metrics (if enabled)
    pub metrics: Option<RunMetrics>,
}

/// A compiled graph ready for execution
pub struct CompiledGraph<S: GraphState> {
    /// Node definitions
    pub(crate) nodes: HashMap<String, NodeSpec<S>>,
    /// Edge definitions
    pub(crate) edges: HashMap<String, Vec<Edge>>,
    /// Branch definitions
    pub(crate) branches: HashMap<String, BranchSpec<S>>,
    /// Execution configuration
    config: ExecutionConfig,
    /// Metrics collector (shared across runs)
    metrics_collector: Option<Arc<MetricsCollector>>,
}

impl<S: GraphState> CompiledGraph<S> {
    /// Create from a StateGraph
    pub(crate) fn new(graph: StateGraph<S>) -> Self {
        Self {
            nodes: graph.nodes,
            edges: graph.edges,
            branches: graph.branches,
            config: ExecutionConfig::new(),
            metrics_collector: None,
        }
    }

    /// Set execution configuration
    pub fn with_config(mut self, config: ExecutionConfig) -> Self {
        self.config = config;
        self
    }

    /// Set max iterations
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.config.max_iterations = max;
        self
    }

    /// Enable debug mode
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.config.debug = debug;
        self
    }

    /// Set metrics collector for accumulating results
    pub fn with_metrics_collector(mut self, collector: Arc<MetricsCollector>) -> Self {
        self.metrics_collector = Some(collector);
        self.config.collect_metrics = true;
        self
    }

    /// Execute the graph with the given initial state
    pub async fn invoke(&self, initial_state: S) -> GraphResult<S> {
        let result = self.invoke_with_metrics(initial_state).await?;
        match result.result {
            ExecutionResult::Complete(state) => Ok(state),
            ExecutionResult::Interrupted { .. } => {
                Err(GraphError::Other("Unexpected interrupt".to_string()))
            }
        }
    }

    /// Execute and return metrics
    pub async fn invoke_with_metrics(&self, initial_state: S) -> GraphResult<ExecutionResultWithMetrics<S>> {
        let run_id = uuid::Uuid::new_v4().to_string();
        let mut metrics_builder = if self.config.collect_metrics {
            Some(RunMetricsBuilder::new(&run_id, &self.config.config_id))
        } else {
            None
        };

        let mut state = initial_state;
        let mut current_node = self.get_next_node(START, &state)?;
        let mut iterations = 0;

        while current_node != END && iterations < self.config.max_iterations {
            iterations += 1;

            if self.config.debug {
                println!("[LangGraph] Executing node: {}", current_node);
            }

            // Check if node is masked
            if self.config.is_masked(&current_node) {
                if self.config.debug {
                    println!("[LangGraph] Skipping masked node: {}", current_node);
                }
                if let Some(ref mut mb) = metrics_builder {
                    mb.skip_node(&current_node);
                }
                // Skip to next node without executing
                current_node = self.get_next_node(&current_node, &state)?;
                continue;
            }

            // Start timing
            if let Some(ref mut mb) = metrics_builder {
                mb.start_node(&current_node);
            }

            // Execute the node
            let node = self.nodes.get(&current_node)
                .ok_or_else(|| GraphError::NodeNotFound(current_node.clone()))?;

            match node.execute(state).await {
                Ok(new_state) => {
                    state = new_state;
                    // Record metrics (tokens would come from state if available)
                    if let Some(ref mut mb) = metrics_builder {
                        mb.end_node(0); // TODO: get tokens from state
                    }
                }
                Err(e) => {
                    if let Some(ref mut mb) = metrics_builder {
                        mb.error(&current_node, &e.to_string());
                    }
                    return Err(e);
                }
            }

            // Determine next node
            current_node = self.get_next_node(&current_node, &state)?;
        }

        if iterations >= self.config.max_iterations {
            return Err(GraphError::MaxIterationsExceeded);
        }

        // Finalize metrics
        let metrics = metrics_builder.map(|mb| {
            let m = mb.build(true);
            // Add to collector if present
            if let Some(ref collector) = self.metrics_collector {
                collector.add_run(m.clone());
            }
            m
        });

        Ok(ExecutionResultWithMetrics {
            result: ExecutionResult::Complete(state),
            metrics,
        })
    }

    /// Execute with streaming - yields state after each node
    pub async fn stream<F>(&self, initial_state: S, mut callback: F) -> GraphResult<S>
    where
        F: FnMut(&str, &S),
    {
        let mut state = initial_state;
        let mut current_node = self.get_next_node(START, &state)?;
        let mut iterations = 0;

        while current_node != END && iterations < self.config.max_iterations {
            iterations += 1;

            // Check if masked
            if self.config.is_masked(&current_node) {
                current_node = self.get_next_node(&current_node, &state)?;
                continue;
            }

            // Execute the node
            let node = self.nodes.get(&current_node)
                .ok_or_else(|| GraphError::NodeNotFound(current_node.clone()))?;

            state = node.execute(state).await?;

            // Callback with current state
            callback(&current_node, &state);

            // Determine next node
            current_node = self.get_next_node(&current_node, &state)?;
        }

        if iterations >= self.config.max_iterations {
            return Err(GraphError::MaxIterationsExceeded);
        }

        Ok(state)
    }

    /// Execute with runtime event streaming.
    /// Nodes that define a stream function will emit events via the sink.
    pub async fn stream_events(&self, initial_state: S, sink: std::sync::Arc<dyn EventSink>) -> GraphResult<S> {
        let mut state = initial_state;
        let mut current_node = self.get_next_node(START, &state)?;
        let mut iterations = 0;
        let use_history = self.config.prune_policy.enabled || self.config.event_history.is_some();
        let history = self
            .config
            .event_history
            .clone()
            .unwrap_or_else(|| Arc::new(std::sync::Mutex::new(Vec::new())));
        let trace = self.config.trace.clone();
        let snapshot = self.config.session_snapshot.clone();
        let sink: Arc<dyn EventSink> = if use_history {
            Arc::new(RecordingSink::new(sink, Arc::clone(&history)))
        } else {
            sink
        };

        while current_node != END && iterations < self.config.max_iterations {
            iterations += 1;

            // Check if masked
            if self.config.is_masked(&current_node) {
                current_node = self.get_next_node(&current_node, &state)?;
                continue;
            }

            let node = self
                .nodes
                .get(&current_node)
                .ok_or_else(|| GraphError::NodeNotFound(current_node.clone()))?;

            if let Some(trace) = &trace {
                trace
                    .lock()
                    .unwrap()
                    .record_event(TraceEvent::NodeStart {
                        node: current_node.clone(),
                    });
            }
            state = node.execute_stream(state, sink.clone()).await?;
            if let Some(snapshot) = &snapshot {
                snapshot.lock().unwrap().messages.push(SessionMessage {
                    role: "system".to_string(),
                    content: format!("node:{}:executed", current_node),
                });
            }
            if let Some(trace) = &trace {
                trace
                    .lock()
                    .unwrap()
                    .record_event(TraceEvent::NodeFinish {
                        node: current_node.clone(),
                    });
            }

            let message_count = resolve_message_count(&snapshot, &history);
            if self.config.compaction_policy.should_compact(message_count) {
                let messages = collect_compaction_messages(&snapshot);
                let context = CompactionContext::new(messages);
                if let Some(summary) = self.config.compaction_hook.before_compaction(&context) {
                    let result = CompactionResult::new(summary, 0);
                    self.config.compaction_hook.after_compaction(&result);
                    let session_id = resolve_session_id(&state);
                    if let Some(trace) = &trace {
                        trace.lock().unwrap().record_event(TraceEvent::Compacted {
                            summary: result.summary.clone(),
                            truncated_before: result.truncated_before,
                        });
                    }
                    if let Some(snapshot) = &snapshot {
                        snapshot.lock().unwrap().compactions.push(result.clone());
                    }
                    sink.emit(crate::langgraph::event::Event::SessionCompacted {
                        session_id,
                        summary: result.summary,
                        truncated_before: result.truncated_before,
                    });
                }
            }

            if use_history && self.config.prune_policy.enabled {
                let mut events = history.lock().unwrap();
                prune_tool_events(&mut events, &self.config.prune_policy);
            }

            current_node = self.get_next_node(&current_node, &state)?;
        }

        if iterations >= self.config.max_iterations {
            return Err(GraphError::MaxIterationsExceeded);
        }

        Ok(state)
    }

    /// Get the next node to execute
    fn get_next_node(&self, current: &str, state: &S) -> GraphResult<String> {
        // Check if state has explicit next
        if let Some(next) = state.get_next() {
            return Ok(next.to_string());
        }

        // Check edges
        let edges = self.edges.get(current);

        match edges {
            None => Ok(END.to_string()),
            Some(edges) if edges.is_empty() => Ok(END.to_string()),
            Some(edges) => {
                match &edges[0] {
                    Edge::Direct(to) => Ok(to.clone()),
                    Edge::Conditional(branch_name) => {
                        let branch = self.branches.get(branch_name)
                            .ok_or_else(|| GraphError::BranchError {
                                node: current.to_string(),
                                message: format!("Branch '{}' not found", branch_name),
                            })?;

                        let result = branch.evaluate(state)?;
                        branch.resolve(&result)
                    }
                }
            }
        }
    }

    /// Get all node names
    pub fn get_nodes(&self) -> Vec<&str> {
        self.nodes.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a node exists
    pub fn has_node(&self, name: &str) -> bool {
        self.nodes.contains_key(name)
    }

    /// Execute graph with interrupt/resume support
    pub async fn invoke_resumable(&self, initial_state: S) -> GraphResult<ExecutionResult<S>> {
        self.run_with_checkpoint(initial_state, START.to_string(), 0, HashMap::new()).await
    }

    /// Resume from checkpoint
    pub async fn resume(&self, checkpoint: Checkpoint<S>, command: ResumeCommand) -> GraphResult<ExecutionResult<S>> {
        let mut resume_values = checkpoint.resume_values;

        // Add new resume value
        if let Some(interrupt_id) = command.interrupt_id {
            resume_values.insert(interrupt_id, command.value);
        } else if let Some(interrupt) = checkpoint.pending_interrupts.first() {
            resume_values.insert(interrupt.id.clone(), command.value);
        }

        self.run_with_checkpoint(
            checkpoint.state,
            checkpoint.next_node,
            checkpoint.iterations,
            resume_values,
        ).await
    }

    /// Internal execution with checkpoint support
    async fn run_with_checkpoint(
        &self,
        initial_state: S,
        start_node: String,
        start_iterations: usize,
        resume_values: HashMap<String, serde_json::Value>,
    ) -> GraphResult<ExecutionResult<S>> {
        let mut state = initial_state;
        let mut current_node = if start_node == START {
            self.get_next_node(START, &state)?
        } else {
            start_node
        };
        let mut iterations = start_iterations;

        while current_node != END && iterations < self.config.max_iterations {
            iterations += 1;

            if self.config.debug {
                println!("[LangGraph] Executing node: {} (iteration {})", current_node, iterations);
            }

            // Check if masked
            if self.config.is_masked(&current_node) {
                if self.config.debug {
                    println!("[LangGraph] Skipping masked node: {}", current_node);
                }
                current_node = self.get_next_node(&current_node, &state)?;
                continue;
            }

            // Check if we have a resume value for this node
            let has_resume = resume_values.contains_key(&current_node);

            // Execute the node
            let node = self.nodes.get(&current_node)
                .ok_or_else(|| GraphError::NodeNotFound(current_node.clone()))?;

            match node.execute(state.clone()).await {
                Ok(new_state) => {
                    state = new_state;
                }
                Err(GraphError::Interrupted(interrupts)) => {
                    if has_resume {
                        if self.config.debug {
                            println!("[LangGraph] Resuming from interrupt at node: {}", current_node);
                        }
                    } else {
                        // No resume value, return interrupted state
                        return Ok(ExecutionResult::Interrupted {
                            checkpoint: Checkpoint {
                                state,
                                next_node: current_node,
                                pending_interrupts: interrupts.clone(),
                                iterations,
                                resume_values,
                            },
                            interrupts,
                        });
                    }
                }
                Err(e) => return Err(e),
            }

            // Determine next node
            current_node = self.get_next_node(&current_node, &state)?;
        }

        if iterations >= self.config.max_iterations {
            return Err(GraphError::MaxIterationsExceeded);
        }

        Ok(ExecutionResult::Complete(state))
    }

    // ============ Ablation Study Methods ============

    /// Run ablation study with multiple configurations
    pub async fn run_ablation<F>(
        &self,
        test_inputs: Vec<S>,
        configs: Vec<ExecutionConfig>,
        _state_factory: F,
    ) -> Vec<(String, RunMetrics)>
    where
        F: FnMut() -> S,
        S: Clone,
    {
        let collector = Arc::new(MetricsCollector::new());
        let mut results = Vec::new();

        for config in configs {
            let config_id = config.config_id.clone();
            
            for input in &test_inputs {
                // Create a new graph with this config
                let graph = CompiledGraph {
                    nodes: self.nodes.clone(),
                    edges: self.edges.clone(),
                    branches: self.branches.clone(),
                    config: config.clone(),
                    metrics_collector: Some(collector.clone()),
                };

                // Run and collect metrics
                let _ = graph.invoke_with_metrics(input.clone()).await;
            }

            // Get aggregated metrics for this config
            let runs = collector.get_runs_by_config(&config_id);
            for run in runs {
                results.push((config_id.clone(), run));
            }
        }

        results
    }

    /// Get current config
    pub fn config(&self) -> &ExecutionConfig {
        &self.config
    }

    /// Get metrics collector
    pub fn metrics_collector(&self) -> Option<&Arc<MetricsCollector>> {
        self.metrics_collector.as_ref()
    }

    /// Build a session snapshot from current config state.
    pub fn build_snapshot(&self, session_id: impl Into<String>) -> SessionSnapshot {
        let trace = self
            .config
            .trace
            .as_ref()
            .map(|trace| trace.lock().unwrap().clone())
            .unwrap_or_else(ExecutionTrace::new);
        let messages = self
            .config
            .session_snapshot
            .as_ref()
            .map(|snapshot| snapshot.lock().unwrap().messages.clone())
            .unwrap_or_default();
        let compactions = self
            .config
            .session_snapshot
            .as_ref()
            .map(|snapshot| snapshot.lock().unwrap().compactions.clone())
            .unwrap_or_default();
        SessionSnapshot {
            session_id: session_id.into(),
            messages,
            trace,
            compactions,
        }
    }
}

struct RecordingSink {
    inner: Arc<dyn EventSink>,
    history: Arc<std::sync::Mutex<Vec<Event>>>,
}

fn resolve_message_count(
    snapshot: &Option<Arc<std::sync::Mutex<SessionSnapshot>>>,
    history: &Arc<std::sync::Mutex<Vec<Event>>>,
) -> usize {
    if let Some(snapshot) = snapshot {
        return snapshot.lock().unwrap().messages.len();
    }
    history.lock().unwrap().len()
}

fn collect_compaction_messages(
    snapshot: &Option<Arc<std::sync::Mutex<SessionSnapshot>>>,
) -> Vec<String> {
    snapshot
        .as_ref()
        .map(|snapshot| {
            snapshot
                .lock()
                .unwrap()
                .messages
                .iter()
                .map(|message| message.content.clone())
                .collect()
        })
        .unwrap_or_default()
}

impl RecordingSink {
    fn new(inner: Arc<dyn EventSink>, history: Arc<std::sync::Mutex<Vec<Event>>>) -> Self {
        Self { inner, history }
    }
}

impl EventSink for RecordingSink {
    fn emit(&self, event: Event) {
        self.history.lock().unwrap().push(event.clone());
        self.inner.emit(event);
    }
}

// Need to implement Clone for CompiledGraph to support ablation studies
impl<S: GraphState> Clone for CompiledGraph<S> {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            branches: self.branches.clone(),
            config: self.config.clone(),
            metrics_collector: self.metrics_collector.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::langgraph::constants::START;
    use crate::langgraph::event::{Event, EventSink};
    use crate::langgraph::graph::StateGraph;
    use crate::langgraph::prune::PrunePolicy;
    use crate::langgraph::state::GraphState;
    use std::sync::{Arc, Mutex};
    use futures::executor::block_on;

    #[test]
    fn test_execution_config() {
        let config = ExecutionConfig::new()
            .mask_node("planner")
            .mask_node("researcher")
            .with_config_id("test_config")
            .with_metrics();

        assert!(config.is_masked("planner"));
        assert!(config.is_masked("researcher"));
        assert!(!config.is_masked("executor"));
        assert_eq!(config.config_id, "test_config");
        assert!(config.collect_metrics);
    }

    #[test]
    fn test_ablation_config() {
        let masked: HashSet<String> = vec!["planner".to_string()].into_iter().collect();
        let config = ExecutionConfig::for_ablation("no_planner", masked);

        assert!(config.is_masked("planner"));
        assert!(config.collect_metrics);
    }

    #[derive(Clone, Default)]
    struct StreamState {
        steps: Vec<String>,
    }

    impl GraphState for StreamState {}

    struct CaptureSink {
        events: Arc<Mutex<Vec<Event>>>,
    }

    impl EventSink for CaptureSink {
        fn emit(&self, event: Event) {
            self.events.lock().unwrap().push(event);
        }
    }

    #[test]
    fn stream_events_emits_from_stream_node() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink {
            events: events.clone(),
        });

        let mut graph = StateGraph::<StreamState>::new();
        graph.add_stream_node("streamer", |mut state, sink| async move {
            sink.emit(Event::TextDelta {
                session_id: "s1".to_string(),
                message_id: "m1".to_string(),
                delta: "hello".to_string(),
            });
            state.steps.push("stream".to_string());
            Ok(state)
        });
        graph.add_edge(START, "streamer");
        graph.add_edge("streamer", END);

        let compiled = graph.compile().expect("compile");
        let final_state = block_on(compiled.stream_events(StreamState::default(), sink)).expect("run");

        assert_eq!(final_state.steps, vec!["stream".to_string()]);
        assert_eq!(events.lock().unwrap().len(), 1);
    }

    #[derive(Debug)]
    struct TestCompactionHook {
        calls: Arc<Mutex<usize>>,
    }

    impl CompactionHook for TestCompactionHook {
        fn before_compaction(&self, _context: &CompactionContext) -> Option<String> {
            let mut calls = self.calls.lock().unwrap();
            if *calls == 0 {
                *calls += 1;
                Some("summary".to_string())
            } else {
                *calls += 1;
                None
            }
        }

        fn after_compaction(&self, _result: &CompactionResult) {
            let mut calls = self.calls.lock().unwrap();
            *calls += 1;
        }
    }

    #[test]
    fn stream_events_emits_compaction_event() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink {
            events: events.clone(),
        });
        let hook_calls = Arc::new(Mutex::new(0));
        let hook: Arc<dyn CompactionHook> = Arc::new(TestCompactionHook {
            calls: hook_calls.clone(),
        });
        let snapshot = Arc::new(Mutex::new(SessionSnapshot::new("s1")));

        let mut graph = StateGraph::<StreamState>::new();
        graph.add_stream_node("streamer", |state, _sink| async move { Ok(state) });
        graph.add_edge(START, "streamer");
        graph.add_edge("streamer", END);

        let compiled = graph
            .compile()
            .expect("compile")
            .with_config(
                ExecutionConfig::new()
                    .with_compaction_hook(hook)
                    .with_compaction_policy(CompactionPolicy::new(0))
                    .with_session_snapshot(Arc::clone(&snapshot)),
            );
        let _ = block_on(compiled.stream_events(StreamState::default(), sink)).expect("run");

        let captured = events.lock().unwrap();
        assert!(captured
            .iter()
            .any(|event| matches!(event, Event::SessionCompacted { .. })));
        assert_eq!(*hook_calls.lock().unwrap(), 2);
    }

    #[test]
    fn stream_events_prunes_event_history() {
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink {
            events: Arc::new(Mutex::new(Vec::new())),
        });
        let history = Arc::new(Mutex::new(Vec::new()));

        let mut graph = StateGraph::<StreamState>::new();
        graph.add_stream_node("first", |state, sink| async move {
            sink.emit(Event::ToolStart {
                tool: "grep".to_string(),
                call_id: "1".to_string(),
                input: serde_json::json!({"q": "hi"}),
            });
            sink.emit(Event::ToolResult {
                tool: "grep".to_string(),
                call_id: "1".to_string(),
                output: crate::langgraph::tool::ToolOutput::text("ok"),
            });
            Ok(state)
        });
        graph.add_stream_node("second", |state, sink| async move {
            sink.emit(Event::TextDelta {
                session_id: "s1".to_string(),
                message_id: "m1".to_string(),
                delta: "hello".to_string(),
            });
            sink.emit(Event::ToolStart {
                tool: "grep".to_string(),
                call_id: "2".to_string(),
                input: serde_json::json!({"q": "hi"}),
            });
            sink.emit(Event::ToolResult {
                tool: "grep".to_string(),
                call_id: "2".to_string(),
                output: crate::langgraph::tool::ToolOutput::text("ok"),
            });
            Ok(state)
        });
        graph.add_edge(START, "first");
        graph.add_edge("first", "second");
        graph.add_edge("second", END);

        let compiled = graph
            .compile()
            .expect("compile")
            .with_config(
                ExecutionConfig::new()
                    .with_event_history(Arc::clone(&history))
                    .with_prune_policy(PrunePolicy::new(2)),
            );

        let _ = block_on(compiled.stream_events(StreamState::default(), sink)).expect("run");

        let events = history.lock().unwrap();
        assert!(events.iter().any(|event| matches!(event, Event::TextDelta { .. })));
        assert!(!events.iter().any(|event| matches!(event, Event::ToolStart { call_id, .. } if call_id == "1")));
        assert!(events.iter().any(|event| matches!(event, Event::ToolStart { call_id, .. } if call_id == "2")));
    }

    #[test]
    fn stream_events_records_trace() {
        let trace = Arc::new(Mutex::new(ExecutionTrace::new()));
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink {
            events: Arc::new(Mutex::new(Vec::new())),
        });

        let mut graph = StateGraph::<StreamState>::new();
        graph.add_stream_node("node", |state, _sink| async move { Ok(state) });
        graph.add_edge(START, "node");
        graph.add_edge("node", END);

        let compiled = graph
            .compile()
            .expect("compile")
            .with_config(ExecutionConfig::new().with_trace(Arc::clone(&trace)));

        let _ = block_on(compiled.stream_events(StreamState::default(), sink)).expect("run");

        let trace = trace.lock().unwrap();
        assert!(trace.events.iter().any(|event| matches!(event, TraceEvent::NodeStart { .. })));
        assert!(trace.events.iter().any(|event| matches!(event, TraceEvent::NodeFinish { .. })));
    }

    #[test]
    fn stream_events_updates_session_snapshot() {
        let trace = Arc::new(Mutex::new(ExecutionTrace::new()));
        let snapshot = Arc::new(Mutex::new(SessionSnapshot::new("s1")));
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink {
            events: Arc::new(Mutex::new(Vec::new())),
        });

        let mut graph = StateGraph::<StreamState>::new();
        graph.add_stream_node("node", |state, _sink| async move { Ok(state) });
        graph.add_edge(START, "node");
        graph.add_edge("node", END);

        let compiled = graph
            .compile()
            .expect("compile")
            .with_config(
                ExecutionConfig::new()
                    .with_trace(Arc::clone(&trace))
                    .with_session_snapshot(Arc::clone(&snapshot)),
            );

        let _ = block_on(compiled.stream_events(StreamState::default(), sink)).expect("run");

        let snapshot = snapshot.lock().unwrap();
        assert!(!snapshot.messages.is_empty());
        assert!(snapshot.compactions.is_empty());
    }
}

fn resolve_session_id<S: GraphState>(state: &S) -> String {
    if let Some(value) = state.get("session_id") {
        if let Some(id) = value.downcast_ref::<String>() {
            return id.clone();
        }
    }
    "unknown".to_string()
}
