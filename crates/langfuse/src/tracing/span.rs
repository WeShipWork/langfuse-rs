//! `LangfuseSpan` — a Langfuse-aware wrapper around an OTel span.
//!
//! Each setter serializes data as JSON and stores it as an OTel string attribute,
//! which the Langfuse exporter later maps to the Langfuse ingestion API.

use langfuse_core::error::LangfuseError;
use langfuse_core::types::{ObservationType, ScoreBody, ScoreValue, SpanLevel, SpanUpdateParams};
use opentelemetry::trace::{SpanContext, TraceContextExt, Tracer};
use opentelemetry::{Context, KeyValue};
use serde::Serialize;

use super::attributes;
use super::embedding::LangfuseEmbedding;
use super::generation::LangfuseGeneration;

/// A Langfuse span backed by an OTel [`Context`] that holds a
/// [`SynchronizedSpan`](opentelemetry) internally.
///
/// All attribute setters go through [`SpanRef`](opentelemetry::trace::SpanRef),
/// which uses interior mutability (`Mutex`), so every method takes `&self`.
pub struct LangfuseSpan {
    context: Context,
}

impl std::fmt::Debug for LangfuseSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LangfuseSpan")
            .field("trace_id", &self.trace_id())
            .field("span_id", &self.span_id())
            .finish()
    }
}

impl Clone for LangfuseSpan {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
        }
    }
}

impl LangfuseSpan {
    // ------------------------------------------------------------------
    // Construction
    // ------------------------------------------------------------------

    /// Start a new root span with the given name.
    ///
    /// The span is created via the global `"langfuse"` tracer and becomes the
    /// active span inside the returned [`LangfuseSpan`]'s context.
    #[must_use]
    pub fn start(name: &str) -> Self {
        let tracer = opentelemetry::global::tracer("langfuse");
        let span = tracer.start(name.to_owned());
        let context = Context::current().with_span(span);
        let this = Self { context };
        this.set_observation_type(ObservationType::Span);
        super::context::apply_propagated_attributes(&this);
        this
    }

    /// Start a new root span with a specific observation type.
    ///
    /// The span is created via the global `"langfuse"` tracer and tagged with
    /// the given [`ObservationType`].
    #[must_use]
    pub fn start_with_type(name: &str, obs_type: ObservationType) -> Self {
        let tracer = opentelemetry::global::tracer("langfuse");
        let span = tracer.start(name.to_owned());
        let context = Context::current().with_span(span);
        let this = Self { context };
        this.set_observation_type(obs_type);
        super::context::apply_propagated_attributes(&this);
        this
    }

    /// Create a `LangfuseSpan` from an already-constructed OTel [`Context`].
    ///
    /// The caller is responsible for ensuring the context contains a valid span.
    pub(crate) fn from_context(context: Context) -> Self {
        Self { context }
    }

    // ------------------------------------------------------------------
    // Langfuse attribute setters
    // ------------------------------------------------------------------

    /// Set the input payload (JSON-serialized).
    pub fn set_input(&self, input: &impl Serialize) -> &Self {
        self.set_json_attribute(attributes::LANGFUSE_INPUT, input);
        self
    }

    /// Set the output payload (JSON-serialized).
    pub fn set_output(&self, output: &impl Serialize) -> &Self {
        self.set_json_attribute(attributes::LANGFUSE_OUTPUT, output);
        self
    }

    /// Set arbitrary metadata (JSON-serialized).
    pub fn set_metadata(&self, metadata: &impl Serialize) -> &Self {
        self.set_json_attribute(attributes::LANGFUSE_METADATA, metadata);
        self
    }

    /// Set the severity level.
    pub fn set_level(&self, level: SpanLevel) -> &Self {
        self.set_json_attribute(attributes::LANGFUSE_LEVEL, &level);
        self
    }

    /// Set a human-readable status message.
    pub fn set_status_message(&self, msg: &str) -> &Self {
        self.set_string_attribute(attributes::LANGFUSE_STATUS_MESSAGE, msg);
        self
    }

    /// Set the trace-level user ID.
    pub fn set_trace_user_id(&self, user_id: &str) -> &Self {
        self.set_string_attribute(attributes::LANGFUSE_USER_ID, user_id);
        self
    }

    /// Set the trace-level session ID.
    pub fn set_trace_session_id(&self, session_id: &str) -> &Self {
        self.set_string_attribute(attributes::LANGFUSE_SESSION_ID, session_id);
        self
    }

    /// Set trace-level tags (JSON-serialized array).
    pub fn set_trace_tags(&self, tags: &[&str]) -> &Self {
        self.set_json_attribute(attributes::LANGFUSE_TAGS, &tags);
        self
    }

    /// Set the version tag.
    pub fn set_version(&self, version: &str) -> &Self {
        self.set_string_attribute(attributes::LANGFUSE_VERSION, version);
        self
    }

    /// Batch-update multiple span attributes in a single call.
    ///
    /// Only fields set to `Some` are applied; `None` fields are left unchanged.
    pub fn update(&self, params: SpanUpdateParams) -> &Self {
        if let Some(ref output) = params.output {
            self.set_json_attribute(attributes::LANGFUSE_OUTPUT, output);
        }
        if let Some(ref metadata) = params.metadata {
            self.set_json_attribute(attributes::LANGFUSE_METADATA, metadata);
        }
        if let Some(level) = params.level {
            self.set_level(level);
        }
        if let Some(ref msg) = params.status_message {
            self.set_status_message(msg);
        }
        if let Some(ref version) = params.version {
            self.set_version(version);
        }
        if let Some(ref tags) = params.tags {
            let tag_refs: Vec<&str> = tags.iter().map(String::as_str).collect();
            self.set_trace_tags(&tag_refs);
        }
        self
    }

    /// Set trace-level input and output from any span in the trace.
    ///
    /// These attributes are stored as `langfuse.trace.input` / `langfuse.trace.output`
    /// and are picked up by the Langfuse server to populate the trace-level IO.
    pub fn set_trace_io(
        &self,
        input: Option<&impl Serialize>,
        output: Option<&impl Serialize>,
    ) -> &Self {
        if let Some(inp) = input {
            self.set_json_attribute(attributes::LANGFUSE_TRACE_INPUT, inp);
        }
        if let Some(out) = output {
            self.set_json_attribute(attributes::LANGFUSE_TRACE_OUTPUT, out);
        }
        self
    }

    /// Mark the parent trace as publicly shareable.
    pub fn set_trace_as_public(&self) -> &Self {
        self.set_string_attribute(attributes::LANGFUSE_TRACE_PUBLIC, "true");
        self
    }

    // ------------------------------------------------------------------
    // Child span creation
    // ------------------------------------------------------------------

    /// Start a child span of observation type `SPAN`.
    #[must_use]
    pub fn start_span(&self, name: &str) -> LangfuseSpan {
        let child_context = self.start_child(name, ObservationType::Span);
        LangfuseSpan::from_context(child_context)
    }

    /// Start a child span of observation type `GENERATION`.
    #[must_use]
    pub fn start_generation(&self, name: &str) -> LangfuseGeneration {
        let child_context = self.start_child(name, ObservationType::Generation);
        LangfuseGeneration::from_span(LangfuseSpan::from_context(child_context))
    }

    /// Start a child span with a specific observation type.
    #[must_use]
    pub fn start_child_with_type(&self, name: &str, obs_type: ObservationType) -> LangfuseSpan {
        let child_context = self.start_child(name, obs_type);
        LangfuseSpan::from_context(child_context)
    }

    /// Start a child span of observation type `AGENT`.
    #[must_use]
    pub fn start_agent(&self, name: &str) -> LangfuseSpan {
        self.start_child_with_type(name, ObservationType::Agent)
    }

    /// Start a child span of observation type `TOOL`.
    #[must_use]
    pub fn start_tool(&self, name: &str) -> LangfuseSpan {
        self.start_child_with_type(name, ObservationType::Tool)
    }

    /// Start a child span of observation type `CHAIN`.
    #[must_use]
    pub fn start_chain(&self, name: &str) -> LangfuseSpan {
        self.start_child_with_type(name, ObservationType::Chain)
    }

    /// Start a child span of observation type `RETRIEVER`.
    #[must_use]
    pub fn start_retriever(&self, name: &str) -> LangfuseSpan {
        self.start_child_with_type(name, ObservationType::Retriever)
    }

    /// Start a child span of observation type `EVALUATOR`.
    #[must_use]
    pub fn start_evaluator(&self, name: &str) -> LangfuseSpan {
        self.start_child_with_type(name, ObservationType::Evaluator)
    }

    /// Start a child span of observation type `GUARDRAIL`.
    #[must_use]
    pub fn start_guardrail(&self, name: &str) -> LangfuseSpan {
        self.start_child_with_type(name, ObservationType::Guardrail)
    }

    /// Start a child embedding observation.
    #[must_use]
    pub fn start_embedding(&self, name: &str) -> LangfuseEmbedding {
        let child_context = self.start_child(name, ObservationType::Embedding);
        LangfuseEmbedding::from_span(LangfuseSpan::from_context(child_context))
    }

    /// Create a zero-duration event observation as a child of this span.
    ///
    /// Events are point-in-time observations that carry input data but do not
    /// support child span creation. The event span is immediately ended.
    pub fn create_event(&self, name: &str, input: &impl Serialize) {
        let child_context = self.start_child(name, ObservationType::Event);
        let event_span = LangfuseSpan::from_context(child_context);
        event_span.set_input(input);
        event_span.end();
    }

    // ------------------------------------------------------------------
    // Span lifecycle & identity
    // ------------------------------------------------------------------

    /// End this span.
    pub fn end(&self) {
        self.context.span().end();
    }

    /// Return the OTel [`SpanContext`] for this span.
    pub fn span_context(&self) -> SpanContext {
        self.context.span().span_context().clone()
    }

    /// Return the trace ID as a hex string.
    pub fn trace_id(&self) -> String {
        self.context.span().span_context().trace_id().to_string()
    }

    /// Return the span ID as a hex string.
    pub fn span_id(&self) -> String {
        self.context.span().span_context().span_id().to_string()
    }

    /// Return the OTel [`Context`] that carries this span.
    pub fn context(&self) -> &Context {
        &self.context
    }

    // ------------------------------------------------------------------
    // Scoring
    // ------------------------------------------------------------------

    /// Score this observation with a name and value.
    ///
    /// Requires the global [`Langfuse`](crate::Langfuse) singleton to be
    /// initialised.
    pub fn score(&self, name: &str, value: ScoreValue) -> Result<(), LangfuseError> {
        let trace_id = self.trace_id();
        let span_id = self.span_id();
        let langfuse = crate::client::Langfuse::try_get()
            .ok_or_else(|| LangfuseError::Otel("Langfuse not initialized".into()))?;
        langfuse
            .scores
            .score_observation(&trace_id, &span_id, name, value);
        Ok(())
    }

    /// Score this observation with a full [`ScoreBody`].
    ///
    /// The `trace_id` and `observation_id` fields on `body` are overwritten
    /// with the values from this span.
    pub fn score_with(&self, mut body: ScoreBody) -> Result<(), LangfuseError> {
        body.trace_id = Some(self.trace_id());
        body.observation_id = Some(self.span_id());
        let langfuse = crate::client::Langfuse::try_get()
            .ok_or_else(|| LangfuseError::Otel("Langfuse not initialized".into()))?;
        langfuse.scores.score(body);
        Ok(())
    }

    /// Score the parent trace (not this specific observation).
    ///
    /// Requires the global [`Langfuse`](crate::Langfuse) singleton to be
    /// initialised.
    pub fn score_trace(&self, name: &str, value: ScoreValue) -> Result<(), LangfuseError> {
        let trace_id = self.trace_id();
        let langfuse = crate::client::Langfuse::try_get()
            .ok_or_else(|| LangfuseError::Otel("Langfuse not initialized".into()))?;
        langfuse.scores.score_trace(&trace_id, name, value);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Set the `langfuse.observation.type` attribute.
    fn set_observation_type(&self, obs_type: ObservationType) -> &Self {
        // Serialize to a JSON Value to get the SCREAMING_SNAKE_CASE string,
        // then store as a plain string attribute (not JSON-quoted).
        if let Ok(serde_json::Value::String(s)) = serde_json::to_value(obs_type) {
            self.set_string_attribute(attributes::LANGFUSE_OBSERVATION_TYPE, &s);
        }
        self
    }

    /// Serialize `value` to JSON and set it as a string attribute.
    ///
    /// If a mask function is configured on the global `Langfuse` instance,
    /// the serialized JSON value is passed through it before being stored.
    /// If the mask function panics, `"[MASK_ERROR]"` is used as a fallback.
    pub(crate) fn set_json_attribute(&self, key: &'static str, value: &impl Serialize) {
        if let Ok(json) = serde_json::to_string(value) {
            let final_json = if let Some(mask_fn) =
                crate::Langfuse::try_get().and_then(|l| l.config().mask.as_ref())
            {
                match serde_json::from_str::<serde_json::Value>(&json) {
                    Ok(parsed) => {
                        let mask_ref = std::panic::AssertUnwindSafe(mask_fn);
                        let parsed = std::panic::AssertUnwindSafe(parsed);
                        match std::panic::catch_unwind(move || (mask_ref)(parsed.0)) {
                            Ok(masked) => serde_json::to_string(&masked).unwrap_or(json),
                            Err(_) => "\"[MASK_ERROR]\"".to_owned(),
                        }
                    }
                    Err(_) => json,
                }
            } else {
                json
            };
            self.context
                .span()
                .set_attribute(KeyValue::new(key, final_json));
        }
    }

    /// Set a plain string attribute.
    pub fn set_string_attribute(&self, key: &'static str, value: &str) {
        self.context
            .span()
            .set_attribute(KeyValue::new(key, value.to_owned()));
    }

    /// Create a child span in this span's context and tag it with the given
    /// observation type.
    fn start_child(&self, name: &str, obs_type: ObservationType) -> Context {
        let tracer = opentelemetry::global::tracer("langfuse");
        let child_span = tracer.start_with_context(name.to_owned(), &self.context);
        let child_context = self.context.with_span(child_span);
        // Tag the child with its observation type (plain string, not JSON-quoted).
        if let Ok(serde_json::Value::String(s)) = serde_json::to_value(obs_type) {
            child_context
                .span()
                .set_attribute(KeyValue::new(attributes::LANGFUSE_OBSERVATION_TYPE, s));
        }
        child_context
    }
}
