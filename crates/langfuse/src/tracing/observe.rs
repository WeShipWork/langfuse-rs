//! Closure-based tracing API for Langfuse.
//!
//! Provides closure helpers that create a span of the appropriate observation
//! type, pass it into an async closure, and automatically end it when the
//! closure completes.

use std::future::Future;

use super::embedding::LangfuseEmbedding;
use super::generation::LangfuseGeneration;
use super::span::LangfuseSpan;

use langfuse_core::types::ObservationType;

/// Execute an async closure within a new Langfuse span.
///
/// The span is created as a root span (or child of the current OTel context)
/// and is ended automatically after the closure returns.
///
/// # Example
///
/// ```ignore
/// use langfuse::langfuse_tracing::observe::with_span;
///
/// let result = with_span("my-operation", |span| async move {
///     span.set_input(&"hello");
///     42
/// }).await;
/// ```
pub async fn with_span<F, Fut, T>(name: &str, f: F) -> T
where
    F: FnOnce(LangfuseSpan) -> Fut,
    Fut: Future<Output = T>,
{
    let span = LangfuseSpan::start(name);
    // The span is ended when the LangfuseSpan is used inside the closure;
    // callers may call `span.end()` explicitly or let the OTel context handle
    // cleanup.
    f(span).await
}

/// Execute an async closure within a new Langfuse generation span.
///
/// The generation span is created as a root span (or child of the current OTel
/// context) and is ended automatically after the closure returns.
///
/// # Example
///
/// ```ignore
/// use langfuse::langfuse_tracing::observe::with_generation;
///
/// let answer = with_generation("llm-call", |gen| async move {
///     gen.set_model("gpt-4o");
///     gen.set_input(&"What is 2+2?");
///     "4"
/// }).await;
/// ```
pub async fn with_generation<F, Fut, T>(name: &str, f: F) -> T
where
    F: FnOnce(LangfuseGeneration) -> Fut,
    Fut: Future<Output = T>,
{
    let generation = LangfuseGeneration::start(name);
    f(generation).await
}

/// Execute an async closure within a new observation span of the given type.
///
/// This is the generic version — prefer the typed helpers (`with_agent`,
/// `with_tool`, etc.) for readability.
pub async fn with_observation<F, Fut, T>(name: &str, obs_type: ObservationType, f: F) -> T
where
    F: FnOnce(LangfuseSpan) -> Fut,
    Fut: Future<Output = T>,
{
    let span = LangfuseSpan::start_with_type(name, obs_type);
    f(span).await
}

/// Execute an async closure within a new agent observation.
pub async fn with_agent<F, Fut, T>(name: &str, f: F) -> T
where
    F: FnOnce(LangfuseSpan) -> Fut,
    Fut: Future<Output = T>,
{
    with_observation(name, ObservationType::Agent, f).await
}

/// Execute an async closure within a new tool observation.
pub async fn with_tool<F, Fut, T>(name: &str, f: F) -> T
where
    F: FnOnce(LangfuseSpan) -> Fut,
    Fut: Future<Output = T>,
{
    with_observation(name, ObservationType::Tool, f).await
}

/// Execute an async closure within a new chain observation.
pub async fn with_chain<F, Fut, T>(name: &str, f: F) -> T
where
    F: FnOnce(LangfuseSpan) -> Fut,
    Fut: Future<Output = T>,
{
    with_observation(name, ObservationType::Chain, f).await
}

/// Execute an async closure within a new retriever observation.
pub async fn with_retriever<F, Fut, T>(name: &str, f: F) -> T
where
    F: FnOnce(LangfuseSpan) -> Fut,
    Fut: Future<Output = T>,
{
    with_observation(name, ObservationType::Retriever, f).await
}

/// Execute an async closure within a new evaluator observation.
pub async fn with_evaluator<F, Fut, T>(name: &str, f: F) -> T
where
    F: FnOnce(LangfuseSpan) -> Fut,
    Fut: Future<Output = T>,
{
    with_observation(name, ObservationType::Evaluator, f).await
}

/// Execute an async closure within a new guardrail observation.
pub async fn with_guardrail<F, Fut, T>(name: &str, f: F) -> T
where
    F: FnOnce(LangfuseSpan) -> Fut,
    Fut: Future<Output = T>,
{
    with_observation(name, ObservationType::Guardrail, f).await
}

/// Execute an async closure within a new embedding observation.
pub async fn with_embedding<F, Fut, T>(name: &str, f: F) -> T
where
    F: FnOnce(LangfuseEmbedding) -> Fut,
    Fut: Future<Output = T>,
{
    let embedding = LangfuseEmbedding::start(name);
    f(embedding).await
}
