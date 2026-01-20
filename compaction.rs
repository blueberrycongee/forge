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
#[derive(Clone, Debug, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::{CompactionPolicy, CompactionResult};

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
}
