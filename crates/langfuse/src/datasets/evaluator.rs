//! Evaluator trait for dataset experiment evaluation.

use async_trait::async_trait;
use langfuse_core::error::LangfuseError;
use langfuse_core::types::Evaluation;

/// Trait for dataset experiment evaluators.
///
/// Implement this trait to define custom evaluation logic that compares
/// task output against expected output and produces scored evaluations.
///
/// # Example
///
/// ```rust,no_run
/// use langfuse::datasets::evaluator::Evaluator;
/// use langfuse_core::types::{Evaluation, ScoreValue};
/// use langfuse_core::error::LangfuseError;
///
/// struct AccuracyEvaluator;
///
/// #[async_trait::async_trait]
/// impl Evaluator for AccuracyEvaluator {
///     async fn evaluate(
///         &self,
///         output: &serde_json::Value,
///         expected: Option<&serde_json::Value>,
///     ) -> Result<Vec<Evaluation>, LangfuseError> {
///         let score = match expected {
///             Some(exp) if output == exp => 1.0,
///             _ => 0.0,
///         };
///         Ok(vec![Evaluation {
///             name: "accuracy".to_string(),
///             value: ScoreValue::Numeric(score),
///             comment: None,
///             metadata: None,
///             data_type: None,
///         }])
///     }
/// }
/// ```
#[async_trait]
pub trait Evaluator: Send + Sync {
    /// Evaluate a task output against an optional expected output.
    ///
    /// Returns a list of [`Evaluation`] results, each with a name and score.
    async fn evaluate(
        &self,
        output: &serde_json::Value,
        expected: Option<&serde_json::Value>,
    ) -> Result<Vec<Evaluation>, LangfuseError>;
}

/// Blanket implementation allowing async closures to be used as evaluators.
///
/// Any function matching the signature
/// `Fn(&Value, Option<&Value>) -> Future<Output = Result<Vec<Evaluation>, LangfuseError>>`
/// automatically implements [`Evaluator`].
///
/// # Example
///
/// ```rust,no_run
/// use langfuse_core::types::{Evaluation, ScoreValue};
/// use langfuse_core::error::LangfuseError;
///
/// let evaluator = |output: &serde_json::Value, expected: Option<&serde_json::Value>| {
///     let score = if expected == Some(output) { 1.0 } else { 0.0 };
///     async move {
///         Ok::<_, LangfuseError>(vec![Evaluation {
///             name: "match".to_string(),
///             value: ScoreValue::Numeric(score),
///             comment: None,
///             metadata: None,
///             data_type: None,
///         }])
///     }
/// };
/// ```
#[async_trait]
impl<F, Fut> Evaluator for F
where
    F: Fn(&serde_json::Value, Option<&serde_json::Value>) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<Vec<Evaluation>, LangfuseError>> + Send,
{
    async fn evaluate(
        &self,
        output: &serde_json::Value,
        expected: Option<&serde_json::Value>,
    ) -> Result<Vec<Evaluation>, LangfuseError> {
        (self)(output, expected).await
    }
}
