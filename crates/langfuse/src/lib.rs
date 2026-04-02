#![warn(missing_docs)]

//! Langfuse SDK for Rust — LLM observability, prompt management, and evaluation.
//!
//! # Quick Start
//! ```no_run
//! use langfuse::Langfuse;
//!
//! let client = Langfuse::new(
//!     langfuse_core::LangfuseConfig::builder()
//!         .public_key("pk-lf-...")
//!         .secret_key("sk-lf-...")
//!         .build()
//!         .unwrap()
//! ).unwrap();
//! ```

pub mod client;
pub mod datasets;
pub mod http;
#[path = "tracing/mod.rs"]
pub mod langfuse_tracing;
pub mod media;
pub mod prompts;
pub mod scoring;

// Re-exports for ergonomic usage
pub use client::Langfuse;
pub use langfuse_macros::observe;

pub use langfuse_core::types::*;
pub use langfuse_core::{LangfuseConfig, LangfuseError, Result};

pub use langfuse_tracing::context::{
    get_current_observation_id, get_current_trace_id, propagate_as_baggage, propagate_attributes,
};
pub use langfuse_tracing::context_apis::{
    get_current_trace_url, score_current_span, score_current_span_with, score_current_trace,
    set_current_trace_as_public, update_current_generation, update_current_span,
};
pub use langfuse_tracing::embedding::LangfuseEmbedding;
pub use langfuse_tracing::generation::LangfuseGeneration;
pub use langfuse_tracing::observe::{
    with_agent, with_chain, with_embedding, with_evaluator, with_generation, with_guardrail,
    with_observation, with_retriever, with_span, with_tool,
};
pub use langfuse_tracing::span::LangfuseSpan;
pub use langfuse_tracing::stream_wrapper::{ObservingIterator, ObservingStream};
