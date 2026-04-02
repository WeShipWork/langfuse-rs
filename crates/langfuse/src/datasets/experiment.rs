//! Experiment runner: execute a task function on each dataset item, then evaluate.

use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::datasets::evaluator::Evaluator;
use crate::datasets::types::DatasetItem;

/// Result of running an experiment task on a single dataset item.
#[derive(Debug, Clone)]
pub struct ExperimentResult {
    /// ID of the dataset item that was processed.
    pub item_id: String,
    /// Output value produced by the task function.
    pub output: serde_json::Value,
    /// List of `(metric_name, score)` pairs from evaluators.
    pub scores: Vec<(String, f64)>,
    /// URL to the dataset run in the Langfuse UI.
    pub dataset_run_url: String,
}

impl ExperimentResult {
    /// Format a summary of this experiment result.
    pub fn format(&self) -> String {
        let mut summary = format!("Item: {}\n", self.item_id);
        summary.push_str(&format!("Output: {}\n", self.output));
        if self.scores.is_empty() {
            summary.push_str("Scores: (none)\n");
        } else {
            summary.push_str("Scores:\n");
            for (name, value) in &self.scores {
                summary.push_str(&format!("  {name}: {value}\n"));
            }
        }
        if !self.dataset_run_url.is_empty() {
            summary.push_str(&format!("Run URL: {}\n", self.dataset_run_url));
        }
        summary
    }
}

impl fmt::Display for ExperimentResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Format a summary of multiple experiment results.
///
/// Shows total count, per-metric averages, and individual item scores.
pub fn format_experiment_summary(results: &[ExperimentResult]) -> String {
    let mut summary = format!("Experiment Summary ({} items)\n", results.len());
    summary.push_str(&"─".repeat(40));
    summary.push('\n');

    if results.is_empty() {
        summary.push_str("No results.\n");
        return summary;
    }

    // Aggregate scores by name
    let mut score_sums: HashMap<String, (f64, usize)> = HashMap::new();
    for result in results {
        for (name, value) in &result.scores {
            let entry = score_sums.entry(name.clone()).or_insert((0.0, 0));
            entry.0 += value;
            entry.1 += 1;
        }
    }

    if !score_sums.is_empty() {
        summary.push_str("Average Scores:\n");
        let mut names: Vec<_> = score_sums.keys().collect();
        names.sort();
        for name in names {
            let (total, count) = score_sums[name];
            let avg = total / count as f64;
            summary.push_str(&format!("  {name}: {avg:.4} (n={count})\n"));
        }
    }

    summary
}

/// Configuration for running an experiment.
#[derive(Debug, Clone)]
pub struct ExperimentConfig {
    /// Name of the experiment run.
    pub name: String,
    /// Maximum number of concurrent task executions.
    pub max_concurrency: usize,
    /// Base URL for constructing dataset run URLs.
    pub base_url: String,
    /// Dataset name for constructing dataset run URLs.
    pub dataset_name: String,
}

impl Default for ExperimentConfig {
    fn default() -> Self {
        Self {
            name: format!("experiment-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S")),
            max_concurrency: 10,
            base_url: String::new(),
            dataset_name: String::new(),
        }
    }
}

impl ExperimentConfig {
    /// Build the dataset run URL from config fields.
    pub fn dataset_run_url(&self) -> String {
        if self.base_url.is_empty() || self.dataset_name.is_empty() {
            return String::new();
        }
        format!(
            "{}/datasets/{}/runs/{}",
            self.base_url.trim_end_matches('/'),
            self.dataset_name,
            self.name,
        )
    }
}

/// Run an experiment: execute a task function on each dataset item, then evaluate.
///
/// The `task_fn` is called for each item to produce an output value.
/// The `evaluator_fn` compares the output against the item (including its
/// `expected_output`) and returns a list of named scores.
///
/// Concurrency is bounded by [`ExperimentConfig::max_concurrency`].
pub async fn run_experiment<T, E>(
    items: Vec<DatasetItem>,
    config: ExperimentConfig,
    task_fn: T,
    evaluator_fn: E,
) -> Vec<ExperimentResult>
where
    T: Fn(DatasetItem) -> Pin<Box<dyn Future<Output = serde_json::Value> + Send>>
        + Send
        + Sync
        + 'static,
    E: Fn(&DatasetItem, &serde_json::Value) -> Vec<(String, f64)> + Send + Sync + 'static,
{
    let semaphore = Arc::new(Semaphore::new(config.max_concurrency));
    let run_url = config.dataset_run_url();
    let task_fn = Arc::new(task_fn);
    let evaluator_fn = Arc::new(evaluator_fn);

    let handles: Vec<_> = items
        .into_iter()
        .map(|item| {
            let sem = semaphore.clone();
            let task = task_fn.clone();
            let eval = evaluator_fn.clone();
            let url = run_url.clone();
            tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed");
                let output = task(item.clone()).await;
                let scores = eval(&item, &output);
                ExperimentResult {
                    item_id: item.id,
                    output,
                    scores,
                    dataset_run_url: url,
                }
            })
        })
        .collect();

    let mut results = Vec::new();
    for handle in handles {
        if let Ok(result) = handle.await {
            results.push(result);
        }
    }
    results
}

/// Run an experiment with trait-based evaluators.
///
/// Similar to [`run_experiment`], but accepts a list of [`Evaluator`] trait
/// objects instead of a simple closure. Each evaluator is called after the
/// task function, and all evaluation results are converted to `(name, f64)`
/// score tuples.
///
/// The original `evaluator_fn` is still called first (if provided), then
/// each trait evaluator is invoked.
pub async fn run_experiment_with_evaluators<T>(
    items: Vec<DatasetItem>,
    config: ExperimentConfig,
    task_fn: T,
    evaluators: Vec<Box<dyn Evaluator>>,
) -> Vec<ExperimentResult>
where
    T: Fn(DatasetItem) -> Pin<Box<dyn Future<Output = serde_json::Value> + Send>>
        + Send
        + Sync
        + 'static,
{
    let semaphore = Arc::new(Semaphore::new(config.max_concurrency));
    let run_url = config.dataset_run_url();
    let task_fn = Arc::new(task_fn);
    let evaluators: Arc<Vec<Box<dyn Evaluator>>> = Arc::new(evaluators);

    let handles: Vec<_> = items
        .into_iter()
        .map(|item| {
            let sem = semaphore.clone();
            let task = task_fn.clone();
            let evals = evaluators.clone();
            let url = run_url.clone();
            tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed");
                let output = task(item.clone()).await;

                let mut scores = Vec::new();
                for evaluator in evals.iter() {
                    match evaluator
                        .evaluate(&output, item.expected_output.as_ref())
                        .await
                    {
                        Ok(evaluations) => {
                            for evaluation in evaluations {
                                let numeric = match evaluation.value {
                                    langfuse_core::types::ScoreValue::Numeric(v) => v,
                                    langfuse_core::types::ScoreValue::Boolean(b) => {
                                        if b {
                                            1.0
                                        } else {
                                            0.0
                                        }
                                    }
                                    langfuse_core::types::ScoreValue::Categorical(_) => 0.0,
                                };
                                scores.push((evaluation.name, numeric));
                            }
                        }
                        Err(err) => {
                            tracing::warn!(
                                item_id = %item.id,
                                error = %err,
                                "Evaluator failed for item"
                            );
                        }
                    }
                }

                ExperimentResult {
                    item_id: item.id,
                    output,
                    scores,
                    dataset_run_url: url,
                }
            })
        })
        .collect();

    let mut results = Vec::new();
    for handle in handles {
        if let Ok(result) = handle.await {
            results.push(result);
        }
    }
    results
}
