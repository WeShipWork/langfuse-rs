use serde::{Deserialize, Serialize};

/// The 10 observation types supported by Langfuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObservationType {
    /// A generic span representing a unit of work.
    Span,
    /// An LLM generation (completion, embedding request).
    Generation,
    /// A discrete event within a trace.
    Event,
    /// An embedding operation.
    Embedding,
    /// An AI agent.
    Agent,
    /// A tool invocation.
    Tool,
    /// A processing chain.
    Chain,
    /// A retrieval operation (e.g. RAG).
    Retriever,
    /// An evaluation step.
    Evaluator,
    /// A guardrail check.
    Guardrail,
}

/// Severity level for spans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpanLevel {
    /// Debug-level severity.
    Debug,
    /// Default severity.
    Default,
    /// Warning-level severity.
    Warning,
    /// Error-level severity.
    Error,
}

/// Token usage details for LLM generations.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageDetails {
    /// Number of input tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<u64>,
    /// Number of output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<u64>,
    /// Total token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

/// Cost details for LLM generations.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostDetails {
    /// Cost of input tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<f64>,
    /// Cost of output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<f64>,
    /// Total cost.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
}

/// Attributes that propagate from a parent trace to child spans.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropagateAttributes {
    /// User ID to associate with the trace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Session ID for grouping traces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Arbitrary metadata as JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// Version tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Tags for filtering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// Custom trace name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_name: Option<String>,
    /// Whether to propagate as W3C baggage.
    #[serde(default, skip_serializing_if = "is_false")]
    pub as_baggage: bool,
}

const fn is_false(value: &bool) -> bool {
    !*value
}

/// Parameters for batch-updating span attributes.
///
/// Only `Some` fields are applied; `None` fields leave the span unchanged.
#[derive(Debug, Clone, Default)]
pub struct SpanUpdateParams {
    /// Output data to set on the span.
    pub output: Option<serde_json::Value>,
    /// Metadata to set on the span.
    pub metadata: Option<serde_json::Value>,
    /// Severity level.
    pub level: Option<SpanLevel>,
    /// Human-readable status message.
    pub status_message: Option<String>,
    /// Version tag.
    pub version: Option<String>,
    /// Trace-level tags.
    pub tags: Option<Vec<String>>,
}
