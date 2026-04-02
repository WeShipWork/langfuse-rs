//! Configuration for the Langfuse SDK.

use crate::error::ConfigError;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// A mask function that transforms JSON values before they are stored as span attributes.
pub type MaskFn = Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync>;

/// A span export filter function. Returns `true` if the span should be exported.
pub type SpanFilterFn = Arc<dyn Fn(&opentelemetry_sdk::trace::SpanData) -> bool + Send + Sync>;

/// Configuration for the Langfuse client.
///
/// Supports construction via the builder pattern ([`LangfuseConfig::builder`])
/// or from environment variables ([`LangfuseConfig::from_env`]).
pub struct LangfuseConfig {
    /// Langfuse public key (required).
    pub public_key: String,
    /// Langfuse secret key (required).
    pub secret_key: String,
    /// Langfuse API base URL. Defaults to `https://cloud.langfuse.com`.
    pub base_url: String,
    /// HTTP request timeout. Defaults to 5 seconds.
    pub timeout: Duration,
    /// Number of events to buffer before flushing. Defaults to 512.
    pub flush_at: usize,
    /// Maximum interval between flushes. Defaults to 5 seconds.
    pub flush_interval: Duration,
    /// Sampling rate for traces (0.0–1.0). Defaults to 1.0 (100%).
    pub sample_rate: f64,
    /// Optional environment tag for traces.
    pub environment: Option<String>,
    /// Optional release tag for traces.
    pub release: Option<String>,
    /// Enable debug logging. Defaults to `false`.
    pub debug: bool,
    /// Enable tracing. Defaults to `true`.
    pub tracing_enabled: bool,
    /// Optional mask function applied to JSON values before storing as span attributes.
    pub mask: Option<MaskFn>,
    /// Optional additional HTTP headers sent with every request.
    pub additional_headers: Option<HashMap<String, String>>,
    /// Maximum number of retries for HTTP requests. Defaults to 3.
    pub max_retries: usize,
    /// Number of threads for media uploads. Defaults to 4.
    pub media_upload_thread_count: usize,
    /// Whether I/O capture is enabled for the observe decorator. Defaults to `true`.
    pub io_capture_enabled: bool,
    /// Optional filter function to decide whether a span should be exported.
    pub should_export_span: Option<SpanFilterFn>,
}

impl std::fmt::Debug for LangfuseConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LangfuseConfig")
            .field("public_key", &self.public_key)
            .field("secret_key", &"[REDACTED]")
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .field("flush_at", &self.flush_at)
            .field("flush_interval", &self.flush_interval)
            .field("sample_rate", &self.sample_rate)
            .field("environment", &self.environment)
            .field("release", &self.release)
            .field("debug", &self.debug)
            .field("tracing_enabled", &self.tracing_enabled)
            .field(
                "mask",
                if self.mask.is_some() {
                    &"Some(<mask fn>)"
                } else {
                    &"None"
                },
            )
            .field("additional_headers", &self.additional_headers)
            .field("max_retries", &self.max_retries)
            .field("media_upload_thread_count", &self.media_upload_thread_count)
            .field("io_capture_enabled", &self.io_capture_enabled)
            .field(
                "should_export_span",
                if self.should_export_span.is_some() {
                    &"Some(<filter fn>)"
                } else {
                    &"None"
                },
            )
            .finish()
    }
}

impl Clone for LangfuseConfig {
    fn clone(&self) -> Self {
        Self {
            public_key: self.public_key.clone(),
            secret_key: self.secret_key.clone(),
            base_url: self.base_url.clone(),
            timeout: self.timeout,
            flush_at: self.flush_at,
            flush_interval: self.flush_interval,
            sample_rate: self.sample_rate,
            environment: self.environment.clone(),
            release: self.release.clone(),
            debug: self.debug,
            tracing_enabled: self.tracing_enabled,
            mask: self.mask.clone(),
            additional_headers: self.additional_headers.clone(),
            max_retries: self.max_retries,
            media_upload_thread_count: self.media_upload_thread_count,
            io_capture_enabled: self.io_capture_enabled,
            should_export_span: self.should_export_span.clone(),
        }
    }
}

impl LangfuseConfig {
    /// Create a new builder for `LangfuseConfig`.
    #[must_use]
    pub fn builder() -> LangfuseConfigBuilder {
        LangfuseConfigBuilder::default()
    }

    /// Build configuration from environment variables.
    ///
    /// Required env vars: `LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY`.
    /// Optional: `LANGFUSE_BASE_URL`, `LANGFUSE_TIMEOUT`, `LANGFUSE_FLUSH_AT`,
    ///           `LANGFUSE_FLUSH_INTERVAL`, `LANGFUSE_SAMPLE_RATE`,
    ///           `LANGFUSE_TRACING_ENVIRONMENT`, `LANGFUSE_RELEASE`,
    ///           `LANGFUSE_DEBUG`, `LANGFUSE_TRACING_ENABLED`,
    ///           `LANGFUSE_MAX_RETRIES`, `LANGFUSE_MEDIA_UPLOAD_THREAD_COUNT`,
    ///           `LANGFUSE_OBSERVE_DECORATOR_IO_CAPTURE_ENABLED`.
    pub fn from_env() -> std::result::Result<Self, ConfigError> {
        let public_key =
            std::env::var("LANGFUSE_PUBLIC_KEY").map_err(|_| ConfigError::MissingField {
                field: "LANGFUSE_PUBLIC_KEY".into(),
            })?;
        let secret_key =
            std::env::var("LANGFUSE_SECRET_KEY").map_err(|_| ConfigError::MissingField {
                field: "LANGFUSE_SECRET_KEY".into(),
            })?;

        let base_url = std::env::var("LANGFUSE_BASE_URL")
            .unwrap_or_else(|_| "https://cloud.langfuse.com".into());
        let timeout = std::env::var("LANGFUSE_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(5));
        let flush_at = std::env::var("LANGFUSE_FLUSH_AT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(512);
        let flush_interval = std::env::var("LANGFUSE_FLUSH_INTERVAL")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(5));
        let sample_rate = std::env::var("LANGFUSE_SAMPLE_RATE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1.0);
        let environment = std::env::var("LANGFUSE_TRACING_ENVIRONMENT").ok();
        let release = std::env::var("LANGFUSE_RELEASE").ok();
        let debug = std::env::var("LANGFUSE_DEBUG")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);
        let tracing_enabled = std::env::var("LANGFUSE_TRACING_ENABLED")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(true);
        let max_retries = std::env::var("LANGFUSE_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);
        let media_upload_thread_count = std::env::var("LANGFUSE_MEDIA_UPLOAD_THREAD_COUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4);
        let io_capture_enabled = std::env::var("LANGFUSE_OBSERVE_DECORATOR_IO_CAPTURE_ENABLED")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(true);

        Ok(Self {
            public_key,
            secret_key,
            base_url,
            timeout,
            flush_at,
            flush_interval,
            sample_rate,
            environment,
            release,
            debug,
            tracing_enabled,
            mask: None,
            additional_headers: None,
            max_retries,
            media_upload_thread_count,
            io_capture_enabled,
            should_export_span: None,
        })
    }

    /// Generate the HTTP Basic Auth header value.
    pub fn basic_auth_header(&self) -> String {
        let credentials = format!("{}:{}", self.public_key, self.secret_key);
        format!("Basic {}", BASE64.encode(credentials.as_bytes()))
    }

    /// Get the OTLP traces endpoint URL.
    pub fn otel_endpoint(&self) -> String {
        format!(
            "{}/api/public/otel/v1/traces",
            self.base_url.trim_end_matches('/')
        )
    }

    /// Get the REST API base URL.
    pub fn api_base_url(&self) -> String {
        format!("{}/api/public", self.base_url.trim_end_matches('/'))
    }
}

/// Builder for [`LangfuseConfig`].
#[derive(Default)]
pub struct LangfuseConfigBuilder {
    public_key: Option<String>,
    secret_key: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
    flush_at: Option<usize>,
    flush_interval: Option<Duration>,
    sample_rate: Option<f64>,
    environment: Option<String>,
    release: Option<String>,
    debug: Option<bool>,
    tracing_enabled: Option<bool>,
    mask: Option<MaskFn>,
    additional_headers: Option<HashMap<String, String>>,
    max_retries: Option<usize>,
    media_upload_thread_count: Option<usize>,
    io_capture_enabled: Option<bool>,
    should_export_span: Option<SpanFilterFn>,
}

impl LangfuseConfigBuilder {
    /// Set the Langfuse public key (required).
    #[must_use]
    pub fn public_key(mut self, key: impl Into<String>) -> Self {
        self.public_key = Some(key.into());
        self
    }

    /// Set the Langfuse secret key (required).
    #[must_use]
    pub fn secret_key(mut self, key: impl Into<String>) -> Self {
        self.secret_key = Some(key.into());
        self
    }

    /// Set the Langfuse API base URL.
    #[must_use]
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the HTTP request timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the number of events to buffer before flushing.
    #[must_use]
    pub fn flush_at(mut self, count: usize) -> Self {
        self.flush_at = Some(count);
        self
    }

    /// Set the maximum interval between flushes.
    #[must_use]
    pub fn flush_interval(mut self, interval: Duration) -> Self {
        self.flush_interval = Some(interval);
        self
    }

    /// Set the sampling rate (0.0–1.0).
    #[must_use]
    pub fn sample_rate(mut self, rate: f64) -> Self {
        self.sample_rate = Some(rate);
        self
    }

    /// Set the environment tag.
    #[must_use]
    pub fn environment(mut self, env: impl Into<String>) -> Self {
        self.environment = Some(env.into());
        self
    }

    /// Set the release tag.
    #[must_use]
    pub fn release(mut self, release: impl Into<String>) -> Self {
        self.release = Some(release.into());
        self
    }

    /// Enable or disable debug logging.
    #[must_use]
    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = Some(debug);
        self
    }

    /// Enable or disable tracing.
    #[must_use]
    pub fn tracing_enabled(mut self, enabled: bool) -> Self {
        self.tracing_enabled = Some(enabled);
        self
    }

    /// Set a mask function to transform JSON values before storing as span attributes.
    #[must_use]
    pub fn mask(
        mut self,
        f: impl Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    ) -> Self {
        self.mask = Some(Arc::new(f));
        self
    }

    /// Set additional HTTP headers to include in all requests.
    #[must_use]
    pub fn additional_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.additional_headers = Some(headers);
        self
    }

    /// Set the maximum number of retries for HTTP requests.
    #[must_use]
    pub fn max_retries(mut self, retries: usize) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Set the number of threads for media uploads.
    #[must_use]
    pub fn media_upload_thread_count(mut self, count: usize) -> Self {
        self.media_upload_thread_count = Some(count);
        self
    }

    /// Enable or disable I/O capture for the observe decorator.
    #[must_use]
    pub fn io_capture_enabled(mut self, enabled: bool) -> Self {
        self.io_capture_enabled = Some(enabled);
        self
    }

    /// Set a filter function to decide whether a span should be exported.
    #[must_use]
    pub fn should_export_span(
        mut self,
        f: impl Fn(&opentelemetry_sdk::trace::SpanData) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.should_export_span = Some(Arc::new(f));
        self
    }

    /// Build the configuration, returning an error if required fields are missing.
    pub fn build(self) -> std::result::Result<LangfuseConfig, ConfigError> {
        let public_key = self.public_key.ok_or_else(|| ConfigError::MissingField {
            field: "public_key".into(),
        })?;
        let secret_key = self.secret_key.ok_or_else(|| ConfigError::MissingField {
            field: "secret_key".into(),
        })?;

        Ok(LangfuseConfig {
            public_key,
            secret_key,
            base_url: self
                .base_url
                .unwrap_or_else(|| "https://cloud.langfuse.com".into()),
            timeout: self.timeout.unwrap_or(Duration::from_secs(5)),
            flush_at: self.flush_at.unwrap_or(512),
            flush_interval: self.flush_interval.unwrap_or(Duration::from_secs(5)),
            sample_rate: self.sample_rate.unwrap_or(1.0),
            environment: self.environment,
            release: self.release,
            debug: self.debug.unwrap_or(false),
            tracing_enabled: self.tracing_enabled.unwrap_or(true),
            mask: self.mask,
            additional_headers: self.additional_headers,
            max_retries: self.max_retries.unwrap_or(3),
            media_upload_thread_count: self.media_upload_thread_count.unwrap_or(4),
            io_capture_enabled: self.io_capture_enabled.unwrap_or(true),
            should_export_span: self.should_export_span,
        })
    }
}
