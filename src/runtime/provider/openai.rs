//! OpenAI chat model adapter for Forge component interfaces.

use std::time::Duration;

use crate::runtime::component::{ChatModel, ChatRequest, ChatResponse};
use crate::runtime::error::{GraphError, GraphResult};
use crate::runtime::message::{Message, MessageRole, Part};
use crate::runtime::node::BoxFuture;

const OPENAI_DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

/// Configuration for OpenAI chat completions API.
#[derive(Clone, Debug)]
pub struct OpenAiChatModelConfig {
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: String,
    pub organization: Option<String>,
    pub project: Option<String>,
    pub timeout_ms: u64,
}

impl OpenAiChatModelConfig {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            api_key: None,
            base_url: OPENAI_DEFAULT_BASE_URL.to_string(),
            organization: None,
            project: None,
            timeout_ms: 30_000,
        }
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_organization(mut self, organization: impl Into<String>) -> Self {
        self.organization = Some(organization.into());
        self
    }

    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// OpenAI chat completions adapter implementing the ChatModel interface.
#[derive(Clone, Debug)]
pub struct OpenAiChatModel {
    model: String,
    api_key: String,
    base_url: String,
    organization: Option<String>,
    project: Option<String>,
    timeout_ms: u64,
}

impl OpenAiChatModel {
    pub fn new(config: OpenAiChatModelConfig) -> GraphResult<Self> {
        let api_key = resolve_api_key(config.api_key)?;
        if config.model.trim().is_empty() {
            return Err(openai_error("model is required"));
        }
        Ok(Self {
            model: config.model,
            api_key,
            base_url: config.base_url,
            organization: config.organization,
            project: config.project,
            timeout_ms: config.timeout_ms.max(1),
        })
    }

    fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    fn request_payload(&self, request: &ChatRequest) -> serde_json::Value {
        build_request_payload(&self.model, request)
    }
}

impl ChatModel for OpenAiChatModel {
    fn model_id(&self) -> &str {
        &self.model
    }

    fn generate(&self, request: ChatRequest) -> BoxFuture<'_, GraphResult<ChatResponse>> {
        let endpoint = self.endpoint();
        let payload = self.request_payload(&request);
        let api_key = self.api_key.clone();
        let organization = self.organization.clone();
        let project = self.project.clone();
        let timeout_ms = self.timeout_ms;
        Box::pin(async move {
            let agent = ureq::AgentBuilder::new()
                .timeout(Duration::from_millis(timeout_ms))
                .build();

            let mut request_builder = agent
                .post(&endpoint)
                .set("Authorization", &format!("Bearer {}", api_key))
                .set("Content-Type", "application/json");
            if let Some(org) = organization.as_deref() {
                request_builder = request_builder.set("OpenAI-Organization", org);
            }
            if let Some(proj) = project.as_deref() {
                request_builder = request_builder.set("OpenAI-Project", proj);
            }

            let response = request_builder.send_json(payload);
            let response_json = match response {
                Ok(resp) => resp
                    .into_json::<serde_json::Value>()
                    .map_err(|err| openai_error(format!("decode response failed: {}", err)))?,
                Err(ureq::Error::Status(status, resp)) => {
                    let body = resp.into_string().unwrap_or_default();
                    let detail = parse_error_message(&body).unwrap_or(body);
                    return Err(openai_error(format!(
                        "openai request failed with status {}: {}",
                        status, detail
                    )));
                }
                Err(err) => {
                    return Err(openai_error(format!("openai request failed: {}", err)));
                }
            };

            parse_chat_response(response_json)
        })
    }
}

fn resolve_api_key(explicit: Option<String>) -> GraphResult<String> {
    if let Some(key) = explicit {
        if !key.trim().is_empty() {
            return Ok(key);
        }
    }
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        if !key.trim().is_empty() {
            return Ok(key);
        }
    }
    Err(openai_error(
        "missing OPENAI_API_KEY (provide config.api_key or environment variable)",
    ))
}

fn openai_error(message: impl Into<String>) -> GraphError {
    GraphError::ExecutionError {
        node: "provider:openai".to_string(),
        message: message.into(),
    }
}

fn build_request_payload(model: &str, request: &ChatRequest) -> serde_json::Value {
    let messages = request
        .messages
        .iter()
        .map(|message| {
            serde_json::json!({
                "role": openai_role(&message.role),
                "content": render_message_content(message),
            })
        })
        .collect::<Vec<_>>();

    let mut payload = serde_json::json!({
        "model": model,
        "messages": messages,
    });
    if let Some(temperature) = request.temperature {
        payload["temperature"] = serde_json::json!(temperature);
    }
    if let Some(max_tokens) = request.max_output_tokens {
        payload["max_tokens"] = serde_json::json!(max_tokens);
    }
    payload
}

fn openai_role(role: &MessageRole) -> &'static str {
    match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    }
}

fn render_message_content(message: &Message) -> String {
    let mut chunks = Vec::new();
    for part in &message.parts {
        match part {
            Part::TextDelta { delta } => chunks.push(delta.clone()),
            Part::TextFinal { text } => chunks.push(text.clone()),
            Part::ToolResult { output, .. } => chunks.push(output.content.to_string()),
            Part::ToolError { error, .. } => chunks.push(error.clone()),
            Part::Attachment { data, .. } => chunks.push(data.to_string()),
            Part::Error { message } => chunks.push(message.clone()),
            _ => {}
        }
    }
    if chunks.is_empty() && !message.metadata.is_null() {
        chunks.push(message.metadata.to_string());
    }
    chunks.join("\n")
}

fn parse_chat_response(value: serde_json::Value) -> GraphResult<ChatResponse> {
    let choice = value
        .get("choices")
        .and_then(|choices| choices.as_array())
        .and_then(|choices| choices.first())
        .ok_or_else(|| openai_error("missing choices in response"))?;
    let raw_message = choice
        .get("message")
        .ok_or_else(|| openai_error("missing message in first choice"))?;

    let content = extract_message_content(raw_message);
    let mut message = Message::new(MessageRole::Assistant);
    if !content.is_empty() {
        message.parts.push(Part::TextFinal { text: content });
    }
    message.metadata = serde_json::json!({
        "provider": "openai",
        "raw": raw_message,
    });
    if let Some(created) = value.get("created").and_then(|v| v.as_u64()) {
        message.created_at_ms = Some(created.saturating_mul(1000));
    }

    let mut response = ChatResponse::new(message);
    response.model = value
        .get("model")
        .and_then(|model| model.as_str())
        .map(|model| model.to_string());
    response.finish_reason = choice
        .get("finish_reason")
        .and_then(|finish_reason| finish_reason.as_str())
        .map(|finish_reason| finish_reason.to_string());
    response.usage = parse_usage(&value);
    if let Some(id) = value.get("id").cloned() {
        response.metadata.insert("id".to_string(), id);
    }
    Ok(response)
}

fn parse_usage(value: &serde_json::Value) -> Option<crate::runtime::event::TokenUsage> {
    let usage = value.get("usage")?;
    Some(crate::runtime::event::TokenUsage {
        input: usage
            .get("prompt_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        output: usage
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        reasoning: usage
            .get("completion_tokens_details")
            .and_then(|v| v.get("reasoning_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cache_read: usage
            .get("prompt_tokens_details")
            .and_then(|v| v.get("cached_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cache_write: 0,
    })
}

fn extract_message_content(raw_message: &serde_json::Value) -> String {
    if let Some(content) = raw_message.get("content").and_then(|v| v.as_str()) {
        return content.to_string();
    }
    if let Some(parts) = raw_message.get("content").and_then(|v| v.as_array()) {
        let mut chunks = Vec::new();
        for part in parts {
            if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                chunks.push(text.to_string());
                continue;
            }
            if let Some(text) = part
                .get("text")
                .and_then(|text| text.get("value"))
                .and_then(|value| value.as_str())
            {
                chunks.push(text.to_string());
            }
        }
        return chunks.join("");
    }
    String::new()
}

fn parse_error_message(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(|message| message.as_str())
        .map(|message| message.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        build_request_payload, extract_message_content, parse_chat_response, parse_error_message,
        OpenAiChatModel, OpenAiChatModelConfig,
    };
    use crate::runtime::component::{ChatModel, ChatRequest};
    use crate::runtime::event::TokenUsage;
    use crate::runtime::message::{Message, MessageRole, Part};
    use futures::executor::block_on;

    #[test]
    fn request_payload_maps_messages_and_generation_options() {
        let mut message = Message::new(MessageRole::User);
        message.parts.push(Part::TextFinal {
            text: "hello".to_string(),
        });
        let request = ChatRequest::new("s1", "m1", vec![message])
            .with_temperature(0.2)
            .with_max_output_tokens(32);

        let payload = build_request_payload("gpt-4o-mini", &request);
        assert_eq!(payload["model"], "gpt-4o-mini");
        assert_eq!(payload["messages"][0]["role"], "user");
        assert_eq!(payload["messages"][0]["content"], "hello");
        let temperature = payload["temperature"]
            .as_f64()
            .expect("temperature should be numeric");
        assert!((temperature - 0.2).abs() < 1e-6);
        assert_eq!(payload["max_tokens"], 32);
    }

    #[test]
    fn parse_chat_response_supports_string_content() {
        let response = serde_json::json!({
            "id": "chatcmpl-1",
            "model": "gpt-4o-mini",
            "created": 123,
            "choices": [{
                "message": {"role": "assistant", "content": "hello"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "completion_tokens_details": {"reasoning_tokens": 2},
                "prompt_tokens_details": {"cached_tokens": 3}
            }
        });

        let parsed = parse_chat_response(response).expect("parse");
        assert_eq!(parsed.model.as_deref(), Some("gpt-4o-mini"));
        assert_eq!(parsed.finish_reason.as_deref(), Some("stop"));
        assert_eq!(parsed.text().as_deref(), Some("hello"));
        assert_eq!(
            parsed.usage,
            Some(TokenUsage {
                input: 10,
                output: 5,
                reasoning: 2,
                cache_read: 3,
                cache_write: 0,
            })
        );
    }

    #[test]
    fn extract_message_content_supports_array_shape() {
        let message = serde_json::json!({
            "content": [
                {"type": "text", "text": "hello"},
                {"type": "text", "text": " world"}
            ]
        });
        assert_eq!(extract_message_content(&message), "hello world");
    }

    #[test]
    fn parse_error_message_extracts_openai_error_text() {
        let body = r#"{"error":{"message":"quota exceeded"}}"#;
        assert_eq!(parse_error_message(body).as_deref(), Some("quota exceeded"));
    }

    #[test]
    fn openai_model_uses_configured_api_key() {
        let model = OpenAiChatModel::new(
            OpenAiChatModelConfig::new("gpt-4o-mini").with_api_key("test-key"),
        )
        .expect("construct");
        assert_eq!(model.model_id(), "gpt-4o-mini");
    }

    #[test]
    #[ignore = "requires OPENAI_API_KEY and external network"]
    fn openai_generate_smoke_test() {
        let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
        let model =
            OpenAiChatModel::new(OpenAiChatModelConfig::new("gpt-4o-mini").with_api_key(api_key))
                .expect("construct");
        let mut message = Message::new(MessageRole::User);
        message.parts.push(Part::TextFinal {
            text: "Reply with one word: pong".to_string(),
        });
        let request = ChatRequest::new("s1", "m1", vec![message]).with_max_output_tokens(8);

        let response = block_on(model.generate(request)).expect("generate");
        assert!(response.text().is_some());
    }
}
