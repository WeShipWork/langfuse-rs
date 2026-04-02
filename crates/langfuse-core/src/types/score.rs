use serde::{Deserialize, Serialize};

/// The data type of a score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScoreDataType {
    /// A numeric score (e.g. 0.0–1.0).
    Numeric,
    /// A categorical score (e.g. "good", "bad").
    Categorical,
    /// A boolean score (pass/fail).
    Boolean,
}

/// A score value — can be numeric, categorical, or boolean.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScoreValue {
    /// A numeric value.
    Numeric(f64),
    /// A categorical (string) value.
    Categorical(String),
    /// A boolean value.
    Boolean(bool),
}

/// Body for creating a score.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreBody {
    /// Name of the score metric.
    pub name: String,
    /// The score value.
    pub value: ScoreValue,
    /// Trace ID this score belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Observation ID this score belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observation_id: Option<String>,
    /// Optional human-readable comment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Optional metadata as JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// Optional score config ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_id: Option<String>,
    /// Optional data type override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_type: Option<ScoreDataType>,
}

/// Builder for constructing a [`ScoreBody`] with a fluent API.
pub struct ScoreBodyBuilder {
    name: String,
    value: ScoreValue,
    trace_id: Option<String>,
    observation_id: Option<String>,
    comment: Option<String>,
    metadata: Option<serde_json::Value>,
    config_id: Option<String>,
    data_type: Option<ScoreDataType>,
}

impl ScoreBody {
    /// Create a new builder with the required `name` and `value`.
    ///
    /// ```ignore
    /// let body = ScoreBody::builder("accuracy", ScoreValue::Numeric(0.95))
    ///     .comment("High accuracy")
    ///     .build();
    /// ```
    #[must_use]
    pub fn builder(name: impl Into<String>, value: ScoreValue) -> ScoreBodyBuilder {
        ScoreBodyBuilder {
            name: name.into(),
            value,
            trace_id: None,
            observation_id: None,
            comment: None,
            metadata: None,
            config_id: None,
            data_type: None,
        }
    }
}

impl ScoreBodyBuilder {
    /// Set the trace ID.
    #[must_use]
    pub fn trace_id(mut self, id: impl Into<String>) -> Self {
        self.trace_id = Some(id.into());
        self
    }

    /// Set the observation ID.
    #[must_use]
    pub fn observation_id(mut self, id: impl Into<String>) -> Self {
        self.observation_id = Some(id.into());
        self
    }

    /// Set a comment.
    #[must_use]
    pub fn comment(mut self, c: impl Into<String>) -> Self {
        self.comment = Some(c.into());
        self
    }

    /// Set metadata as a JSON value.
    #[must_use]
    pub fn metadata(mut self, m: serde_json::Value) -> Self {
        self.metadata = Some(m);
        self
    }

    /// Set the score config ID.
    #[must_use]
    pub fn config_id(mut self, id: impl Into<String>) -> Self {
        self.config_id = Some(id.into());
        self
    }

    /// Set the data type.
    #[must_use]
    pub fn data_type(mut self, dt: ScoreDataType) -> Self {
        self.data_type = Some(dt);
        self
    }

    /// Build the [`ScoreBody`].
    #[must_use]
    pub fn build(self) -> ScoreBody {
        ScoreBody {
            name: self.name,
            value: self.value,
            trace_id: self.trace_id,
            observation_id: self.observation_id,
            comment: self.comment,
            metadata: self.metadata,
            config_id: self.config_id,
            data_type: self.data_type,
        }
    }
}
