//! Core types, configuration, and API client for the Langfuse SDK.
//!
//! This crate provides the foundational building blocks used by the higher-level
//! `langfuse-sdk` crate: configuration management, error types, domain types
//! (prompts, scores, observations, media), and an auto-generated REST API client
//! derived from Langfuse's OpenAPI specification.

#![warn(missing_docs)]

#[allow(
    missing_docs,
    unused_imports,
    mismatched_lifetime_syntaxes,
    irrefutable_let_patterns,
    clippy::all
)]
pub mod api;
pub mod config;
pub mod error;
pub mod types;

pub use config::LangfuseConfig;
pub use error::{LangfuseError, Result};
