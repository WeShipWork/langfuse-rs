//! Tests for the `#[observe]` proc macro.
//!
//! These tests run against the default no-op OTel provider — no real exporter
//! is configured, so spans are silently discarded.  The goal is to verify that
//! the macro expansion compiles and the decorated functions return correct values.

use langfuse::observe;

// =========================================================================
// Decorated functions
// =========================================================================

#[observe]
async fn simple_async(input: String) -> String {
    format!("hello {}", input)
}

#[observe(name = "custom-name")]
async fn custom_named(x: i32) -> i32 {
    x + 1
}

#[observe(as_type = "generation")]
async fn llm_call(prompt: String) -> String {
    "response".to_string()
}

#[observe(capture_input = false, capture_output = false)]
async fn no_capture(data: String) -> String {
    data
}

#[observe]
fn sync_function(x: i32) -> i32 {
    x * 2
}

#[observe(as_type = "generation")]
fn sync_generation(prompt: String) -> String {
    format!("answer to {}", prompt)
}

#[observe(name = "custom-sync", capture_input = false)]
fn sync_no_input(x: i32) -> i32 {
    x + 10
}

// --- New observation type macro expansions ---

#[observe(as_type = "agent")]
async fn agent_fn(input: String) -> String {
    format!("agent processed {input}")
}

#[observe(as_type = "tool")]
async fn tool_fn(query: String) -> String {
    format!("tool result for {query}")
}

#[observe(as_type = "chain")]
fn sync_chain(x: i32) -> i32 {
    x + 100
}

#[observe(as_type = "retriever")]
async fn retriever_fn(query: String) -> Vec<String> {
    vec![format!("doc about {query}")]
}

#[observe(as_type = "evaluator")]
fn evaluator_fn(score: f64) -> bool {
    score > 0.5
}

#[observe(as_type = "guardrail")]
async fn guardrail_fn(text: String) -> bool {
    !text.is_empty()
}

#[observe(as_type = "embedding")]
async fn embedding_fn(text: String) -> Vec<f32> {
    vec![0.1, 0.2, 0.3]
}

#[observe(as_type = "event")]
fn event_fn(msg: String) -> String {
    format!("event: {msg}")
}

// =========================================================================
// Tests
// =========================================================================

#[tokio::test]
async fn test_observe_async() {
    let result = simple_async("world".to_string()).await;
    assert_eq!(result, "hello world");
}

#[tokio::test]
async fn test_observe_custom_name() {
    let result = custom_named(5).await;
    assert_eq!(result, 6);
}

#[tokio::test]
async fn test_observe_generation() {
    let result = llm_call("test".to_string()).await;
    assert_eq!(result, "response");
}

#[tokio::test]
async fn test_observe_no_capture() {
    let result = no_capture("data".to_string()).await;
    assert_eq!(result, "data");
}

#[test]
fn test_observe_sync() {
    let result = sync_function(21);
    assert_eq!(result, 42);
}

#[test]
fn test_observe_sync_generation() {
    let result = sync_generation("life".to_string());
    assert_eq!(result, "answer to life");
}

#[test]
fn test_observe_sync_no_input() {
    let result = sync_no_input(5);
    assert_eq!(result, 15);
}

// --- Tests for new observation type macro expansions ---

#[tokio::test]
async fn test_observe_as_agent() {
    let result = agent_fn("request".to_string()).await;
    assert_eq!(result, "agent processed request");
}

#[tokio::test]
async fn test_observe_as_tool() {
    let result = tool_fn("search".to_string()).await;
    assert_eq!(result, "tool result for search");
}

#[test]
fn test_observe_as_chain() {
    let result = sync_chain(42);
    assert_eq!(result, 142);
}

#[tokio::test]
async fn test_observe_as_retriever() {
    let result = retriever_fn("rust".to_string()).await;
    assert_eq!(result, vec!["doc about rust".to_string()]);
}

#[test]
fn test_observe_as_evaluator() {
    assert!(evaluator_fn(0.8));
    assert!(!evaluator_fn(0.3));
}

#[tokio::test]
async fn test_observe_as_guardrail() {
    assert!(guardrail_fn("hello".to_string()).await);
    assert!(!guardrail_fn(String::new()).await);
}

#[tokio::test]
async fn test_observe_as_embedding() {
    let result = embedding_fn("hello".to_string()).await;
    assert_eq!(result, vec![0.1, 0.2, 0.3]);
}

#[test]
fn test_observe_as_event() {
    let result = event_fn("click".to_string());
    assert_eq!(result, "event: click");
}
