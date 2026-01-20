//! Compaction policy and result types.

/// Compaction trigger policy.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompactionPolicy {
    pub max_messages: usize,
    pub enabled: bool,
}

impl CompactionPolicy {
    pub fn new(max_messages: usize) -> Self {
        Self {
            max_messages,
            enabled: true,
        }
    }

    pub fn should_compact(&self, message_count: usize) -> bool {
        self.enabled && message_count > self.max_messages
    }
}

/// Compaction output summary and truncation boundary.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CompactionResult {
    pub summary: String,
    pub truncated_before: usize,
}

impl CompactionResult {
    pub fn new(summary: impl Into<String>, truncated_before: usize) -> Self {
        Self {
            summary: summary.into(),
            truncated_before,
        }
    }
}

/// Context passed to compaction hooks.
#[derive(Clone, Debug)]
pub struct CompactionContext {
    pub messages: Vec<String>,
    pub prompt_hint: Option<String>,
}

impl CompactionContext {
    pub fn new(messages: Vec<String>) -> Self {
        Self {
            messages,
            prompt_hint: None,
        }
    }

    pub fn with_prompt_hint(mut self, hint: impl Into<String>) -> Self {
        self.prompt_hint = Some(hint.into());
        self
    }
}

/// Hook to customize compaction behavior (pre/post).
pub trait CompactionHook: Send + Sync + std::fmt::Debug {
    fn before_compaction(&self, _context: &CompactionContext) -> Option<String> {
        None
    }

    fn after_compaction(&self, _result: &CompactionResult) {}
}

/// Default no-op compaction hook.
#[derive(Debug)]
pub struct NoopCompactionHook;

impl CompactionHook for NoopCompactionHook {}

#[cfg(test)]
mod tests {
    use super::{
        CompactionContext,
        CompactionHook,
        CompactionPolicy,
        CompactionResult,
        NoopCompactionHook,
    };

    #[test]
    fn compaction_policy_threshold() {
        let policy = CompactionPolicy::new(3);
        assert!(!policy.should_compact(3));
        assert!(policy.should_compact(4));
    }

    #[test]
    fn compaction_result_holds_summary() {
        let result = CompactionResult::new("summary", 10);
        assert_eq!(result.summary, "summary");
        assert_eq!(result.truncated_before, 10);
    }

    #[test]
    fn compaction_hook_defaults() {
        let hook = NoopCompactionHook;
        let context = CompactionContext::new(vec!["m1".to_string()]);
        assert_eq!(hook.before_compaction(&context), None);
        hook.after_compaction(&CompactionResult::new("summary", 1));
    }

    #[test]
    fn compaction_context_holds_prompt_hint() {
        let context = CompactionContext::new(vec!["m1".to_string()])
            .with_prompt_hint("focus");
        assert_eq!(context.prompt_hint.as_deref(), Some("focus"));
    }
}
