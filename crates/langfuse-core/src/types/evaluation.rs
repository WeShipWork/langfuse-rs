//! Evaluation result types for dataset experiment evaluators.

use super::score::ScoreValue;
use serde::{Deserialize, Serialize};

/// An evaluation result produced by an `Evaluator`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    /// Name of the evaluation metric (e.g. "accuracy", "relevance").
    pub name: String,
    /// The score value — numeric, categorical, or boolean.
    pub value: ScoreValue,
    /// Optional human-readable comment explaining the evaluation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Optional structured metadata attached to the evaluation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// Optional data type hint (e.g. "NUMERIC", "BOOLEAN", "CATEGORICAL").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_type: Option<String>,
}
