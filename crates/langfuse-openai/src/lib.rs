//! `OpenAI` integration for the Langfuse SDK.
//!
//! Wraps [`async_openai`] to automatically create Langfuse observation spans
//! for every chat completion and embedding API call. Drop-in tracing for
//! existing `async-openai` usage with zero code changes beyond wrapping the client.

#![warn(missing_docs)]

pub mod parser;
pub mod wrapper;

pub use wrapper::{
    TracedChat, TracedEmbeddings, TracedStream, observe_openai, observe_openai_embeddings,
};
