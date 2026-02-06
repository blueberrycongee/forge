//! State trait and utilities for Forge
//!
//! The state is the shared data structure that nodes read from and write to.

use std::any::Any;

use serde::{Deserialize, Serialize};

/// Trait for graph state
///
/// Implement this trait for your state type to use it with StateGraph.
/// The state should be cloneable and thread-safe.
///
/// # Example
/// ```rust,no_run
/// use forge::runtime::state::GraphState;
///
/// #[derive(Clone, Default)]
/// struct MyState {
///     counter: i32,
///     messages: Vec<String>,
/// }
///
/// impl GraphState for MyState {
///     // Optional: override if you need custom routing logic
/// }
/// ```
pub trait GraphState: Clone + Send + Sync + 'static {
    /// Get the next node to execute (optional, used for internal routing)
    fn get_next(&self) -> Option<&str> {
        None
    }

    /// Set the next node to execute (optional, used for internal routing)
    fn set_next(&mut self, _next: Option<String>) {}

    /// Check if the state indicates completion
    fn is_complete(&self) -> bool {
        false
    }

    /// Mark the state as complete
    fn mark_complete(&mut self) {}

    /// Get a value by key (for channel-based state)
    fn get(&self, _key: &str) -> Option<&dyn Any> {
        None
    }

    /// Set a value by key (for channel-based state)
    fn set(&mut self, _key: &str, _value: Box<dyn Any + Send + Sync>) {}
}

/// A simple state that stores values in a HashMap
///
/// Useful for prototyping or when you don't need a custom state type.
#[derive(Clone, Default)]
pub struct DictState {
    values: std::collections::HashMap<String, Box<dyn CloneableAny + Send + Sync>>,
    next: Option<String>,
    complete: bool,
}

/// Trait for cloneable Any
pub trait CloneableAny: Any + Send + Sync {
    fn clone_box(&self) -> Box<dyn CloneableAny + Send + Sync>;
    fn as_any(&self) -> &dyn Any;
}

impl<T: Clone + Send + Sync + 'static> CloneableAny for T {
    fn clone_box(&self) -> Box<dyn CloneableAny + Send + Sync> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Clone for Box<dyn CloneableAny + Send + Sync> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl DictState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_value<T: Clone + Send + Sync + 'static>(mut self, key: &str, value: T) -> Self {
        self.values.insert(key.to_string(), Box::new(value));
        self
    }

    pub fn get_value<T: Clone + 'static>(&self, key: &str) -> Option<&T> {
        self.values.get(key)?.as_any().downcast_ref::<T>()
    }

    pub fn set_value<T: Clone + Send + Sync + 'static>(&mut self, key: &str, value: T) {
        self.values.insert(key.to_string(), Box::new(value));
    }

    pub fn merge_from(&mut self, other: &DictState) {
        for (key, value) in &other.values {
            self.values.insert(key.clone(), value.clone());
        }
        if other.next.is_some() {
            self.next = other.next.clone();
        }
        if other.complete {
            self.complete = true;
        }
    }
}

impl GraphState for DictState {
    fn get_next(&self) -> Option<&str> {
        self.next.as_deref()
    }

    fn set_next(&mut self, next: Option<String>) {
        self.next = next;
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn mark_complete(&mut self) {
        self.complete = true;
    }
}

/// Shared state for multi-agent workflows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SharedState {
    pub data: serde_json::Map<String, serde_json::Value>,
    pub version: u64,
}

impl SharedState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_value(key: impl Into<String>, value: serde_json::Value) -> Self {
        let mut state = Self::new();
        state.insert(key, value);
        state
    }

    pub fn insert(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.data.insert(key.into(), value);
        self.version = self.version.saturating_add(1);
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    pub fn merge(&self, other: &SharedState) -> SharedState {
        let mut data = self.data.clone();
        for (key, value) in &other.data {
            data.insert(key.clone(), value.clone());
        }
        SharedState {
            data,
            version: self.version.max(other.version).saturating_add(1),
        }
    }
}

/// State update - represents partial updates to state
#[derive(Clone)]
pub struct StateUpdate<S: GraphState> {
    pub state: S,
    pub next: Option<String>,
}

impl<S: GraphState> StateUpdate<S> {
    pub fn new(state: S) -> Self {
        Self { state, next: None }
    }

    pub fn with_next(mut self, next: impl Into<String>) -> Self {
        self.next = Some(next.into());
        self
    }

    pub fn goto(mut self, node: impl Into<String>) -> Self {
        self.next = Some(node.into());
        self
    }
}

/// LoopState holds session/message metadata for streaming loop execution.
#[derive(Clone, Debug)]
pub struct LoopState {
    pub session_id: String,
    pub message_id: String,
    pub step: u64,
    next: Option<String>,
    complete: bool,
}

impl LoopState {
    pub fn new(session_id: impl Into<String>, message_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            message_id: message_id.into(),
            step: 0,
            next: None,
            complete: false,
        }
    }

    pub fn advance_step(&mut self) {
        self.step = self.step.saturating_add(1);
    }
}

impl GraphState for LoopState {
    fn get_next(&self) -> Option<&str> {
        self.next.as_deref()
    }

    fn set_next(&mut self, next: Option<String>) {
        self.next = next;
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn mark_complete(&mut self) {
        self.complete = true;
    }
}

#[cfg(test)]
mod tests {
    use super::{GraphState, LoopState, SharedState};

    #[test]
    fn loop_state_tracks_session_and_routing() {
        let mut state = LoopState::new("s1", "m1");

        assert_eq!(state.session_id, "s1");
        assert_eq!(state.message_id, "m1");
        assert_eq!(state.step, 0);
        assert_eq!(state.get_next(), None);
        assert!(!state.is_complete());

        state.set_next(Some("node-a".to_string()));
        assert_eq!(state.get_next(), Some("node-a"));

        state.advance_step();
        assert_eq!(state.step, 1);

        state.mark_complete();
        assert!(state.is_complete());
    }

    #[test]
    fn shared_state_merges_with_last_writer() {
        let mut base = SharedState::new();
        base.insert("plan", serde_json::json!("alpha"));
        let mut update = SharedState::new();
        update.insert("plan", serde_json::json!("beta"));
        update.insert("work", serde_json::json!("done"));

        let merged = base.merge(&update);

        assert_eq!(merged.get("plan"), Some(&serde_json::json!("beta")));
        assert_eq!(merged.get("work"), Some(&serde_json::json!("done")));
    }
}
