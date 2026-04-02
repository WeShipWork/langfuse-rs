//! OpenTelemetry-native tracing pipeline for Langfuse.
//!
//! Maps Langfuse observations (spans, generations, events) to OpenTelemetry spans
//! and exports them via OTLP HTTP to Langfuse's `/api/public/otel` endpoint.

pub mod attributes;
pub mod context;
pub mod context_apis;
pub mod embedding;
pub mod exporter;
pub mod generation;
pub mod observe;
pub mod span;
pub mod stream_wrapper;
