//! Integration tests for event creation (standalone and nested).

use langfuse::langfuse_tracing::span::LangfuseSpan;
use langfuse_core::types::ObservationType;
use opentelemetry_sdk::trace::SdkTracerProvider;
use serde_json::json;

/// Set up a no-op OTel provider for tests.
fn init_test_provider() {
    let provider = SdkTracerProvider::builder().build();
    opentelemetry::global::set_tracer_provider(provider);
}

#[test]
fn test_create_event_nested() {
    init_test_provider();

    let parent = LangfuseSpan::start("parent-span");

    // Create a nested event — should create a child span and immediately end it.
    parent.create_event("user-clicked", &json!({"button": "submit"}));
    parent.create_event("page-view", &json!({"url": "/home"}));

    parent.end();
}

#[test]
fn test_create_event_standalone() {
    init_test_provider();

    // Create a standalone root-level event via start_with_type.
    let event = LangfuseSpan::start_with_type("standalone-event", ObservationType::Event);
    event.set_input(&json!({"action": "login"}));
    event.end();
}

#[test]
fn test_create_event_multiple_in_sequence() {
    init_test_provider();

    let span = LangfuseSpan::start("pipeline");

    // Events are lightweight — we can create many.
    for i in 0..10 {
        span.create_event(&format!("step-{i}"), &json!({"index": i}));
    }

    span.end();
}
