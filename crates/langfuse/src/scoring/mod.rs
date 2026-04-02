//! Score management with batched async flush.
//!
//! Provides [`manager::ScoreManager`] for creating scores and a background
//! [`queue::BatchQueue`] that automatically flushes scores to the Langfuse API.

pub mod manager;
pub mod queue;
