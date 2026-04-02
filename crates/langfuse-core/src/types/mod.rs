//! Domain types for the Langfuse SDK.
//!
//! Includes observation types, prompt types, score types, media handling,
//! and evaluation result types.

/// Evaluation result types for dataset evaluators.
pub mod evaluation;
/// Media content and reference types.
pub mod media;
/// Observation types, levels, usage/cost details, and span attributes.
pub mod observation;
/// Prompt template types (text and chat).
pub mod prompt;
/// Score types, values, and builder.
pub mod score;

pub use evaluation::*;
pub use media::*;
pub use observation::*;
pub use prompt::*;
pub use score::*;
