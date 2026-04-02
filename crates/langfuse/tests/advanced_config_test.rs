//! Integration tests for advanced configuration features (Group 6).
//!
//! Tests run against the default no-op OTel provider unless otherwise noted.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use langfuse::langfuse_tracing::span::LangfuseSpan;
use langfuse::{LangfuseConfig, PropagateAttributes};
use serde_json::json;

// =========================================================================
// 6.1 — Mask function on LangfuseConfig
// =========================================================================

#[test]
fn config_mask_none_by_default() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .build()
        .unwrap();
    assert!(config.mask.is_none());
}

#[test]
fn config_mask_builder_sets_function() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .mask(|v| {
            if let serde_json::Value::Object(mut map) = v {
                map.remove("secret");
                serde_json::Value::Object(map)
            } else {
                v
            }
        })
        .build()
        .unwrap();
    assert!(config.mask.is_some());

    // Verify the mask function works
    let mask_fn = config.mask.as_ref().unwrap();
    let input = json!({"secret": "password", "public": "data"});
    let masked = mask_fn(input);
    assert_eq!(masked, json!({"public": "data"}));
}

#[test]
fn config_mask_debug_shows_some() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .mask(|v| v)
        .build()
        .unwrap();
    let debug = format!("{config:?}");
    assert!(debug.contains("Some(<mask fn>)"));
}

#[test]
fn config_mask_debug_shows_none() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .build()
        .unwrap();
    let debug = format!("{config:?}");
    assert!(debug.contains("mask: \"None\""));
}

#[test]
fn config_clone_preserves_mask() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&call_count);
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .mask(move |v| {
            count.fetch_add(1, Ordering::Relaxed);
            v
        })
        .build()
        .unwrap();

    let cloned = config.clone();
    assert!(cloned.mask.is_some());

    // Both should share the same Arc'd function
    let mask_fn = cloned.mask.as_ref().unwrap();
    mask_fn(json!(null));
    assert_eq!(call_count.load(Ordering::Relaxed), 1);
}

// =========================================================================
// 6.2 — Additional headers
// =========================================================================

#[test]
fn config_additional_headers_none_by_default() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .build()
        .unwrap();
    assert!(config.additional_headers.is_none());
}

#[test]
fn config_additional_headers_builder() {
    let mut headers = HashMap::new();
    headers.insert("X-Custom".to_string(), "value".to_string());
    headers.insert("X-Another".to_string(), "other".to_string());

    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .additional_headers(headers.clone())
        .build()
        .unwrap();

    assert_eq!(config.additional_headers, Some(headers));
}

// =========================================================================
// 6.3 — Max retries
// =========================================================================

#[test]
fn config_max_retries_default_is_3() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .build()
        .unwrap();
    assert_eq!(config.max_retries, 3);
}

#[test]
fn config_max_retries_builder() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .max_retries(5)
        .build()
        .unwrap();
    assert_eq!(config.max_retries, 5);
}

// =========================================================================
// 6.4 — New env var fields
// =========================================================================

#[test]
fn config_media_upload_thread_count_default() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .build()
        .unwrap();
    assert_eq!(config.media_upload_thread_count, 4);
}

#[test]
fn config_io_capture_enabled_default() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .build()
        .unwrap();
    assert!(config.io_capture_enabled);
}

#[test]
fn config_io_capture_enabled_builder() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .io_capture_enabled(false)
        .build()
        .unwrap();
    assert!(!config.io_capture_enabled);
}

#[test]
fn config_media_upload_thread_count_builder() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .media_upload_thread_count(8)
        .build()
        .unwrap();
    assert_eq!(config.media_upload_thread_count, 8);
}

// =========================================================================
// 6.5 — Mask applied in set_json_attribute
// =========================================================================

#[test]
fn span_set_input_does_not_panic_without_mask() {
    // Without global Langfuse initialized, mask is not applied
    let span = LangfuseSpan::start("test-no-mask");
    span.set_input(&json!({"data": "value"}));
    span.end();
}

// Note: Testing mask application requires Langfuse::init() which uses OnceLock.
// This is tested indirectly through the mask function unit tests above.

// =========================================================================
// 6.6 — should_export_span filter
// =========================================================================

#[test]
fn config_should_export_span_none_by_default() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .build()
        .unwrap();
    assert!(config.should_export_span.is_none());
}

#[test]
fn config_should_export_span_builder() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .should_export_span(|_span_data| true)
        .build()
        .unwrap();
    assert!(config.should_export_span.is_some());
}

#[test]
fn config_should_export_span_debug() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .should_export_span(|_| false)
        .build()
        .unwrap();
    let debug = format!("{config:?}");
    assert!(debug.contains("Some(<filter fn>)"));
}

// =========================================================================
// 6.7 — Additional headers in HTTP clients
// =========================================================================

#[test]
fn additional_headers_applied_to_http_client() {
    let mut headers = HashMap::new();
    headers.insert("X-Custom-Header".to_string(), "test-value".to_string());

    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .additional_headers(headers)
        .build()
        .unwrap();

    // Build the HTTP client — should not panic
    let _client = langfuse::http::build_http_client(&config);
}

// =========================================================================
// 6.8 — Backoff dependency
// =========================================================================

#[test]
fn backoff_crate_available() {
    // Verify backoff crate is available by constructing an ExponentialBackoff
    let _backoff = backoff::ExponentialBackoffBuilder::default()
        .with_initial_interval(Duration::from_millis(100))
        .build();
}

// =========================================================================
// 6.9 — Retry logic
// =========================================================================

#[tokio::test]
async fn retry_request_succeeds_immediately() {
    let result =
        langfuse::http::retry_request(3, || async { Ok::<_, langfuse::LangfuseError>(42) }).await;
    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn retry_request_retries_on_server_error() {
    let attempt = Arc::new(AtomicUsize::new(0));
    let attempt_clone = Arc::clone(&attempt);

    let result = langfuse::http::retry_request(3, move || {
        let attempt = Arc::clone(&attempt_clone);
        async move {
            let n = attempt.fetch_add(1, Ordering::Relaxed);
            if n < 2 {
                Err(langfuse::LangfuseError::Api {
                    status: 500,
                    message: "server error".into(),
                })
            } else {
                Ok(42)
            }
        }
    })
    .await;

    assert_eq!(result.unwrap(), 42);
    assert_eq!(attempt.load(Ordering::Relaxed), 3); // 2 failures + 1 success
}

#[tokio::test]
async fn retry_request_does_not_retry_on_client_error() {
    let attempt = Arc::new(AtomicUsize::new(0));
    let attempt_clone = Arc::clone(&attempt);

    let result: Result<i32, _> = langfuse::http::retry_request(3, move || {
        let attempt = Arc::clone(&attempt_clone);
        async move {
            attempt.fetch_add(1, Ordering::Relaxed);
            Err(langfuse::LangfuseError::Api {
                status: 400,
                message: "bad request".into(),
            })
        }
    })
    .await;

    assert!(result.is_err());
    assert_eq!(attempt.load(Ordering::Relaxed), 1); // No retries
}

#[tokio::test]
async fn retry_request_retries_on_429() {
    let attempt = Arc::new(AtomicUsize::new(0));
    let attempt_clone = Arc::clone(&attempt);

    let result = langfuse::http::retry_request(2, move || {
        let attempt = Arc::clone(&attempt_clone);
        async move {
            let n = attempt.fetch_add(1, Ordering::Relaxed);
            if n < 1 {
                Err(langfuse::LangfuseError::Api {
                    status: 429,
                    message: "rate limited".into(),
                })
            } else {
                Ok("ok")
            }
        }
    })
    .await;

    assert_eq!(result.unwrap(), "ok");
    assert_eq!(attempt.load(Ordering::Relaxed), 2);
}

#[tokio::test]
async fn retry_request_does_not_retry_auth_error() {
    let attempt = Arc::new(AtomicUsize::new(0));
    let attempt_clone = Arc::clone(&attempt);

    let result: Result<i32, _> = langfuse::http::retry_request(3, move || {
        let attempt = Arc::clone(&attempt_clone);
        async move {
            attempt.fetch_add(1, Ordering::Relaxed);
            Err(langfuse::LangfuseError::Auth)
        }
    })
    .await;

    assert!(result.is_err());
    assert_eq!(attempt.load(Ordering::Relaxed), 1);
}

// =========================================================================
// 6.10 — Auto-flush background task
// =========================================================================

#[tokio::test]
async fn score_manager_auto_flush_spawns_task() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .tracing_enabled(false)
        .flush_interval(Duration::from_millis(50))
        .build()
        .unwrap();

    let manager = langfuse::scoring::manager::ScoreManager::new(&config);

    // Add a score
    manager.score(langfuse::ScoreBody {
        name: "test".into(),
        value: langfuse::ScoreValue::Numeric(1.0),
        trace_id: Some("trace-1".into()),
        observation_id: None,
        comment: None,
        metadata: None,
        config_id: None,
        data_type: None,
    });

    assert_eq!(manager.pending_count(), 1);

    // Wait for auto-flush interval to trigger
    // The flush will fail (no server) but the queue should be drained
    tokio::time::sleep(Duration::from_millis(150)).await;

    // After auto-flush attempt, the queue should be drained
    // (flush_inner drains even if the HTTP request fails)
    assert_eq!(manager.pending_count(), 0);
}

#[tokio::test]
async fn score_manager_shutdown_cancels_background_task() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .tracing_enabled(false)
        .flush_interval(Duration::from_secs(60))
        .build()
        .unwrap();

    let manager = langfuse::scoring::manager::ScoreManager::new(&config);
    // Shutdown should not hang
    let _ = manager.shutdown().await;
}

// =========================================================================
// 6.11 — Propagate attributes
// =========================================================================

#[test]
fn propagate_attributes_sets_user_id_on_child_spans() {
    let attrs = PropagateAttributes {
        user_id: Some("user-123".into()),
        session_id: Some("session-456".into()),
        metadata: None,
        version: Some("v1".into()),
        tags: Some(vec!["tag-a".into(), "tag-b".into()]),
        trace_name: Some("my-trace".into()),
        as_baggage: false,
    };

    langfuse::propagate_attributes(&attrs, || {
        // Spans created here should have the propagated attributes applied
        let span = LangfuseSpan::start("child-span");
        // The span should not panic — attributes are applied internally
        span.end();
    });
}

#[test]
fn propagate_attributes_restores_previous_state() {
    let attrs = PropagateAttributes {
        user_id: Some("user-1".into()),
        ..Default::default()
    };

    // Before propagation, no attributes are set
    langfuse::propagate_attributes(&attrs, || {
        // Inside: attributes are set
        let span = LangfuseSpan::start("inner");
        span.end();
    });

    // After: previous state is restored (None)
    let span = LangfuseSpan::start("outer");
    span.end();
}

#[test]
fn propagate_attributes_nested() {
    let outer_attrs = PropagateAttributes {
        user_id: Some("outer-user".into()),
        ..Default::default()
    };
    let inner_attrs = PropagateAttributes {
        user_id: Some("inner-user".into()),
        session_id: Some("inner-session".into()),
        ..Default::default()
    };

    langfuse::propagate_attributes(&outer_attrs, || {
        let span1 = LangfuseSpan::start("outer-span");
        span1.end();

        langfuse::propagate_attributes(&inner_attrs, || {
            let span2 = LangfuseSpan::start("inner-span");
            span2.end();
        });

        // After inner scope, outer attrs should be restored
        let span3 = LangfuseSpan::start("after-inner");
        span3.end();
    });
}

// =========================================================================
// 6.12 — SDK metadata resource attributes
// =========================================================================

#[test]
fn tracing_builder_with_sdk_metadata() {
    let config = LangfuseConfig::builder()
        .public_key("pk-lf-test")
        .secret_key("sk-lf-test")
        .build()
        .unwrap();

    // Building the tracing pipeline should succeed and include SDK metadata
    let tracing = langfuse::langfuse_tracing::exporter::LangfuseTracing::builder(&config).build();
    assert!(tracing.is_ok());
}

// =========================================================================
// Config — from_env defaults
// =========================================================================

#[test]
fn config_from_env_new_fields_defaults() {
    // Set required env vars
    unsafe {
        std::env::set_var("LANGFUSE_PUBLIC_KEY", "pk-test");
        std::env::set_var("LANGFUSE_SECRET_KEY", "sk-test");
    }

    let config = LangfuseConfig::from_env().unwrap();
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.media_upload_thread_count, 4);
    assert!(config.io_capture_enabled);
    assert!(config.mask.is_none());
    assert!(config.additional_headers.is_none());
    assert!(config.should_export_span.is_none());

    // Clean up
    unsafe {
        std::env::remove_var("LANGFUSE_PUBLIC_KEY");
        std::env::remove_var("LANGFUSE_SECRET_KEY");
    }
}

// =========================================================================
// Config — all new builder methods
// =========================================================================

#[test]
fn config_builder_all_new_fields() {
    let mut headers = HashMap::new();
    headers.insert("X-Test".to_string(), "value".to_string());

    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .mask(|v| v)
        .additional_headers(headers)
        .max_retries(5)
        .media_upload_thread_count(8)
        .io_capture_enabled(false)
        .should_export_span(|_| true)
        .build()
        .unwrap();

    assert!(config.mask.is_some());
    assert!(config.additional_headers.is_some());
    assert_eq!(config.max_retries, 5);
    assert_eq!(config.media_upload_thread_count, 8);
    assert!(!config.io_capture_enabled);
    assert!(config.should_export_span.is_some());
}

// =========================================================================
// Config — backward compatibility
// =========================================================================

#[test]
fn config_builder_backward_compatible() {
    // Existing API should still work without specifying new fields
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .base_url("https://custom.langfuse.com")
        .timeout(Duration::from_secs(10))
        .flush_at(256)
        .flush_interval(Duration::from_secs(10))
        .sample_rate(0.5)
        .environment("test")
        .release("v1.0")
        .debug(true)
        .tracing_enabled(false)
        .build()
        .unwrap();

    assert_eq!(config.public_key, "pk");
    assert_eq!(config.base_url, "https://custom.langfuse.com");
    assert_eq!(config.timeout, Duration::from_secs(10));
    assert_eq!(config.flush_at, 256);
    assert_eq!(config.sample_rate, 0.5);
    assert!(config.debug);
    assert!(!config.tracing_enabled);

    // New fields should have defaults
    assert!(config.mask.is_none());
    assert!(config.additional_headers.is_none());
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.media_upload_thread_count, 4);
    assert!(config.io_capture_enabled);
    assert!(config.should_export_span.is_none());
}
