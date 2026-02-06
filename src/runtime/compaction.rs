//! Compaction policy and result types.

/// Compaction trigger policy.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CompactionPolicy {
    /// Trigger based on message count.
    pub max_messages: Option<usize>,
    /// Trigger based on absolute token count.
    pub max_tokens: Option<u64>,
    /// Trigger based on context window ratio (e.g. 0.95).
    pub token_ratio: Option<f64>,
    /// Context window size used with `token_ratio`.
    pub context_window: Option<u64>,
    /// Whether compaction is enabled.
    pub enabled: bool,
}

impl CompactionPolicy {
    pub fn new(max_messages: usize) -> Self {
        Self {
            max_messages: Some(max_messages),
            max_tokens: None,
            token_ratio: None,
            context_window: None,
            enabled: true,
        }
    }

    pub fn token_ratio(context_window: u64, ratio: f64) -> Self {
        Self {
            max_messages: None,
            max_tokens: None,
            token_ratio: Some(ratio),
            context_window: Some(context_window),
            enabled: true,
        }
    }

    pub fn with_context_window(mut self, context_window: u64) -> Self {
        self.context_window = Some(context_window);
        self
    }

    pub fn with_token_ratio(mut self, ratio: f64) -> Self {
        self.token_ratio = Some(ratio);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn should_compact(&self, message_count: usize) -> bool {
        self.enabled
            && self
                .max_messages
                .map(|max| message_count > max)
                .unwrap_or(false)
    }

    pub fn should_compact_with_usage(
        &self,
        message_count: usize,
        token_total: Option<u64>,
    ) -> bool {
        if !self.enabled {
            return false;
        }

        let message_trigger = self
            .max_messages
            .map(|max| message_count > max)
            .unwrap_or(false);

        let token_threshold = self.resolve_token_threshold();
        let token_trigger = match (token_threshold, token_total) {
            (Some(threshold), Some(total)) => total >= threshold,
            _ => false,
        };

        message_trigger || token_trigger
    }

    pub fn requires_token_usage(&self) -> bool {
        self.enabled
            && (self.max_tokens.is_some()
                || (self.token_ratio.is_some() && self.context_window.is_some()))
    }

    pub fn resolve_token_threshold(&self) -> Option<u64> {
        if let Some(max_tokens) = self.max_tokens {
            return Some(max_tokens);
        }
        let ratio = self.token_ratio?;
        let window = self.context_window?;
        if ratio <= 0.0 {
            return None;
        }
        Some(((window as f64) * ratio).floor() as u64)
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
        CompactionContext, CompactionHook, CompactionPolicy, CompactionResult, NoopCompactionHook,
    };

    #[test]
    fn compaction_policy_threshold() {
        let policy = CompactionPolicy::new(3);
        assert!(!policy.should_compact(3));
        assert!(policy.should_compact(4));
    }

    #[test]
    fn compaction_policy_token_ratio_threshold() {
        let policy = CompactionPolicy::token_ratio(100, 0.95);
        assert!(!policy.should_compact_with_usage(0, Some(94)));
        assert!(policy.should_compact_with_usage(0, Some(95)));
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
        let context = CompactionContext::new(vec!["m1".to_string()]).with_prompt_hint("focus");
        assert_eq!(context.prompt_hint.as_deref(), Some("focus"));
    }
}
