//! Example: Advanced langfuse-rs features
//!
//! Demonstrates observation types, span scoring, context-aware APIs,
//! named instances, and ObservingIterator.
//!
//! Run: LANGFUSE_PUBLIC_KEY=pk-lf-... LANGFUSE_SECRET_KEY=sk-lf-... cargo run --example advanced_features

use langfuse::{
    Langfuse, LangfuseConfig, LangfuseSpan, ObservingIterator, ScoreBody, ScoreValue,
    SpanUpdateParams, UsageDetails,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the global singleton from environment variables.
    let config = LangfuseConfig::from_env()?;
    Langfuse::init(config.clone())?;

    println!("=== 1. Observation types (agent, tool, chain) ===");
    example_observation_types();

    println!("\n=== 2. Span scoring ===");
    example_span_scoring()?;

    println!("\n=== 3. Context-aware APIs ===");
    example_context_apis()?;

    println!("\n=== 4. Named instances ===");
    example_named_instances()?;

    println!("\n=== 5. ObservingIterator ===");
    example_observing_iterator();

    // Flush all pending data before exit.
    Langfuse::get().shutdown().await?;
    println!("\nAll examples completed successfully!");
    Ok(())
}

/// 1. Create spans with different observation types and nest them.
fn example_observation_types() {
    let agent = LangfuseSpan::start_with_type("my-agent", langfuse::ObservationType::Agent);
    agent.set_input(&json!({"task": "answer user question"}));

    // Child tool span
    let tool = agent.start_tool("web-search");
    tool.set_input(&json!({"query": "Rust programming language"}));
    tool.set_output(&json!({"results": ["rust-lang.org", "doc.rust-lang.org"]}));
    tool.end();

    // Child chain span
    let chain = agent.start_chain("reasoning-chain");
    chain.set_metadata(&json!({"steps": 3}));

    // Nested generation inside the chain
    let r#gen = chain.start_generation("llm-call");
    r#gen.set_model("gpt-4o");
    r#gen.set_usage(&UsageDetails {
        input: Some(100),
        output: Some(250),
        total: Some(350),
    });
    r#gen.set_output(&json!("Rust is a systems programming language."));
    r#gen.end();

    chain.set_output(&json!({"conclusion": "answered"}));
    chain.end();

    // Child embedding span
    let emb = agent.start_embedding("embed-query");
    emb.set_model("text-embedding-3-small");
    emb.set_usage(&UsageDetails {
        input: Some(10),
        output: None,
        total: Some(10),
    });
    emb.end();

    agent.set_output(&json!({"answer": "Rust is a systems programming language."}));
    agent.end();

    println!("  Agent trace_id: {}", agent.trace_id());
}

/// 2. Score spans using both the simple API and the ScoreBody builder.
fn example_span_scoring() -> Result<(), Box<dyn std::error::Error>> {
    let span = LangfuseSpan::start("scored-operation");
    span.set_input(&json!({"prompt": "Summarize this document"}));
    span.set_output(&json!({"summary": "A concise summary."}));

    // Simple score
    span.score("relevance", ScoreValue::Numeric(0.95))?;

    // Rich score via builder
    let body = ScoreBody::builder("quality", ScoreValue::Categorical("excellent".into()))
        .comment("High quality summary with good coverage")
        .metadata(json!({"evaluator": "human", "round": 1}))
        .build();
    span.score_with(body)?;

    // Boolean score
    span.score("factual", ScoreValue::Boolean(true))?;

    span.end();
    println!("  Scored span: {}", span.span_id());
    Ok(())
}

/// 3. Use context-aware APIs to update the current span without explicit handles.
fn example_context_apis() -> Result<(), Box<dyn std::error::Error>> {
    let span = LangfuseSpan::start("context-aware-demo");
    span.set_input(&json!({"step": "initial"}));

    // Batch-update via SpanUpdateParams
    span.update(SpanUpdateParams {
        output: Some(json!({"result": "processed"})),
        metadata: Some(json!({"version": "2.0"})),
        status_message: Some("completed successfully".into()),
        ..Default::default()
    });

    // Score the current trace (not a specific observation)
    langfuse::score_current_trace("overall-quality", ScoreValue::Numeric(0.9))?;

    // Get the trace URL for logging/debugging
    let url = langfuse::get_current_trace_url()?;
    println!("  Trace URL: {}", url);

    span.end();
    Ok(())
}

/// 4. Use named instances for multi-project or multi-environment setups.
fn example_named_instances() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize a named instance (reuses same config for demo purposes).
    let config = LangfuseConfig::from_env()?;
    Langfuse::init_named("staging", config)?;

    // Retrieve the named instance by name.
    if let Some(staging) = Langfuse::get_named("staging") {
        let span = staging.start_span("staging-operation");
        span.set_input(&json!({"env": "staging"}));
        span.end();
        println!("  Named instance 'staging' span: {}", span.span_id());
    }

    // try_get_named returns a Result for error handling.
    let _prod = Langfuse::try_get_named("production");
    println!("  'production' instance exists: {}", _prod.is_ok());

    Ok(())
}

/// 5. Use ObservingIterator to automatically collect output from an iterator.
fn example_observing_iterator() {
    let span = LangfuseSpan::start("iterator-demo");
    span.set_input(&json!({"source": "token stream"}));

    // Simulate a stream of tokens from an LLM response.
    let tokens = vec!["Hello", ", ", "world", "!"];
    let observed = ObservingIterator::new(span, tokens.into_iter());

    // Consume the iterator — each item is serialized and collected.
    // When exhausted, the collected output is set on the span and it is ended.
    let result: String = observed.collect();
    println!("  Collected output: {}", result);
}
