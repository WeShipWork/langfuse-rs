//! Error types for the Langfuse SDK.

/// Primary error type for all Langfuse operations.
#[derive(thiserror::Error, Debug)]
pub enum LangfuseError {
    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// API returned an error response.
    #[error("API error: {status} - {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Error message from the API.
        message: String,
    },

    /// Authentication failed (401).
    #[error("Authentication failed")]
    Auth,

    /// Network-level error.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Requested prompt was not found.
    #[error("Prompt not found: {name}")]
    PromptNotFound {
        /// Name of the prompt that was not found.
        name: String,
    },

    /// Prompt template compilation failed due to a missing variable.
    #[error("Prompt compilation error: missing variable '{variable}'")]
    PromptCompilation {
        /// Name of the missing template variable.
        variable: String,
    },

    /// Media upload or processing error.
    #[error("Media error: {0}")]
    Media(String),

    /// OpenTelemetry pipeline error.
    #[error("OpenTelemetry error: {0}")]
    Otel(String),
}

/// Configuration-specific errors.
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    /// A required configuration field was not provided.
    #[error("Missing required configuration: {field}")]
    MissingField {
        /// Name of the missing configuration field.
        field: String,
    },

    /// A configuration field had an invalid value.
    #[error("Invalid configuration value for '{field}': {message}")]
    InvalidValue {
        /// Name of the invalid field.
        field: String,
        /// Description of why the value is invalid.
        message: String,
    },
}

/// Convenience result type for Langfuse operations.
pub type Result<T> = std::result::Result<T, LangfuseError>;
