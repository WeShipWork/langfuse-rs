//! `LangfuseGeneration` — a generation-specific wrapper around [`LangfuseSpan`].
//!
//! Adds setters for model name, model parameters, token usage, cost, and
//! completion-start time.  All base span methods are available via `Deref`.

use std::ops::Deref;

use langfuse_core::types::{CostDetails, ObservationType, UsageDetails};
use serde::Serialize;

use super::attributes;
use super::span::LangfuseSpan;

/// A Langfuse generation observation.
///
/// Wraps a [`LangfuseSpan`] and exposes generation-specific attribute setters.
/// All span-level methods are accessible through [`Deref`].
pub struct LangfuseGeneration {
    span: LangfuseSpan,
}

impl std::fmt::Debug for LangfuseGeneration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LangfuseGeneration")
            .field("trace_id", &self.span.trace_id())
            .field("span_id", &self.span.span_id())
            .finish()
    }
}

impl Clone for LangfuseGeneration {
    fn clone(&self) -> Self {
        Self {
            span: self.span.clone(),
        }
    }
}

impl LangfuseGeneration {
    /// Start a new root generation span.
    #[must_use]
    pub fn start(name: &str) -> Self {
        use opentelemetry::trace::{TraceContextExt, Tracer};
        let tracer = opentelemetry::global::tracer("langfuse");
        let otel_span = tracer.start(name.to_owned());
        let context = opentelemetry::Context::current().with_span(otel_span);
        let span = LangfuseSpan::from_context(context);
        if let Ok(serde_json::Value::String(s)) = serde_json::to_value(ObservationType::Generation)
        {
            span.set_string_attribute(attributes::LANGFUSE_OBSERVATION_TYPE, &s);
        }
        Self { span }
    }

    /// Wrap an existing [`LangfuseSpan`] as a generation.
    pub(crate) fn from_span(span: LangfuseSpan) -> Self {
        Self { span }
    }

    /// Create a `LangfuseGeneration` from an already-constructed OTel [`Context`].
    ///
    /// The caller is responsible for ensuring the context contains a valid span.
    pub(crate) fn from_context(context: opentelemetry::Context) -> Self {
        Self {
            span: LangfuseSpan::from_context(context),
        }
    }

    // ------------------------------------------------------------------
    // Generation-specific setters
    // ------------------------------------------------------------------

    /// Set the model name (e.g. `"gpt-4o"`).
    pub fn set_model(&self, model: &str) -> &Self {
        self.span
            .set_string_attribute(attributes::LANGFUSE_MODEL, model);
        self
    }

    /// Set model parameters (JSON-serialized).
    pub fn set_model_parameters(&self, params: &impl Serialize) -> &Self {
        self.span
            .set_json_attribute(attributes::LANGFUSE_MODEL_PARAMETERS, params);
        self
    }

    /// Set token usage details.
    pub fn set_usage(&self, usage: &UsageDetails) -> &Self {
        self.span
            .set_json_attribute(attributes::LANGFUSE_USAGE, usage);
        self
    }

    /// Set cost details.
    pub fn set_cost(&self, cost: &CostDetails) -> &Self {
        self.span
            .set_json_attribute(attributes::LANGFUSE_COST, cost);
        self
    }

    /// Set tool-call details extracted from a model response.
    pub fn set_tool_calls(&self, tool_calls: &impl Serialize) -> &Self {
        self.span
            .set_json_attribute(attributes::LANGFUSE_TOOL_CALLS, tool_calls);
        self
    }

    /// Set the completion start time (ISO 8601).
    pub fn set_completion_start_time(&self, time: &chrono::DateTime<chrono::Utc>) -> &Self {
        self.span.set_string_attribute(
            attributes::LANGFUSE_COMPLETION_START_TIME,
            &time.to_rfc3339(),
        );
        self
    }
}

impl Deref for LangfuseGeneration {
    type Target = LangfuseSpan;

    fn deref(&self) -> &Self::Target {
        &self.span
    }
}
