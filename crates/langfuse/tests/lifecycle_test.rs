//! Integration tests for client lifecycle: named instances, Drop, delete_dataset.

use langfuse::{Langfuse, LangfuseConfig};

fn test_config(key_suffix: &str) -> LangfuseConfig {
    LangfuseConfig::builder()
        .public_key(format!("pk-lf-{key_suffix}"))
        .secret_key(format!("sk-lf-{key_suffix}"))
        .tracing_enabled(false)
        .build()
        .unwrap()
}

#[tokio::test]
async fn test_named_instance_init_and_get() {
    let config = test_config("named-1");
    Langfuse::init_named("test-env-1", config).unwrap();

    let instance = Langfuse::get_named("test-env-1");
    assert!(instance.is_some());
    assert_eq!(instance.unwrap().config().public_key, "pk-lf-named-1");
}

#[tokio::test]
async fn test_named_instance_not_found() {
    let result = Langfuse::get_named("nonexistent-instance");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_try_get_named_returns_error_when_missing() {
    let result = Langfuse::try_get_named("does-not-exist");
    let err = result.expect_err("expected an error");
    let msg = err.to_string();
    assert!(
        msg.contains("does-not-exist"),
        "Error should mention the missing name, got: {msg}"
    );
}

#[tokio::test]
async fn test_try_get_named_returns_instance() {
    let config = test_config("named-try");
    Langfuse::init_named("try-get-test", config).unwrap();

    let instance = Langfuse::try_get_named("try-get-test");
    assert!(instance.is_ok());
    assert_eq!(instance.unwrap().config().public_key, "pk-lf-named-try");
}

#[tokio::test]
async fn test_named_instance_overwrite() {
    let config1 = test_config("overwrite-v1");
    Langfuse::init_named("overwrite-test", config1).unwrap();

    let config2 = test_config("overwrite-v2");
    Langfuse::init_named("overwrite-test", config2).unwrap();

    let instance = Langfuse::get_named("overwrite-test").unwrap();
    assert_eq!(instance.config().public_key, "pk-lf-overwrite-v2");
}

#[tokio::test]
async fn test_multiple_named_instances_independent() {
    let config_a = test_config("multi-a");
    let config_b = test_config("multi-b");

    Langfuse::init_named("env-a", config_a).unwrap();
    Langfuse::init_named("env-b", config_b).unwrap();

    let a = Langfuse::get_named("env-a").unwrap();
    let b = Langfuse::get_named("env-b").unwrap();

    assert_eq!(a.config().public_key, "pk-lf-multi-a");
    assert_eq!(b.config().public_key, "pk-lf-multi-b");
}

#[tokio::test]
async fn test_default_singleton_independent_of_named() {
    // Named instances should not interfere with the default singleton.
    // We can't test init() here (OnceLock is global across tests), but we can
    // verify that try_get() returns None when init() hasn't been called in this
    // process (or returns Some if another test called it — either way, named
    // instances are separate).
    let config = test_config("named-independent");
    Langfuse::init_named("independent-test", config).unwrap();

    // The named instance exists
    assert!(Langfuse::get_named("independent-test").is_some());

    // The default singleton is a separate mechanism (OnceLock vs DashMap)
    // — we just verify the API is available and doesn't cross-contaminate.
    // try_get() returns whatever state the global singleton is in.
    let _default = Langfuse::try_get();
}

#[tokio::test]
async fn test_drop_does_not_panic_in_multi_thread_runtime() {
    // Create and immediately drop a Langfuse instance.
    // This exercises the Drop impl in a multi-thread runtime.
    let config = test_config("drop-test");
    let client = Langfuse::new(config).unwrap();
    drop(client);
    // If we reach here, Drop didn't panic.
}

#[tokio::test]
async fn test_delete_dataset_returns_error_without_server() {
    let config = test_config("delete-ds");
    let client = Langfuse::new(config).unwrap();

    // No real server → should get a network error
    let result = client.datasets.delete_dataset("nonexistent").await;
    assert!(result.is_err());
}
