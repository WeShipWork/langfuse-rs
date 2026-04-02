//! Integration tests for the context-aware APIs in `context_apis.rs`.
//!
//! Tests that require the global `Langfuse` singleton verify error behaviour
//! when the singleton is not initialised (since tests cannot safely share a
//! global singleton across threads).

use langfuse::langfuse_tracing::context_apis::{
    get_current_trace_url, score_current_span, score_current_trace, set_current_trace_as_public,
    update_current_generation, update_current_span,
};
use langfuse::langfuse_tracing::span::LangfuseSpan;
use langfuse_core::types::ScoreValue;
use opentelemetry_sdk::trace::SdkTracerProvider;
use serde_json::json;

/// Set up a no-op OTel provider so spans get valid IDs.
fn init_test_provider() {
    let provider = SdkTracerProvider::builder().build();
    opentelemetry::global::set_tracer_provider(provider);
}

// =========================================================================
// update_current_span
// =========================================================================

#[test]
fn update_current_span_no_active_span_does_not_panic() {
    // No span in context — closure should be silently skipped.
    update_current_span(|_span| {
        panic!("should not be called");
    });
}

#[test]
fn update_current_span_applies_closure_to_active_span() {
    init_test_provider();

    let span = LangfuseSpan::start("ctx-update-test");
    // Attach the span's context as the current context.
    let _guard = span.context().clone().attach();

    update_current_span(|s| {
        s.set_input(&json!({"from": "context_api"}));
        s.set_metadata(&json!({"updated": true}));
    });

    span.end();
}

// =========================================================================
// update_current_generation
// =========================================================================

#[test]
fn update_current_generation_no_active_span_does_not_panic() {
    update_current_generation(|_generation| {
        panic!("should not be called");
    });
}

#[test]
fn update_current_generation_applies_closure() {
    init_test_provider();

    let span = LangfuseSpan::start("gen-ctx-test");
    let _guard = span.context().clone().attach();

    update_current_generation(|generation| {
        generation.set_model("gpt-4o");
        generation.set_input(&json!({"prompt": "hello"}));
    });

    span.end();
}

// =========================================================================
// set_current_trace_as_public
// =========================================================================

#[test]
fn set_current_trace_as_public_no_active_span_does_not_panic() {
    set_current_trace_as_public();
}

#[test]
fn set_current_trace_as_public_with_active_span() {
    init_test_provider();

    let span = LangfuseSpan::start("public-ctx-test");
    let _guard = span.context().clone().attach();

    set_current_trace_as_public();

    span.end();
}

// =========================================================================
// score_current_span — error when singleton not initialised
// =========================================================================

#[test]
fn score_current_span_returns_error_without_active_span() {
    // No span in context → should return Otel error.
    let result = score_current_span("quality", ScoreValue::Numeric(0.9));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("no active span"), "unexpected error: {err}");
}

#[test]
fn score_current_span_returns_error_without_singleton() {
    init_test_provider();

    let span = LangfuseSpan::start("score-ctx-test");
    let _guard = span.context().clone().attach();

    // Span is active but Langfuse singleton is not initialised.
    let result = score_current_span("quality", ScoreValue::Numeric(0.9));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not initialized"), "unexpected error: {err}");

    span.end();
}

// =========================================================================
// score_current_trace — error when singleton not initialised
// =========================================================================

#[test]
fn score_current_trace_returns_error_without_active_span() {
    let result = score_current_trace("accuracy", ScoreValue::Numeric(0.8));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("no active span"), "unexpected error: {err}");
}

#[test]
fn score_current_trace_returns_error_without_singleton() {
    init_test_provider();

    let span = LangfuseSpan::start("score-trace-ctx-test");
    let _guard = span.context().clone().attach();

    let result = score_current_trace("accuracy", ScoreValue::Numeric(0.8));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not initialized"), "unexpected error: {err}");

    span.end();
}

// =========================================================================
// get_current_trace_url — error when singleton not initialised
// =========================================================================

#[test]
fn get_current_trace_url_returns_error_without_active_span() {
    let result = get_current_trace_url();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("no active span"), "unexpected error: {err}");
}

#[test]
fn get_current_trace_url_returns_error_without_singleton() {
    init_test_provider();

    let span = LangfuseSpan::start("url-ctx-test");
    let _guard = span.context().clone().attach();

    let result = get_current_trace_url();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not initialized"), "unexpected error: {err}");

    span.end();
}
