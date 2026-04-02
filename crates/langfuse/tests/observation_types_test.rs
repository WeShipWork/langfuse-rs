//! Integration tests for observation types: create, set attributes, verify OTel span type attribute.

use langfuse::langfuse_tracing::embedding::LangfuseEmbedding;
use langfuse::langfuse_tracing::generation::LangfuseGeneration;
use langfuse::langfuse_tracing::span::LangfuseSpan;
use langfuse_core::types::{ObservationType, SpanLevel, SpanUpdateParams};
use opentelemetry_sdk::trace::SdkTracerProvider;
use serde_json::json;

/// Set up a no-op OTel provider for tests.
fn init_test_provider() {
    let provider = SdkTracerProvider::builder().build();
    opentelemetry::global::set_tracer_provider(provider);
}

#[test]
fn test_start_with_type_creates_valid_span() {
    init_test_provider();

    let span = LangfuseSpan::start_with_type("test-agent", ObservationType::Agent);
    assert!(!span.trace_id().is_empty());
    assert!(!span.span_id().is_empty());
    span.end();
}

#[test]
fn test_start_with_type_all_observation_types() {
    init_test_provider();

    let types = [
        ObservationType::Span,
        ObservationType::Generation,
        ObservationType::Event,
        ObservationType::Embedding,
        ObservationType::Agent,
        ObservationType::Tool,
        ObservationType::Chain,
        ObservationType::Retriever,
        ObservationType::Evaluator,
        ObservationType::Guardrail,
    ];

    for obs_type in types {
        let span = LangfuseSpan::start_with_type("test", obs_type);
        assert!(!span.trace_id().is_empty(), "Failed for {obs_type:?}");
        span.end();
    }
}

#[test]
fn test_child_with_type() {
    init_test_provider();

    let parent = LangfuseSpan::start("parent");
    let child = parent.start_child_with_type("child", ObservationType::Tool);

    // Child should have the same trace ID but different span ID.
    assert_eq!(parent.trace_id(), child.trace_id());
    assert_ne!(parent.span_id(), child.span_id());

    child.end();
    parent.end();
}

#[test]
fn test_convenience_child_methods() {
    init_test_provider();

    let parent = LangfuseSpan::start("parent");

    let agent = parent.start_agent("agent");
    assert_eq!(parent.trace_id(), agent.trace_id());
    agent.end();

    let tool = parent.start_tool("tool");
    assert_eq!(parent.trace_id(), tool.trace_id());
    tool.end();

    let chain = parent.start_chain("chain");
    assert_eq!(parent.trace_id(), chain.trace_id());
    chain.end();

    let retriever = parent.start_retriever("retriever");
    assert_eq!(parent.trace_id(), retriever.trace_id());
    retriever.end();

    let evaluator = parent.start_evaluator("evaluator");
    assert_eq!(parent.trace_id(), evaluator.trace_id());
    evaluator.end();

    let guardrail = parent.start_guardrail("guardrail");
    assert_eq!(parent.trace_id(), guardrail.trace_id());
    guardrail.end();

    let embedding = parent.start_embedding("embedding");
    assert_eq!(parent.trace_id(), embedding.trace_id());
    embedding.end();

    parent.end();
}

#[test]
fn test_embedding_wrapper() {
    init_test_provider();

    let embedding = LangfuseEmbedding::start("embed-query");
    embedding.set_model("text-embedding-ada-002");
    embedding.set_usage(&langfuse_core::types::UsageDetails {
        input: Some(10),
        output: None,
        total: Some(10),
    });
    // Deref to LangfuseSpan allows calling span methods.
    embedding.set_input(&json!("Hello world"));
    embedding.set_metadata(&json!({"source": "test"}));
    embedding.end();
}

#[test]
fn test_generation_start_and_methods() {
    init_test_provider();

    let generation = LangfuseGeneration::start("gen");
    generation.set_model("gpt-4o");
    generation.set_input(&json!({"prompt": "test"}));
    generation.set_output(&json!({"response": "ok"}));
    generation.end();
}

#[test]
fn test_update_span_params() {
    init_test_provider();

    let span = LangfuseSpan::start("update-test");

    let params = SpanUpdateParams {
        output: Some(json!({"result": "ok"})),
        metadata: Some(json!({"key": "value"})),
        level: Some(SpanLevel::Warning),
        status_message: Some("attention needed".into()),
        version: Some("v2".into()),
        tags: Some(vec!["tag1".into(), "tag2".into()]),
    };

    span.update(params);
    span.end();
}

#[test]
fn test_update_partial_params() {
    init_test_provider();

    let span = LangfuseSpan::start("partial-update");

    // Only set output and level, leave the rest as None.
    let params = SpanUpdateParams {
        output: Some(json!("partial output")),
        level: Some(SpanLevel::Error),
        ..Default::default()
    };

    span.update(params);
    span.end();
}

#[test]
fn test_set_trace_io() {
    init_test_provider();

    let span = LangfuseSpan::start("trace-io-test");
    let input = json!({"query": "test input"});
    let output = json!({"result": "test output"});

    span.set_trace_io(Some(&input), Some(&output));
    span.end();
}

#[test]
fn test_set_trace_io_partial() {
    init_test_provider();

    let span = LangfuseSpan::start("trace-io-partial");

    // Set only input, not output.
    span.set_trace_io(Some(&json!("only input")), None::<&serde_json::Value>);
    span.end();
}

#[test]
fn test_set_version() {
    init_test_provider();

    let span = LangfuseSpan::start("version-test");
    span.set_version("v1.2.3");
    span.end();
}

#[test]
fn test_set_trace_as_public() {
    init_test_provider();

    let span = LangfuseSpan::start("public-test");
    span.set_trace_as_public();
    span.end();
}

#[tokio::test]
async fn test_closure_helpers_with_agent() {
    init_test_provider();

    let result = langfuse::with_agent("my-agent", |span| async move {
        span.set_input(&json!({"task": "research"}));
        42
    })
    .await;

    assert_eq!(result, 42);
}

#[tokio::test]
async fn test_closure_helpers_with_tool() {
    init_test_provider();

    let result = langfuse::with_tool("my-tool", |span| async move {
        span.set_input(&json!({"command": "execute"}));
        "done"
    })
    .await;

    assert_eq!(result, "done");
}

#[tokio::test]
async fn test_closure_helpers_with_observation() {
    init_test_provider();

    let result =
        langfuse::with_observation("custom-obs", ObservationType::Chain, |span| async move {
            span.set_input(&json!({"step": 1}));
            true
        })
        .await;

    assert!(result);
}

#[tokio::test]
async fn test_closure_helpers_with_embedding() {
    init_test_provider();

    let result = langfuse::with_embedding("embed", |emb| async move {
        emb.set_model("ada-002");
        emb.set_input(&json!("embed this text"));
        vec![0.1, 0.2, 0.3]
    })
    .await;

    assert_eq!(result, vec![0.1, 0.2, 0.3]);
}
