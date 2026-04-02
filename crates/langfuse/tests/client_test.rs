use langfuse::{Langfuse, LangfuseConfig};

#[tokio::test]
async fn test_client_creation() {
    let config = LangfuseConfig::builder()
        .public_key("pk-lf-test")
        .secret_key("sk-lf-test")
        .tracing_enabled(false)
        .build()
        .unwrap();
    let client = Langfuse::new(config);
    assert!(client.is_ok());
}

#[tokio::test]
async fn test_client_tracing_disabled() {
    let config = LangfuseConfig::builder()
        .public_key("pk-lf-test")
        .secret_key("sk-lf-test")
        .tracing_enabled(false)
        .build()
        .unwrap();
    let client = Langfuse::new(config).unwrap();
    // Should not panic — tracing is disabled
    let span = client.start_span("test");
    span.end();
}

#[tokio::test]
async fn test_client_config_access() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .tracing_enabled(false)
        .build()
        .unwrap();
    let client = Langfuse::new(config).unwrap();
    assert_eq!(client.config().public_key, "pk");
}

// Note: can't test singleton in unit tests (OnceLock is global, would interfere between tests)
// Integration tests would cover Langfuse::init() and Langfuse::get()
