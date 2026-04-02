//! Basic tracing example showing span creation, nesting, and the #[observe] macro.
//!
//! This example demonstrates:
//! - Creating root spans and nested generations
//! - Setting input/output and metadata
//! - Using the closure-based API (with_span, with_generation)
//! - Using the #[observe] macro for automatic instrumentation
//!
//! Run: LANGFUSE_PUBLIC_KEY=pk-... LANGFUSE_SECRET_KEY=sk-... cargo run --example basic_tracing

use langfuse::{Langfuse, LangfuseConfig, observe, with_generation, with_span};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Langfuse from environment variables
    let config = LangfuseConfig::from_env()?;
    let langfuse = Langfuse::new(config)?;

    println!("=== Example 1: Manual span creation ===");
    example_manual_span(&langfuse).await;

    println!("\n=== Example 2: Closure-based API ===");
    example_closure_api().await;

    println!("\n=== Example 3: Decorated function ===");
    example_decorated_function().await;

    println!("\nAll examples completed successfully!");
    Ok(())
}

/// Example 1: Manually create spans and generations with explicit lifecycle management.
async fn example_manual_span(_langfuse: &Langfuse) {
    // Create a root span
    let span = langfuse::LangfuseSpan::start("my-pipeline");
    span.set_input(&json!({"query": "What is Rust?"}));
    span.set_trace_user_id("user-123");
    span.set_trace_session_id("session-456");
    span.set_metadata(&json!({"environment": "example"}));

    // Create a nested generation
    let r#gen = span.start_generation("llm-call");
    r#gen.set_model("gpt-4o");
    r#gen.set_input(&json!({
        "messages": [
            {"role": "user", "content": "What is Rust?"}
        ]
    }));
    r#gen.set_output(&json!({
        "content": "Rust is a systems programming language focused on safety, speed, and concurrency."
    }));

    // Set token usage
    r#gen.set_usage(&langfuse::UsageDetails {
        input: Some(15),
        output: Some(50),
        total: Some(65),
    });

    r#gen.end();

    // Set span output and end
    span.set_output(&json!({
        "answer": "Rust is a systems programming language focused on safety, speed, and concurrency."
    }));
    span.end();

    println!("Manual span created with trace_id: {}", span.trace_id());
}

/// Example 2: Use the closure-based API for cleaner, scoped tracing.
async fn example_closure_api() {
    // with_span creates a span, passes it to the closure, and ends it automatically
    let result = with_span("closure-example", |span| async move {
        span.set_input(&json!({"input": "hello"}));

        // Nested generation within the span
        let answer = with_generation("nested-gen", |r#gen| async move {
            r#gen.set_model("gpt-4o-mini");
            r#gen.set_input(&json!({"prompt": "Say hello"}));
            "Hello, world!"
        })
        .await;

        span.set_output(&json!({"result": answer}));
        answer
    })
    .await;

    println!("Closure-based result: {}", result);
}

/// Example 3: Use the #[observe] macro for automatic instrumentation.
#[observe(as_type = "generation", name = "process-request")]
async fn example_decorated_function() {
    let query = "What is Rust?";
    let answer = process_request(query).await;
    println!("Decorated function result: {}", answer);
}

/// A simple async function that will be instrumented by the #[observe] macro.
#[observe(as_type = "generation", name = "llm-call")]
async fn process_request(query: &str) -> String {
    format!("Answer to: {}", query)
}
