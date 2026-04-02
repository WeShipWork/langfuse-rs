//! Data types for Langfuse dataset management.

use serde::{Deserialize, Serialize};

/// A Langfuse dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dataset {
    /// Unique dataset identifier.
    pub id: String,
    /// Dataset name.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// A dataset item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetItem {
    /// Unique item identifier.
    pub id: String,
    /// ID of the parent dataset.
    pub dataset_id: String,
    /// Input data for this item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    /// Expected output for evaluation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_output: Option<serde_json::Value>,
    /// Optional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// Trace ID this item was sourced from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_trace_id: Option<String>,
    /// Observation ID this item was sourced from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_observation_id: Option<String>,
    /// Item status (e.g. "ACTIVE").
    pub status: String,
}

/// A dataset run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetRun {
    /// Unique run identifier.
    pub id: String,
    /// Run name.
    pub name: String,
    /// ID of the parent dataset.
    pub dataset_id: String,
    /// Optional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Body for creating a dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDatasetBody {
    /// Dataset name.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Body for creating a dataset item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDatasetItemBody {
    /// Name of the dataset to add the item to.
    pub dataset_name: String,
    /// Input data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    /// Expected output for evaluation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_output: Option<serde_json::Value>,
    /// Optional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// Optional explicit item ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}
