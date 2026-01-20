//! Tool lifecycle types for streaming execution.

/// Tool lifecycle states for execution tracking.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolState {
    Pending,
    Running,
    Completed,
    Error,
}

#[cfg(test)]
mod tests {
    use super::ToolState;

    #[test]
    fn tool_state_equality() {
        assert_eq!(ToolState::Pending, ToolState::Pending);
        assert_ne!(ToolState::Pending, ToolState::Running);
    }
}
