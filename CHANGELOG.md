# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-04-01

### Added

- **OTel-native tracing**: Full OpenTelemetry integration via OTLP HTTP with span and generation tracking
- **Prompt management**: TTL-cached prompt fetching with variable interpolation and Chat/Text prompt types
- **Score management**: Batched score submission with automatic background flush and configurable intervals
- **Dataset and experiment management**: Dataset CRUD operations, experiment runner with bounded parallelism, and evaluator trait for custom evaluation logic
- **Media handling**: Two-step presigned URL upload and fetch for media attachments
- **OpenAI integration**: Async-openai wrapper for automatic generation span creation
- **#[observe] proc macro**: Attribute macro for automatic span instrumentation with configurable capture of inputs/outputs
- **Auto-generated REST API client**: Progenitor-based API client from OpenAPI specification
- **Context-aware APIs**: Free functions for span mutation without explicit handles (update_current_span, score_current_span, get_current_trace_url)
- **Stream and iterator observation**: Wrappers for automatic span finalization on stream/iterator completion
- **Embedding support**: Dedicated LangfuseEmbedding type for embedding operation tracking
- **Named instances**: Multi-project support via named Langfuse client instances alongside default singleton
- **Retry logic**: Exponential backoff retry mechanism for all HTTP requests
- **Comprehensive configuration**: Builder pattern config with environment variable support and sensible defaults

[Unreleased]: https://github.com/weshipwork/langfuse-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/weshipwork/langfuse-rs/releases/tag/v0.1.0
