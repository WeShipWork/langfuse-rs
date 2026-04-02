use langfuse_core::config::LangfuseConfig;

#[test]
fn test_langfuse_tracing_builder_creates_provider() {
    let config = LangfuseConfig::builder()
        .public_key("pk-lf-test")
        .secret_key("sk-lf-test")
        .build()
        .unwrap();

    // Build should succeed (creates exporter + processor + provider)
    let tracing = langfuse::langfuse_tracing::exporter::LangfuseTracing::builder(&config).build();
    assert!(tracing.is_ok());
}

#[test]
fn test_langfuse_tracing_builder_custom_endpoint() {
    let config = LangfuseConfig::builder()
        .public_key("pk-lf-test")
        .secret_key("sk-lf-test")
        .base_url("https://custom.langfuse.com")
        .build()
        .unwrap();

    let tracing = langfuse::langfuse_tracing::exporter::LangfuseTracing::builder(&config).build();
    assert!(tracing.is_ok());
}

#[test]
fn test_attributes_constants_are_correct() {
    use langfuse::langfuse_tracing::attributes::*;
    assert_eq!(LANGFUSE_OBSERVATION_TYPE, "langfuse.observation.type");
    assert_eq!(LANGFUSE_INPUT, "langfuse.observation.input");
    assert_eq!(LANGFUSE_OUTPUT, "langfuse.observation.output");
    assert_eq!(LANGFUSE_MODEL, "langfuse.observation.model.name");
    assert_eq!(LANGFUSE_USER_ID, "langfuse.user.id");
    assert_eq!(LANGFUSE_SESSION_ID, "langfuse.session.id");
}
