//! `LangfuseEmbedding` — an embedding-specific wrapper around [`LangfuseSpan`].
//!
//! Adds setters for model name and token usage. All base span methods are
//! available via `Deref`.

use std::ops::Deref;

use langfuse_core::types::{ObservationType, UsageDetails};

use super::attributes;
use super::span::LangfuseSpan;

/// A Langfuse embedding observation.
///
/// Wraps a [`LangfuseSpan`] and exposes embedding-specific attribute setters
/// for model name and token usage. All span-level methods are accessible
/// through [`Deref`].
///
/// ```ignore
/// let emb = LangfuseEmbedding::start("embed-query");
/// emb.set_model("text-embedding-ada-002");
/// emb.set_input(&"search query");
/// emb.end();
/// ```
pub struct LangfuseEmbedding {
    span: LangfuseSpan,
}

impl std::fmt::Debug for LangfuseEmbedding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LangfuseEmbedding")
            .field("trace_id", &self.span.trace_id())
            .field("span_id", &self.span.span_id())
            .finish()
    }
}

impl Clone for LangfuseEmbedding {
    fn clone(&self) -> Self {
        Self {
            span: self.span.clone(),
        }
    }
}

impl LangfuseEmbedding {
    /// Start a new root embedding span.
    #[must_use]
    pub fn start(name: &str) -> Self {
        use opentelemetry::trace::{TraceContextExt, Tracer};
        let tracer = opentelemetry::global::tracer("langfuse");
        let otel_span = tracer.start(name.to_owned());
        let context = opentelemetry::Context::current().with_span(otel_span);
        let span = LangfuseSpan::from_context(context);
        if let Ok(serde_json::Value::String(s)) = serde_json::to_value(ObservationType::Embedding) {
            span.set_string_attribute(attributes::LANGFUSE_OBSERVATION_TYPE, &s);
        }
        Self { span }
    }

    /// Wrap an existing [`LangfuseSpan`] as an embedding.
    pub(crate) fn from_span(span: LangfuseSpan) -> Self {
        Self { span }
    }

    // ------------------------------------------------------------------
    // Embedding-specific setters
    // ------------------------------------------------------------------

    /// Set the model name (e.g. `"text-embedding-ada-002"`).
    pub fn set_model(&self, model: &str) -> &Self {
        self.span
            .set_string_attribute(attributes::LANGFUSE_MODEL, model);
        self
    }

    /// Set token usage details.
    pub fn set_usage(&self, usage: &UsageDetails) -> &Self {
        self.span
            .set_json_attribute(attributes::LANGFUSE_USAGE, usage);
        self
    }
}

impl Deref for LangfuseEmbedding {
    type Target = LangfuseSpan;

    fn deref(&self) -> &Self::Target {
        &self.span
    }
}
