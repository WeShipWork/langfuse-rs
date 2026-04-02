//! OTLP HTTP exporter configured for Langfuse's trace ingestion endpoint.

use langfuse_core::config::{LangfuseConfig, SpanFilterFn};
use langfuse_core::error::LangfuseError;
use opentelemetry::Context;
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::trace::{
    BatchConfigBuilder, BatchSpanProcessor, SdkTracerProvider, SpanData, SpanProcessor,
};
use std::collections::HashMap;
use std::time::Duration;

/// A Langfuse-configured OTel TracerProvider.
///
/// Creates an OTLP HTTP exporter pointed at Langfuse with Basic Auth,
/// wrapped in a `BatchSpanProcessor`.
pub struct LangfuseTracing {
    provider: SdkTracerProvider,
}

impl std::fmt::Debug for LangfuseTracing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LangfuseTracing")
            .field("provider", &"<SdkTracerProvider>")
            .finish()
    }
}

impl LangfuseTracing {
    /// Create a new builder from a [`LangfuseConfig`].
    pub fn builder(config: &LangfuseConfig) -> LangfuseTracingBuilder {
        LangfuseTracingBuilder {
            config: config.clone(),
        }
    }

    /// Get a reference to the underlying [`SdkTracerProvider`].
    pub fn provider(&self) -> &SdkTracerProvider {
        &self.provider
    }

    /// Shut down the tracing pipeline, flushing remaining spans.
    pub fn shutdown(&self) -> Result<(), opentelemetry_sdk::error::OTelSdkError> {
        self.provider.shutdown()
    }
}

/// Builder for [`LangfuseTracing`].
pub struct LangfuseTracingBuilder {
    config: LangfuseConfig,
}

impl std::fmt::Debug for LangfuseTracingBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LangfuseTracingBuilder")
            .field("config", &self.config)
            .finish()
    }
}

impl LangfuseTracingBuilder {
    /// Build the [`LangfuseTracing`] instance, creating the OTLP exporter,
    /// batch processor, and tracer provider.
    pub fn build(self) -> Result<LangfuseTracing, LangfuseError> {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), self.config.basic_auth_header());

        // Merge additional_headers from config
        if let Some(ref extra) = self.config.additional_headers {
            for (k, v) in extra {
                headers.insert(k.clone(), v.clone());
            }
        }

        let exporter = SpanExporter::builder()
            .with_http()
            .with_endpoint(format!(
                "{}/api/public/otel/v1/traces",
                self.config.base_url.trim_end_matches('/')
            ))
            .with_protocol(Protocol::HttpBinary)
            .with_timeout(self.config.timeout)
            .with_headers(headers)
            .build()
            .map_err(|e| LangfuseError::Otel(e.to_string()))?;

        let batch_config = BatchConfigBuilder::default()
            .with_max_queue_size(self.config.flush_at * 4)
            .with_scheduled_delay(self.config.flush_interval)
            .with_max_export_batch_size(self.config.flush_at)
            .build();

        let processor = BatchSpanProcessor::builder(exporter)
            .with_batch_config(batch_config)
            .build();

        // Wrap with filtering processor if a filter is configured
        let span_filter = self.config.should_export_span.clone();

        // Build resource with SDK + service metadata attributes.
        // Read OTEL_SERVICE_NAME (standard OTel env var) for service.name.
        let service_name =
            std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "unknown_service".to_string());
        let mut attrs = vec![
            opentelemetry::KeyValue::new("sdk.name", "langfuse-rs"),
            opentelemetry::KeyValue::new("sdk.version", env!("CARGO_PKG_VERSION")),
            opentelemetry::KeyValue::new("service.name", service_name),
        ];
        if let Some(ref env) = self.config.environment {
            attrs.push(opentelemetry::KeyValue::new(
                "deployment.environment",
                env.clone(),
            ));
        }
        if let Some(ref rel) = self.config.release {
            attrs.push(opentelemetry::KeyValue::new("service.version", rel.clone()));
        }
        let resource = opentelemetry_sdk::Resource::builder()
            .with_attributes(attrs)
            .build();

        let mut builder = SdkTracerProvider::builder().with_resource(resource);

        if let Some(filter) = span_filter {
            builder = builder.with_span_processor(FilteringSpanProcessor::new(processor, filter));
        } else {
            builder = builder.with_span_processor(processor);
        }

        let provider = builder.build();

        Ok(LangfuseTracing { provider })
    }
}

/// A span processor that wraps an inner processor and filters spans
/// based on a user-provided predicate before forwarding to the inner processor.
struct FilteringSpanProcessor {
    inner: BatchSpanProcessor,
    filter: SpanFilterFn,
}

impl std::fmt::Debug for FilteringSpanProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilteringSpanProcessor")
            .field("inner", &self.inner)
            .field("filter", &"<filter fn>")
            .finish()
    }
}

impl FilteringSpanProcessor {
    fn new(inner: BatchSpanProcessor, filter: SpanFilterFn) -> Self {
        Self { inner, filter }
    }
}

impl SpanProcessor for FilteringSpanProcessor {
    fn on_start(&self, span: &mut opentelemetry_sdk::trace::Span, cx: &Context) {
        self.inner.on_start(span, cx);
    }

    fn on_end(&self, span: SpanData) {
        if (self.filter)(&span) {
            self.inner.on_end(span);
        }
    }

    fn force_flush(&self) -> OTelSdkResult {
        self.inner.force_flush()
    }

    fn shutdown_with_timeout(&self, timeout: Duration) -> OTelSdkResult {
        self.inner.shutdown_with_timeout(timeout)
    }
}
