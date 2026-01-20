//! Structured message + part model for runtime sessions.

use serde::{Deserialize, Serialize};

use crate::runtime::event::{Event, TokenUsage};
use crate::runtime::tool::ToolOutput;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub parts: Vec<Part>,
    pub metadata: serde_json::Value,
    pub created_at_ms: Option<u64>,
}

impl Message {
    pub fn new(role: MessageRole) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role,
            parts: Vec::new(),
            metadata: serde_json::json!({}),
            created_at_ms: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Part {
    TextDelta { delta: String },
    TextFinal { text: String },
    ToolCall {
        tool: String,
        call_id: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool: String,
        call_id: String,
        output: ToolOutput,
    },
    ToolError {
        tool: String,
        call_id: String,
        error: String,
    },
    Attachment {
        name: String,
        mime_type: String,
        data: serde_json::Value,
    },
    TokenUsage { usage: TokenUsage },
    Error { message: String },
}

impl Part {
    pub fn from_event(event: &Event) -> Option<Self> {
        match event {
            Event::TextDelta { delta, .. } => Some(Part::TextDelta {
                delta: delta.clone(),
            }),
            Event::ToolStart {
                tool,
                call_id,
                input,
            } => Some(Part::ToolCall {
                tool: tool.clone(),
                call_id: call_id.clone(),
                input: input.clone(),
            }),
            Event::ToolResult {
                tool,
                call_id,
                output,
            } => Some(Part::ToolResult {
                tool: tool.clone(),
                call_id: call_id.clone(),
                output: output.clone(),
            }),
            Event::ToolError {
                tool,
                call_id,
                error,
            } => Some(Part::ToolError {
                tool: tool.clone(),
                call_id: call_id.clone(),
                error: error.clone(),
            }),
            Event::StepFinish { tokens, .. } => Some(Part::TokenUsage { usage: tokens.clone() }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Message, MessageRole, Part};
    use crate::runtime::event::{Event, TokenUsage};
    use crate::runtime::tool::ToolOutput;

    #[test]
    fn message_new_initializes_empty_parts_and_metadata() {
        let message = Message::new(MessageRole::User);
        assert!(!message.id.is_empty());
        assert!(message.parts.is_empty());
        assert_eq!(message.metadata, serde_json::json!({}));
    }

    #[test]
    fn part_from_event_maps_text_delta() {
        let event = Event::TextDelta {
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            delta: "hi".to_string(),
        };

        assert_eq!(
            Part::from_event(&event),
            Some(Part::TextDelta {
                delta: "hi".to_string()
            })
        );
    }

    #[test]
    fn part_from_event_maps_tool_result() {
        let output = ToolOutput::text("ok");
        let event = Event::ToolResult {
            tool: "grep".to_string(),
            call_id: "c1".to_string(),
            output: output.clone(),
        };

        assert_eq!(
            Part::from_event(&event),
            Some(Part::ToolResult {
                tool: "grep".to_string(),
                call_id: "c1".to_string(),
                output,
            })
        );
    }

    #[test]
    fn part_from_event_maps_tool_error() {
        let event = Event::ToolError {
            tool: "rg".to_string(),
            call_id: "c2".to_string(),
            error: "boom".to_string(),
        };

        assert_eq!(
            Part::from_event(&event),
            Some(Part::ToolError {
                tool: "rg".to_string(),
                call_id: "c2".to_string(),
                error: "boom".to_string(),
            })
        );
    }

    #[test]
    fn part_from_event_maps_token_usage() {
        let event = Event::StepFinish {
            session_id: "s2".to_string(),
            tokens: TokenUsage {
                input: 1,
                output: 2,
                reasoning: 3,
                cache_read: 4,
                cache_write: 5,
            },
            cost: 0.01,
        };

        assert_eq!(
            Part::from_event(&event),
            Some(Part::TokenUsage {
                usage: TokenUsage {
                    input: 1,
                    output: 2,
                    reasoning: 3,
                    cache_read: 4,
                    cache_write: 5,
                }
            })
        );
    }

    #[test]
    fn part_from_event_ignores_unrelated_events() {
        let event = Event::PermissionAsked {
            permission: "fs.read".to_string(),
            patterns: vec!["*".to_string()],
        };

        assert_eq!(Part::from_event(&event), None);
    }
}
