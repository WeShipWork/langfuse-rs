//! Traced wrappers around async-openai types that automatically create
//! Langfuse observation spans for every API call.

use std::pin::Pin;
use std::task::{Context, Poll};

use async_openai::Chat;
use async_openai::Embeddings;
use async_openai::config::Config;
use async_openai::error::OpenAIError;
use async_openai::types::chat::{
    CreateChatCompletionRequest, CreateChatCompletionResponse, CreateChatCompletionStreamResponse,
};
use async_openai::types::embeddings::{CreateEmbeddingRequest, CreateEmbeddingResponse};
use futures::Stream;

use crate::parser::{self, ToolCallAccumulator};
use langfuse::{LangfuseEmbedding, LangfuseGeneration};
use langfuse_core::types::UsageDetails;

/// A wrapper around async-openai's [`Chat`] that automatically creates
/// Langfuse generation spans for every chat completion API call.
pub struct TracedChat<'c, C: Config> {
    inner: Chat<'c, C>,
}

impl<C: Config> std::fmt::Debug for TracedChat<'_, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TracedChat").finish()
    }
}

impl<'c, C: Config> TracedChat<'c, C> {
    /// Wrap an existing [`Chat`] instance with Langfuse tracing.
    #[must_use]
    pub fn new(chat: Chat<'c, C>) -> Self {
        Self { inner: chat }
    }

    /// Create a chat completion with automatic Langfuse tracing.
    ///
    /// A generation span is created before the request and ended after the
    /// response is received. Model, usage, input, and output are recorded
    /// automatically.
    ///
    /// # Errors
    ///
    /// Returns the underlying [`OpenAIError`] if the API call fails.
    pub async fn create(
        &self,
        request: CreateChatCompletionRequest,
    ) -> Result<CreateChatCompletionResponse, OpenAIError> {
        let generation = LangfuseGeneration::start("chat-completion");
        generation.set_input(&request);

        match self.inner.create(request).await {
            Ok(response) => {
                generation.set_model(&parser::extract_model(&response));
                if let Some(usage) = parser::extract_usage(&response) {
                    generation.set_usage(&usage);
                }
                generation.set_output(&parser::extract_output(&response));
                if let Some(tool_calls) = parser::extract_tool_calls(&response) {
                    generation.set_tool_calls(&tool_calls);
                }
                generation.end();
                Ok(response)
            }
            Err(err) => {
                generation.set_level(langfuse_core::types::SpanLevel::Error);
                generation.set_status_message(&err.to_string());
                generation.end();
                Err(err)
            }
        }
    }

    /// Create a streaming chat completion with automatic Langfuse tracing.
    ///
    /// A generation span is created before the request. The returned
    /// [`TracedStream`] accumulates content from delta chunks and records
    /// `completion_start_time` on the first chunk. The span is finalized
    /// when the stream ends or is dropped.
    ///
    /// # Errors
    ///
    /// Returns the underlying [`OpenAIError`] if the API call fails.
    pub async fn create_stream(
        &self,
        request: CreateChatCompletionRequest,
    ) -> Result<TracedStream, OpenAIError> {
        let generation = LangfuseGeneration::start("chat-completion");
        generation.set_input(&request);

        let stream = self.inner.create_stream(request).await?;
        Ok(TracedStream::new(stream, generation))
    }
}

/// Create a [`TracedChat`] wrapper from an async-openai
/// [`Client`](async_openai::Client).
pub fn observe_openai<C: Config>(client: &async_openai::Client<C>) -> TracedChat<'_, C> {
    TracedChat::new(client.chat())
}

// ---------------------------------------------------------------------------
// TracedStream
// ---------------------------------------------------------------------------

/// A stream wrapper that accumulates content and records Langfuse
/// generation attributes as chunks arrive.
///
/// On the first chunk, `completion_start_time` is set. On each chunk,
/// delta content is accumulated. When the stream ends (or on drop), the
/// generation span is finalized with model, usage, and output.
pub struct TracedStream {
    inner:
        Pin<Box<dyn Stream<Item = Result<CreateChatCompletionStreamResponse, OpenAIError>> + Send>>,
    generation: Option<LangfuseGeneration>,
    accumulated_content: String,
    model: Option<String>,
    first_chunk: bool,
    tool_call_acc: ToolCallAccumulator,
}

impl std::fmt::Debug for TracedStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TracedStream")
            .field("first_chunk", &self.first_chunk)
            .field("model", &self.model)
            .field("accumulated_content", &self.accumulated_content)
            .finish_non_exhaustive()
    }
}

impl TracedStream {
    fn new(
        inner: Pin<
            Box<dyn Stream<Item = Result<CreateChatCompletionStreamResponse, OpenAIError>> + Send>,
        >,
        generation: LangfuseGeneration,
    ) -> Self {
        Self {
            inner,
            generation: Some(generation),
            accumulated_content: String::new(),
            model: None,
            first_chunk: true,
            tool_call_acc: ToolCallAccumulator::new(),
        }
    }

    /// End the generation span with whatever data has been accumulated so far.
    fn finalize(&mut self) {
        if let Some(generation) = self.generation.take() {
            if let Some(model) = &self.model {
                generation.set_model(model);
            }
            if !self.accumulated_content.is_empty() {
                generation.set_output(&self.accumulated_content);
            }
            if self.tool_call_acc.has_calls() {
                generation.set_tool_calls(&self.tool_call_acc.finalize());
            }
            generation.end();
        }
    }
}

impl Stream for TracedStream {
    type Item = Result<CreateChatCompletionStreamResponse, OpenAIError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // `Pin<Box<dyn Stream + Send>>` is `Unpin`, so we can safely get `&mut Self`.
        let this = self.get_mut();

        match this.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                // Record completion_start_time on the very first chunk.
                if this.first_chunk {
                    this.first_chunk = false;
                    if let Some(span) = this.generation.as_ref() {
                        span.set_completion_start_time(&chrono::Utc::now());
                    }
                }

                // Capture model from the first chunk that has it.
                if this.model.is_none() {
                    this.model = Some(chunk.model.clone());
                }

                // Accumulate delta content.
                if let Some(content) = parser::extract_stream_chunk_content(&chunk) {
                    this.accumulated_content.push_str(&content);
                }

                // If this chunk carries usage (final chunk), record it.
                if let Some(usage) = parser::extract_stream_usage(&chunk)
                    && let Some(span) = this.generation.as_ref()
                {
                    span.set_usage(&usage);
                }

                // Accumulate tool call deltas.
                this.tool_call_acc.accumulate(&chunk);

                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(err))) => {
                // Record the error on the generation span and finalize.
                if let Some(span) = this.generation.as_ref() {
                    span.set_level(langfuse_core::types::SpanLevel::Error);
                    span.set_status_message(&err.to_string());
                }
                this.finalize();
                Poll::Ready(Some(Err(err)))
            }
            Poll::Ready(None) => {
                // Stream ended — finalize the generation span.
                this.finalize();
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for TracedStream {
    fn drop(&mut self) {
        self.finalize();
    }
}

// ---------------------------------------------------------------------------
// TracedEmbeddings
// ---------------------------------------------------------------------------

/// A wrapper around async-openai's [`Embeddings`] that automatically creates
/// Langfuse embedding spans for every embedding API call.
pub struct TracedEmbeddings<'c, C: Config> {
    inner: Embeddings<'c, C>,
}

impl<C: Config> std::fmt::Debug for TracedEmbeddings<'_, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TracedEmbeddings").finish()
    }
}

impl<'c, C: Config> TracedEmbeddings<'c, C> {
    /// Wrap an existing [`Embeddings`] instance with Langfuse tracing.
    #[must_use]
    pub fn new(embeddings: Embeddings<'c, C>) -> Self {
        Self { inner: embeddings }
    }

    /// Create an embedding with automatic Langfuse tracing.
    ///
    /// An embedding span is created before the request and ended after the
    /// response is received. Model, usage, and input are recorded
    /// automatically. Output vectors are intentionally omitted (too large).
    ///
    /// # Errors
    ///
    /// Returns the underlying [`OpenAIError`] if the API call fails.
    pub async fn create(
        &self,
        request: CreateEmbeddingRequest,
    ) -> Result<CreateEmbeddingResponse, OpenAIError> {
        let embedding = LangfuseEmbedding::start("embedding");
        embedding.set_input(&serde_json::json!(request.input));
        embedding.set_model(&request.model);

        match self.inner.create(request).await {
            Ok(response) => {
                // Record the model actually used (may differ from request).
                embedding.set_model(&response.model);
                embedding.set_usage(&UsageDetails {
                    input: Some(u64::from(response.usage.prompt_tokens)),
                    output: None,
                    total: Some(u64::from(response.usage.total_tokens)),
                });
                // Intentionally skip output — embedding vectors are too large.
                embedding.end();
                Ok(response)
            }
            Err(err) => {
                embedding.set_level(langfuse_core::types::SpanLevel::Error);
                embedding.set_status_message(&err.to_string());
                embedding.end();
                Err(err)
            }
        }
    }
}

/// Create a [`TracedEmbeddings`] wrapper from an async-openai
/// [`Client`](async_openai::Client).
pub fn observe_openai_embeddings<C: Config>(
    client: &async_openai::Client<C>,
) -> TracedEmbeddings<'_, C> {
    TracedEmbeddings::new(client.embeddings())
}
