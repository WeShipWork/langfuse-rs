use langfuse::{Langfuse, LangfuseConfig, ScoreValue};

#[tokio::test]
async fn test_score_within_span() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .tracing_enabled(false) // no real OTel
        .build()
        .unwrap();
    let client = Langfuse::new(config).unwrap();

    // Create a span and score it
    let span = client.start_span("test-span");
    let trace_id = span.trace_id();
    let span_id = span.span_id();

    client
        .scores
        .score_observation(&trace_id, &span_id, "quality", ScoreValue::Numeric(0.95));

    assert_eq!(client.scores.pending_count(), 1);
    span.end();
}

#[tokio::test]
async fn test_score_trace() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .tracing_enabled(false)
        .build()
        .unwrap();
    let client = Langfuse::new(config).unwrap();

    let span = client.start_span("my-trace");
    client
        .scores
        .score_trace(&span.trace_id(), "accuracy", ScoreValue::Numeric(0.9));
    assert_eq!(client.scores.pending_count(), 1);
    span.end();
}
