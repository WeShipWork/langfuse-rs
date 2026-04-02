use langfuse::{LangfuseSpan, ScoreBody, ScoreDataType, ScoreValue};

#[test]
fn test_score_body_builder_required_fields_only() {
    let body = ScoreBody::builder("accuracy", ScoreValue::Numeric(0.95)).build();

    assert_eq!(body.name, "accuracy");
    assert_eq!(body.value, ScoreValue::Numeric(0.95));
    assert!(body.trace_id.is_none());
    assert!(body.observation_id.is_none());
    assert!(body.comment.is_none());
    assert!(body.metadata.is_none());
    assert!(body.config_id.is_none());
    assert!(body.data_type.is_none());
}

#[test]
fn test_score_body_builder_all_optional_fields() {
    let metadata = serde_json::json!({"source": "test"});
    let body = ScoreBody::builder("quality", ScoreValue::Boolean(true))
        .trace_id("trace-123")
        .observation_id("obs-456")
        .comment("Excellent quality")
        .metadata(metadata.clone())
        .config_id("cfg-789")
        .data_type(ScoreDataType::Boolean)
        .build();

    assert_eq!(body.name, "quality");
    assert_eq!(body.value, ScoreValue::Boolean(true));
    assert_eq!(body.trace_id.as_deref(), Some("trace-123"));
    assert_eq!(body.observation_id.as_deref(), Some("obs-456"));
    assert_eq!(body.comment.as_deref(), Some("Excellent quality"));
    assert_eq!(body.metadata, Some(metadata));
    assert_eq!(body.config_id.as_deref(), Some("cfg-789"));
    assert_eq!(body.data_type, Some(ScoreDataType::Boolean));
}

#[test]
fn test_score_body_builder_categorical_value() {
    let body = ScoreBody::builder("sentiment", ScoreValue::Categorical("positive".into())).build();

    assert_eq!(body.name, "sentiment");
    assert_eq!(body.value, ScoreValue::Categorical("positive".to_string()));
}

#[tokio::test]
async fn test_span_score_returns_error_without_singleton() {
    // Without Langfuse::init(), scoring methods should return an error
    // because the global singleton is not available.
    let span = LangfuseSpan::start("test-span");
    let result = span.score("quality", ScoreValue::Numeric(0.9));
    assert!(result.is_err());
    span.end();
}

#[tokio::test]
async fn test_span_score_with_returns_error_without_singleton() {
    let span = LangfuseSpan::start("test-span");
    let body = ScoreBody::builder("quality", ScoreValue::Numeric(0.9))
        .comment("test comment")
        .build();
    let result = span.score_with(body);
    assert!(result.is_err());
    span.end();
}

#[tokio::test]
async fn test_span_score_trace_returns_error_without_singleton() {
    let span = LangfuseSpan::start("test-span");
    let result = span.score_trace("accuracy", ScoreValue::Numeric(0.8));
    assert!(result.is_err());
    span.end();
}
