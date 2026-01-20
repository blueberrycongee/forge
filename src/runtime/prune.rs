//! Prune policy for trimming old tool events.

use crate::runtime::event::Event;

/// Prune policy for tool events.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrunePolicy {
    pub enabled: bool,
    pub keep_tool_events: usize,
}

impl PrunePolicy {
    pub fn new(keep_tool_events: usize) -> Self {
        Self {
            enabled: true,
            keep_tool_events,
        }
    }
}

/// Result of pruning.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PruneResult {
    pub pruned: usize,
}

/// Prune old tool events in-place, keeping the most recent N tool events.
pub fn prune_tool_events(events: &mut Vec<Event>, policy: &PrunePolicy) -> PruneResult {
    if !policy.enabled {
        return PruneResult { pruned: 0 };
    }

    let mut keep_tool_indices = std::collections::HashSet::new();
    let mut seen = 0usize;
    for (idx, event) in events.iter().enumerate().rev() {
        if is_tool_event(event) {
            seen += 1;
            if seen <= policy.keep_tool_events {
                keep_tool_indices.insert(idx);
            }
        }
    }

    let before = events.len();
    events.retain_with_index(|idx, event| {
        if !is_tool_event(event) {
            return true;
        }
        keep_tool_indices.contains(&idx)
    });
    PruneResult {
        pruned: before - events.len(),
    }
}

fn is_tool_event(event: &Event) -> bool {
    matches!(
        event,
        Event::ToolStart { .. }
            | Event::ToolResult { .. }
            | Event::ToolError { .. }
            | Event::ToolStatus { .. }
    )
}

trait RetainWithIndex<T> {
    fn retain_with_index<F>(&mut self, f: F)
    where
        F: FnMut(usize, &T) -> bool;
}

impl<T> RetainWithIndex<T> for Vec<T> {
    fn retain_with_index<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &T) -> bool,
    {
        let len = self.len();
        let mut del = 0;
        for i in 0..len {
            if !f(i, &self[i]) {
                del += 1;
            } else if del > 0 {
                self.swap(i - del, i);
            }
        }
        if del > 0 {
            self.truncate(len - del);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{prune_tool_events, PrunePolicy};
    use crate::runtime::event::{Event, TokenUsage};
    use crate::runtime::tool::ToolOutput;

    fn make_tool_start(call_id: &str) -> Event {
        Event::ToolStart {
            tool: "grep".to_string(),
            call_id: call_id.to_string(),
            input: serde_json::json!({"q": "hi"}),
        }
    }

    fn make_tool_result(call_id: &str) -> Event {
        Event::ToolResult {
            tool: "grep".to_string(),
            call_id: call_id.to_string(),
            output: ToolOutput::text("ok"),
        }
    }

    #[test]
    fn prune_keeps_recent_tool_events() {
        let mut events = vec![
            make_tool_start("1"),
            make_tool_result("1"),
            Event::TextDelta {
                session_id: "s1".to_string(),
                message_id: "m1".to_string(),
                delta: "hello".to_string(),
            },
            make_tool_start("2"),
            make_tool_result("2"),
            Event::StepFinish {
                session_id: "s1".to_string(),
                tokens: TokenUsage::default(),
                cost: 0.0,
            },
        ];

        let result = prune_tool_events(&mut events, &PrunePolicy::new(2));
        assert_eq!(result.pruned, 2);
        assert_eq!(events.len(), 4);
        assert!(events.iter().any(|event| matches!(event, Event::TextDelta { .. })));
        assert!(events.iter().any(|event| matches!(event, Event::StepFinish { .. })));
    }

    #[test]
    fn prune_disabled_keeps_all() {
        let mut events = vec![make_tool_start("1"), make_tool_result("1")];
        let policy = PrunePolicy {
            enabled: false,
            keep_tool_events: 0,
        };
        let result = prune_tool_events(&mut events, &policy);
        assert_eq!(result.pruned, 0);
        assert_eq!(events.len(), 2);
    }
}
