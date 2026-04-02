//! Context-aware convenience APIs for interacting with the current span,
//! generation, trace scoring, and trace metadata without requiring explicit
//! handles.

use langfuse_core::error::LangfuseError;
use langfuse_core::types::{ScoreBody, ScoreValue};
use opentelemetry::Context;
use opentelemetry::trace::TraceContextExt;

use super::context::{get_current_observation_id, get_current_trace_id};
use super::generation::LangfuseGeneration;
use super::span::LangfuseSpan;
use crate::client::Langfuse;

// ---------------------------------------------------------------------------
// Task 2.1: update_current_span
// ---------------------------------------------------------------------------

/// Apply a mutation to the currently-active [`LangfuseSpan`].
///
/// If there is no valid span in the current OTel context the closure is
/// silently skipped.
///
/// ```ignore
/// update_current_span(|span| {
///     span.set_metadata(&serde_json::json!({"key": "value"}));
/// });
/// ```
pub fn update_current_span(f: impl FnOnce(&LangfuseSpan)) {
    let cx = Context::current();
    let span = cx.span();
    let sc = span.span_context();
    if sc.is_valid() {
        let langfuse_span = LangfuseSpan::from_context(cx.clone());
        f(&langfuse_span);
    }
}

// ---------------------------------------------------------------------------
// Task 2.2: update_current_generation
// ---------------------------------------------------------------------------

/// Apply a mutation to the currently-active span, viewed as a
/// [`LangfuseGeneration`].
///
/// If there is no valid span in the current OTel context the closure is
/// silently skipped.
pub fn update_current_generation(f: impl FnOnce(&LangfuseGeneration)) {
    let cx = Context::current();
    let span = cx.span();
    if span.span_context().is_valid() {
        let generation = LangfuseGeneration::from_context(cx.clone());
        f(&generation);
    }
}

// ---------------------------------------------------------------------------
// Task 2.3: score_current_span
// ---------------------------------------------------------------------------

/// Score the currently-active observation (span).
///
/// Requires the global [`Langfuse`] singleton to be initialised and a valid
/// span in the current OTel context.
///
/// ```ignore
/// score_current_span("relevance", ScoreValue::Numeric(0.95))?;
/// ```
pub fn score_current_span(name: &str, value: ScoreValue) -> Result<(), LangfuseError> {
    let trace_id =
        get_current_trace_id().ok_or_else(|| LangfuseError::Otel("no active span".into()))?;
    let obs_id =
        get_current_observation_id().ok_or_else(|| LangfuseError::Otel("no active span".into()))?;
    let langfuse = Langfuse::try_get()
        .ok_or_else(|| LangfuseError::Otel("Langfuse not initialized".into()))?;
    langfuse
        .scores
        .score_observation(&trace_id, &obs_id, name, value);
    Ok(())
}

/// Score the currently-active observation with a full [`ScoreBody`].
///
/// The `trace_id` and `observation_id` fields on `body` are overwritten with
/// the values from the current OTel context.
pub fn score_current_span_with(mut body: ScoreBody) -> Result<(), LangfuseError> {
    let trace_id =
        get_current_trace_id().ok_or_else(|| LangfuseError::Otel("no active span".into()))?;
    let obs_id =
        get_current_observation_id().ok_or_else(|| LangfuseError::Otel("no active span".into()))?;
    let langfuse = Langfuse::try_get()
        .ok_or_else(|| LangfuseError::Otel("Langfuse not initialized".into()))?;
    body.trace_id = Some(trace_id);
    body.observation_id = Some(obs_id);
    langfuse.scores.score(body);
    Ok(())
}

// ---------------------------------------------------------------------------
// Task 2.4: score_current_trace
// ---------------------------------------------------------------------------

/// Score the currently-active trace (not a specific observation).
///
/// Requires the global [`Langfuse`] singleton to be initialised and a valid
/// span in the current OTel context.
pub fn score_current_trace(name: &str, value: ScoreValue) -> Result<(), LangfuseError> {
    let trace_id =
        get_current_trace_id().ok_or_else(|| LangfuseError::Otel("no active span".into()))?;
    let langfuse = Langfuse::try_get()
        .ok_or_else(|| LangfuseError::Otel("Langfuse not initialized".into()))?;
    langfuse.scores.score_trace(&trace_id, name, value);
    Ok(())
}

// ---------------------------------------------------------------------------
// Task 2.6: get_current_trace_url
// ---------------------------------------------------------------------------

/// Return the Langfuse UI URL for the currently-active trace.
///
/// Requires the global [`Langfuse`] singleton to be initialised and a valid
/// span in the current OTel context.
pub fn get_current_trace_url() -> Result<String, LangfuseError> {
    let trace_id =
        get_current_trace_id().ok_or_else(|| LangfuseError::Otel("no active span".into()))?;
    let langfuse = Langfuse::try_get()
        .ok_or_else(|| LangfuseError::Otel("Langfuse not initialized".into()))?;
    Ok(langfuse.get_trace_url(&trace_id))
}

// ---------------------------------------------------------------------------
// Task 2.8: set_current_trace_as_public
// ---------------------------------------------------------------------------

/// Mark the currently-active trace as publicly shareable.
///
/// If there is no valid span in the current OTel context the call is
/// silently skipped.
pub fn set_current_trace_as_public() {
    let cx = Context::current();
    let span = cx.span();
    if span.span_context().is_valid() {
        let langfuse_span = LangfuseSpan::from_context(cx.clone());
        langfuse_span.set_trace_as_public();
    }
}
