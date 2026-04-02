//! Tests for `LangfuseSpan`, `LangfuseGeneration`, context helpers, and the
//! closure-based tracing API.
//!
//! These tests run against the default no-op OTel provider — no real exporter
//! is configured, so spans are silently discarded.  The goal is to verify that
//! the API surface is usable and does not panic.

use langfuse::langfuse_tracing::context::{get_current_observation_id, get_current_trace_id};
use langfuse::langfuse_tracing::generation::LangfuseGeneration;
use langfuse::langfuse_tracing::observe::{with_generation, with_span};
use langfuse::langfuse_tracing::span::LangfuseSpan;
use langfuse_core::types::{CostDetails, SpanLevel, UsageDetails};
use serde_json::json;

// =========================================================================
// LangfuseSpan — basic construction & setters
// =========================================================================

#[test]
fn span_start_does_not_panic() {
    let span = LangfuseSpan::start("test-span");
    // With the no-op provider the trace/span IDs are all-zero, but the call
    // must not panic.
    let _trace_id = span.trace_id();
    let _span_id = span.span_id();
    span.end();
}

#[test]
fn span_set_input_output_metadata() {
    let span = LangfuseSpan::start("io-span");
    span.set_input(&json!({"prompt": "hello"}))
        .set_output(&json!({"response": "world"}))
        .set_metadata(&json!({"key": "value"}));
    span.end();
}

#[test]
fn span_set_level_and_status_message() {
    let span = LangfuseSpan::start("level-span");
    span.set_level(SpanLevel::Warning)
        .set_status_message("something went wrong");
    span.end();
}

#[test]
fn span_set_trace_user_and_session() {
    let span = LangfuseSpan::start("user-span");
    span.set_trace_user_id("user-123")
        .set_trace_session_id("session-456")
        .set_trace_tags(&["tag-a", "tag-b"]);
    span.end();
}

#[test]
fn span_context_returns_context() {
    let span = LangfuseSpan::start("ctx-span");
    let _ctx = span.context();
    span.end();
}

#[test]
fn span_context_returns_span_context() {
    let span = LangfuseSpan::start("sc-span");
    let sc = span.span_context();
    // With no-op provider, the span context exists but may not be valid.
    let _trace_id = sc.trace_id();
    let _span_id = sc.span_id();
    span.end();
}

// =========================================================================
// LangfuseSpan — child span creation
// =========================================================================

#[test]
fn span_start_child_span() {
    let parent = LangfuseSpan::start("parent");
    let child = parent.start_span("child");
    // Child should be a distinct span.
    let _child_id = child.span_id();
    child.end();
    parent.end();
}

#[test]
fn span_start_child_generation() {
    let parent = LangfuseSpan::start("parent");
    let child_gen = parent.start_generation("llm-call");
    child_gen.set_model("gpt-4o");
    child_gen.end();
    parent.end();
}

// =========================================================================
// LangfuseGeneration — construction & setters
// =========================================================================

#[test]
fn generation_start_does_not_panic() {
    let generation = LangfuseGeneration::start("test-gen");
    let _trace_id = generation.trace_id();
    generation.end();
}

#[test]
fn generation_set_model_and_parameters() {
    let generation = LangfuseGeneration::start("model-gen");
    generation
        .set_model("gpt-4o")
        .set_model_parameters(&json!({"temperature": 0.7, "max_tokens": 1024}));
    generation.end();
}

#[test]
fn generation_set_usage() {
    let generation = LangfuseGeneration::start("usage-gen");
    let usage = UsageDetails {
        input: Some(100),
        output: Some(50),
        total: Some(150),
    };
    generation.set_usage(&usage);
    generation.end();
}

#[test]
fn generation_set_cost() {
    let generation = LangfuseGeneration::start("cost-gen");
    let cost = CostDetails {
        input: Some(0.001),
        output: Some(0.002),
        total: Some(0.003),
    };
    generation.set_cost(&cost);
    generation.end();
}

#[test]
fn generation_set_completion_start_time() {
    let generation = LangfuseGeneration::start("time-gen");
    let now = chrono::Utc::now();
    generation.set_completion_start_time(&now);
    generation.end();
}

#[test]
fn generation_deref_to_span_methods() {
    let generation = LangfuseGeneration::start("deref-gen");
    // These methods come from LangfuseSpan via Deref.
    generation
        .set_input(&"prompt text")
        .set_output(&"completion text")
        .set_level(SpanLevel::Default);
    generation.end();
}

// =========================================================================
// Nesting — multi-level span trees
// =========================================================================

#[test]
fn nested_span_tree() {
    let root = LangfuseSpan::start("root");
    let child_a = root.start_span("child-a");
    let grandchild = child_a.start_span("grandchild");
    grandchild.set_input(&"deep input");
    grandchild.end();
    child_a.end();

    let child_b = root.start_generation("child-b-gen");
    child_b.set_model("claude-3");
    child_b.end();

    root.end();
}

// =========================================================================
// Context helpers
// =========================================================================

#[test]
fn context_helpers_return_none_without_active_span() {
    // Without any span in the current context, these should return None.
    assert!(get_current_trace_id().is_none());
    assert!(get_current_observation_id().is_none());
}

// =========================================================================
// Closure-based API
// =========================================================================

#[tokio::test]
async fn with_span_executes_closure() {
    let result = with_span("closure-span", |span| async move {
        span.set_input(&"hello");
        42
    })
    .await;
    assert_eq!(result, 42);
}

#[tokio::test]
async fn with_generation_executes_closure() {
    let result = with_generation("closure-gen", |generation| async move {
        generation.set_model("gpt-4o");
        generation.set_input(&"prompt");
        "answer"
    })
    .await;
    assert_eq!(result, "answer");
}

#[tokio::test]
async fn with_span_nested_closures() {
    let result = with_span("outer", |outer| async move {
        outer.set_input(&"outer-input");
        let inner_result = with_span("inner", |inner| async move {
            inner.set_input(&"inner-input");
            99
        })
        .await;
        inner_result + 1
    })
    .await;
    assert_eq!(result, 100);
}

#[tokio::test]
async fn with_span_and_generation_nested() {
    with_span("parent-span", |parent| async move {
        parent.set_input(&"question");
        with_generation("llm-call", |generation| async move {
            generation.set_model("gpt-4o");
            generation.set_input(&"question");
            generation.set_output(&"answer");
            generation.set_usage(&UsageDetails {
                input: Some(10),
                output: Some(20),
                total: Some(30),
            });
        })
        .await;
        parent.set_output(&"answer");
    })
    .await;
}

// =========================================================================
// Method chaining
// =========================================================================

#[test]
fn span_method_chaining() {
    let span = LangfuseSpan::start("chain-span");
    span.set_input(&"in")
        .set_output(&"out")
        .set_metadata(&json!({}))
        .set_level(SpanLevel::Debug)
        .set_status_message("ok")
        .set_trace_user_id("u1")
        .set_trace_session_id("s1")
        .set_trace_tags(&["a", "b"]);
    span.end();
}

#[test]
fn generation_method_chaining() {
    let generation = LangfuseGeneration::start("chain-gen");
    generation
        .set_model("gpt-4o")
        .set_model_parameters(&json!({"temperature": 0.5}))
        .set_usage(&UsageDetails {
            input: Some(1),
            output: Some(2),
            total: Some(3),
        })
        .set_cost(&CostDetails {
            input: Some(0.01),
            output: Some(0.02),
            total: Some(0.03),
        })
        .set_completion_start_time(&chrono::Utc::now());
    // Also chain inherited span methods via Deref.
    generation.set_input(&"prompt").set_output(&"completion");
    generation.end();
}
