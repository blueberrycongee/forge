//! Standard component interfaces for model, embedding, and retrieval.

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::runtime::error::{GraphError, GraphResult};
use crate::runtime::event::{Event, EventSink, TokenUsage};
use crate::runtime::message::{Message, MessageRole, Part};
use crate::runtime::node::BoxFuture;
use crate::runtime::tool::{ToolCall, ToolDefinition, ToolOutput, ToolRegistry};

/// Request payload for chat model generation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatRequest {
    pub session_id: String,
    pub message_id: String,
    pub messages: Vec<Message>,
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u32>,
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl ChatRequest {
    pub fn new(
        session_id: impl Into<String>,
        message_id: impl Into<String>,
        messages: Vec<Message>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            message_id: message_id.into(),
            messages,
            temperature: None,
            max_output_tokens: None,
            metadata: serde_json::Map::new(),
        }
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_max_output_tokens(mut self, max_output_tokens: u32) -> Self {
        self.max_output_tokens = Some(max_output_tokens);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Response payload from chat model generation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: Message,
    pub usage: Option<TokenUsage>,
    pub model: Option<String>,
    pub finish_reason: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl ChatResponse {
    pub fn new(message: Message) -> Self {
        Self {
            message,
            usage: None,
            model: None,
            finish_reason: None,
            metadata: serde_json::Map::new(),
        }
    }

    pub fn with_usage(mut self, usage: TokenUsage) -> Self {
        self.usage = Some(usage);
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_finish_reason(mut self, finish_reason: impl Into<String>) -> Self {
        self.finish_reason = Some(finish_reason.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn text(&self) -> Option<String> {
        let mut text = String::new();
        for part in &self.message.parts {
            match part {
                Part::TextDelta { delta } => text.push_str(delta),
                Part::TextFinal { text: final_text } => text.push_str(final_text),
                _ => {}
            }
        }
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

/// Standard interface for chat-capable models.
pub trait ChatModel: Send + Sync {
    fn model_id(&self) -> &str;

    fn generate(&self, request: ChatRequest) -> BoxFuture<'_, GraphResult<ChatResponse>>;

    fn stream(
        &self,
        request: ChatRequest,
        sink: Arc<dyn EventSink>,
    ) -> BoxFuture<'_, GraphResult<ChatResponse>> {
        Box::pin(async move {
            let response = self.generate(request.clone()).await?;
            if let Some(text) = response.text() {
                sink.emit(Event::TextFinal {
                    session_id: request.session_id,
                    message_id: request.message_id,
                    text,
                })?;
            }
            Ok(response)
        })
    }
}

/// Deterministic mock chat model useful for local tests.
#[derive(Clone, Debug)]
pub struct MockChatModel {
    model_id: String,
    response_text: String,
}

impl MockChatModel {
    pub fn new(model_id: impl Into<String>, response_text: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            response_text: response_text.into(),
        }
    }
}

impl ChatModel for MockChatModel {
    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn generate(&self, request: ChatRequest) -> BoxFuture<'_, GraphResult<ChatResponse>> {
        let mut message = Message::new(MessageRole::Assistant);
        message.id = request.message_id;
        message.parts.push(Part::TextFinal {
            text: self.response_text.clone(),
        });
        let response = ChatResponse::new(message)
            .with_model(self.model_id.clone())
            .with_finish_reason("stop")
            .with_usage(TokenUsage {
                input: request.messages.len() as u64,
                output: 1,
                reasoning: 0,
                cache_read: 0,
                cache_write: 0,
            });
        Box::pin(async move { Ok(response) })
    }
}

/// Standard interface for embedding models.
pub trait EmbeddingModel: Send + Sync {
    fn model_id(&self) -> &str;

    fn embed(&self, inputs: Vec<String>) -> BoxFuture<'_, GraphResult<Vec<Vec<f32>>>>;
}

/// Simple hashing-based embedding model for tests/local pipelines.
#[derive(Clone, Debug)]
pub struct HashEmbeddingModel {
    model_id: String,
    dimensions: usize,
}

impl HashEmbeddingModel {
    pub fn new(model_id: impl Into<String>, dimensions: usize) -> Self {
        Self {
            model_id: model_id.into(),
            dimensions: dimensions.max(1),
        }
    }

    fn embed_text(text: &str, dimensions: usize) -> Vec<f32> {
        let mut vector = vec![0.0; dimensions];
        for token in tokenize(text) {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            token.hash(&mut hasher);
            let index = (hasher.finish() as usize) % dimensions;
            vector[index] += 1.0;
        }
        let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for value in &mut vector {
                *value /= norm;
            }
        }
        vector
    }
}

impl EmbeddingModel for HashEmbeddingModel {
    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn embed(&self, inputs: Vec<String>) -> BoxFuture<'_, GraphResult<Vec<Vec<f32>>>> {
        let dimensions = self.dimensions;
        Box::pin(async move {
            let vectors = inputs
                .into_iter()
                .map(|input| Self::embed_text(&input, dimensions))
                .collect();
            Ok(vectors)
        })
    }
}

/// Retrieved document from a retriever.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetrievedDocument {
    pub id: String,
    pub content: String,
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
    pub score: Option<f32>,
}

impl RetrievedDocument {
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            metadata: serde_json::Map::new(),
            score: None,
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn with_score(mut self, score: f32) -> Self {
        self.score = Some(score);
        self
    }
}

/// Standard interface for retrievers.
pub trait Retriever: Send + Sync {
    fn retriever_id(&self) -> &str;

    fn retrieve(
        &self,
        query: String,
        limit: usize,
    ) -> BoxFuture<'_, GraphResult<Vec<RetrievedDocument>>>;
}

/// In-memory retriever with lexical overlap ranking.
#[derive(Clone, Debug, Default)]
pub struct InMemoryRetriever {
    retriever_id: String,
    docs: Vec<RetrievedDocument>,
}

impl InMemoryRetriever {
    pub fn new(retriever_id: impl Into<String>) -> Self {
        Self {
            retriever_id: retriever_id.into(),
            docs: Vec::new(),
        }
    }

    pub fn from_documents(retriever_id: impl Into<String>, docs: Vec<RetrievedDocument>) -> Self {
        Self {
            retriever_id: retriever_id.into(),
            docs,
        }
    }

    pub fn add_document(&mut self, doc: RetrievedDocument) {
        self.docs.push(doc);
    }
}

impl Retriever for InMemoryRetriever {
    fn retriever_id(&self) -> &str {
        &self.retriever_id
    }

    fn retrieve(
        &self,
        query: String,
        limit: usize,
    ) -> BoxFuture<'_, GraphResult<Vec<RetrievedDocument>>> {
        let docs = self.docs.clone();
        Box::pin(async move {
            if limit == 0 {
                return Ok(Vec::new());
            }

            let query_terms = tokenize(&query);
            let mut ranked = Vec::new();

            for doc in docs {
                let doc_terms = tokenize(&doc.content);
                let overlap = query_terms.intersection(&doc_terms).count();
                if overlap == 0 {
                    continue;
                }
                let score = overlap as f32 / query_terms.len().max(1) as f32;
                ranked.push(doc.with_score(score));
            }

            ranked.sort_by(|a, b| {
                b.score
                    .unwrap_or(0.0)
                    .partial_cmp(&a.score.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.id.cmp(&b.id))
            });

            ranked.truncate(limit);
            Ok(ranked)
        })
    }
}

#[derive(Deserialize)]
struct RetrieverToolInput {
    query: String,
    #[serde(default = "default_retrieval_limit")]
    limit: usize,
}

fn default_retrieval_limit() -> usize {
    5
}

fn tokenize(text: &str) -> HashSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

/// Register a retriever as a runtime tool.
pub fn register_retriever_tool(
    registry: &mut ToolRegistry,
    tool_name: impl Into<String>,
    description: impl Into<String>,
    retriever: Arc<dyn Retriever>,
) {
    let tool_name = tool_name.into();
    let description = description.into();
    registry.register_with_definition(
        ToolDefinition::new(tool_name.clone(), description)
            .with_input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1}
                },
                "required": ["query"]
            }))
            .with_output_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "documents": {"type": "array"},
                    "count": {"type": "integer"}
                },
                "required": ["documents", "count"]
            })),
        Arc::new(move |call: ToolCall, _ctx| {
            let retriever = Arc::clone(&retriever);
            Box::pin(async move {
                let input: RetrieverToolInput =
                    serde_json::from_value(call.input).map_err(|err| {
                        GraphError::ExecutionError {
                            node: format!("tool:{}", call.tool),
                            message: format!("invalid retriever input: {}", err),
                        }
                    })?;
                let docs = retriever.retrieve(input.query, input.limit).await?;
                let payload = docs
                    .iter()
                    .map(|doc| {
                        serde_json::json!({
                            "id": doc.id,
                            "content": doc.content,
                            "metadata": doc.metadata,
                            "score": doc.score,
                        })
                    })
                    .collect::<Vec<_>>();
                Ok(ToolOutput::new(serde_json::json!({
                    "documents": payload,
                    "count": docs.len(),
                })))
            })
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::{
        register_retriever_tool, ChatModel, ChatRequest, EmbeddingModel, HashEmbeddingModel,
        InMemoryRetriever, MockChatModel, RetrievedDocument, Retriever,
    };
    use crate::runtime::constants::{END, START};
    use crate::runtime::event::{Event, EventSink, NoopEventSink};
    use crate::runtime::graph::StateGraph;
    use crate::runtime::permission::{PermissionPolicy, PermissionSession};
    use crate::runtime::state::GraphState;
    use crate::runtime::tool::{AttachmentPolicy, ToolCall, ToolContext, ToolRegistry};
    use futures::executor::block_on;
    use std::sync::{Arc, Mutex};

    #[test]
    fn mock_chat_model_generates_response_text() {
        let model = MockChatModel::new("mock", "hello");
        let request = ChatRequest::new("s1", "m1", Vec::new());

        let response = block_on(model.generate(request)).expect("generate");
        assert_eq!(response.model.as_deref(), Some("mock"));
        assert_eq!(response.text().as_deref(), Some("hello"));
    }

    struct CaptureSink {
        events: Arc<Mutex<Vec<Event>>>,
    }

    impl EventSink for CaptureSink {
        fn emit(&self, event: Event) -> crate::runtime::error::GraphResult<()> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
    }

    #[test]
    fn mock_chat_model_stream_emits_text_final() {
        let model = MockChatModel::new("mock", "streamed");
        let request = ChatRequest::new("s1", "m1", Vec::new());
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink: Arc<dyn EventSink> = Arc::new(CaptureSink {
            events: Arc::clone(&events),
        });

        let response = block_on(model.stream(request, sink)).expect("stream");
        assert_eq!(response.text().as_deref(), Some("streamed"));
        assert!(events.lock().unwrap().iter().any(|event| matches!(
            event,
            Event::TextFinal { text, .. } if text == "streamed"
        )));
    }

    #[test]
    fn hash_embedding_model_returns_stable_dimensions() {
        let model = HashEmbeddingModel::new("hash", 8);
        let embeddings = block_on(model.embed(vec!["alpha beta".to_string()])).expect("embed");
        assert_eq!(embeddings.len(), 1);
        assert_eq!(embeddings[0].len(), 8);
        assert!(embeddings[0].iter().any(|value| *value > 0.0));
    }

    #[test]
    fn in_memory_retriever_ranks_by_overlap() {
        let retriever = InMemoryRetriever::from_documents(
            "mem",
            vec![
                RetrievedDocument::new("a", "rust async runtime"),
                RetrievedDocument::new("b", "python web framework"),
                RetrievedDocument::new("c", "rust graph execution"),
            ],
        );
        let docs = block_on(retriever.retrieve("rust graph".to_string(), 2)).expect("retrieve");
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].id, "c");
        assert_eq!(docs[1].id, "a");
    }

    #[test]
    fn retriever_can_be_registered_as_tool() {
        let retriever: Arc<dyn Retriever> = Arc::new(InMemoryRetriever::from_documents(
            "mem",
            vec![RetrievedDocument::new("a", "rust graph runtime")],
        ));
        let mut registry = ToolRegistry::new();
        register_retriever_tool(&mut registry, "retrieve", "Retrieve docs", retriever);

        let gate = Arc::new(PermissionSession::new(PermissionPolicy::default()));
        let sink: Arc<dyn EventSink> = Arc::new(NoopEventSink);
        let call = ToolCall::new(
            "retrieve",
            "call-1",
            serde_json::json!({"query": "rust", "limit": 1}),
        );
        let context = ToolContext::new(
            sink,
            gate,
            AttachmentPolicy::default(),
            call.tool.clone(),
            call.call_id.clone(),
        );

        let output = block_on(registry.run_with_events(call, context)).expect("tool run");
        let count = output
            .content
            .get("count")
            .and_then(|value| value.as_u64())
            .unwrap_or_default();
        assert_eq!(count, 1);
    }

    #[derive(Clone, Default)]
    struct ComponentState {
        query: String,
        retrieved: usize,
        answer: String,
    }

    impl GraphState for ComponentState {}

    #[test]
    fn component_interfaces_work_in_graph_workflow() {
        let retriever = InMemoryRetriever::from_documents(
            "mem",
            vec![
                RetrievedDocument::new("d1", "forge is a stateful runtime"),
                RetrievedDocument::new("d2", "forge supports event streaming"),
                RetrievedDocument::new("d3", "irrelevant text"),
            ],
        );
        let chat = MockChatModel::new("mock-chat", "Forge supports event-driven workflows.");

        let mut graph = StateGraph::<ComponentState>::new();
        graph.add_node("answer", move |mut state: ComponentState| {
            let retriever = retriever.clone();
            let chat = chat.clone();
            async move {
                let docs = retriever.retrieve(state.query.clone(), 2).await?;
                state.retrieved = docs.len();

                let request = ChatRequest::new("s1", "m1", Vec::new())
                    .with_metadata("query", serde_json::json!(state.query))
                    .with_metadata("docs", serde_json::json!(docs.len()));
                let response = chat.generate(request).await?;
                state.answer = response.text().unwrap_or_default();
                Ok(state)
            }
        });
        graph.add_edge(START, "answer");
        graph.add_edge("answer", END);

        let compiled = graph.compile().expect("compile");
        let state = ComponentState {
            query: "forge runtime".to_string(),
            ..ComponentState::default()
        };
        let final_state = block_on(compiled.invoke(state)).expect("invoke");
        assert_eq!(final_state.retrieved, 2);
        assert_eq!(final_state.answer, "Forge supports event-driven workflows.");
    }
}
