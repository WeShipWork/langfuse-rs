//! Dataset manager: CRUD operations against the Langfuse dataset API.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use langfuse_core::config::LangfuseConfig;
use langfuse_core::error::LangfuseError;
use serde::Deserialize;
use tokio::sync::Semaphore;

use crate::datasets::evaluator::Evaluator;
use crate::datasets::experiment::{ExperimentConfig, ExperimentResult};
use crate::datasets::types::{
    CreateDatasetBody, CreateDatasetItemBody, Dataset, DatasetItem, DatasetRun,
};
use crate::http::retry_request;

/// Wrapper for paginated dataset-items responses.
#[derive(Debug, Deserialize)]
struct DatasetItemsResponse {
    data: Vec<DatasetItem>,
}

/// Wrapper for dataset-runs responses.
#[derive(Debug, Deserialize)]
struct DatasetRunsResponse {
    data: Vec<DatasetRun>,
}

/// Configuration for batched evaluation runs.
#[derive(Debug, Clone)]
pub struct BatchedEvaluationConfig {
    /// Maximum number of concurrent task executions.
    pub max_concurrency: usize,
    /// Page size for fetching dataset items.
    pub page_size: i32,
    /// Maximum number of retries for HTTP requests.
    pub max_retries: usize,
    /// Resume token: skip items with IDs lexicographically before this value.
    pub start_after: Option<String>,
    /// Name for the experiment run.
    pub run_name: String,
}

impl Default for BatchedEvaluationConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 10,
            page_size: 50,
            max_retries: 3,
            start_after: None,
            run_name: format!("batch-eval-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S")),
        }
    }
}

/// Manages dataset CRUD operations against the Langfuse API.
pub struct DatasetManager {
    config: LangfuseConfig,
    http_client: reqwest::Client,
}

impl std::fmt::Debug for DatasetManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatasetManager")
            .field("config", &self.config)
            .finish()
    }
}

impl DatasetManager {
    /// Create a new `DatasetManager` from the given configuration.
    pub fn new(config: &LangfuseConfig) -> Self {
        let http_client = crate::http::build_http_client(config);

        Self {
            config: config.clone(),
            http_client,
        }
    }

    /// Create a new dataset.
    pub async fn create_dataset(&self, body: CreateDatasetBody) -> Result<Dataset, LangfuseError> {
        let url = format!("{}/datasets", self.config.api_base_url());

        let resp = self
            .http_client
            .post(&url)
            .header("Authorization", self.config.basic_auth_header())
            .json(&body)
            .send()
            .await?;

        self.handle_response(resp).await
    }

    /// Get a dataset by name.
    pub async fn get_dataset(&self, name: &str) -> Result<Dataset, LangfuseError> {
        let url = format!("{}/datasets/{}", self.config.api_base_url(), name);

        let resp = self
            .http_client
            .get(&url)
            .header("Authorization", self.config.basic_auth_header())
            .send()
            .await?;

        self.handle_response(resp).await
    }

    /// Delete a dataset by name.
    ///
    /// Sends `DELETE /api/public/datasets/{name}` with retry logic for
    /// transient failures.
    pub async fn delete_dataset(&self, name: &str) -> Result<(), LangfuseError> {
        let url = format!("{}/datasets/{}", self.config.api_base_url(), name);
        let client = self.http_client.clone();
        let auth = self.config.basic_auth_header();

        retry_request(3, || {
            let url = url.clone();
            let client = client.clone();
            let auth = auth.clone();
            async move {
                let resp = client
                    .delete(&url)
                    .header("Authorization", auth)
                    .send()
                    .await?;

                let status = resp.status();
                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return Err(LangfuseError::Auth);
                }
                if !status.is_success() {
                    let message = resp.text().await.unwrap_or_default();
                    return Err(LangfuseError::Api {
                        status: status.as_u16(),
                        message,
                    });
                }
                Ok(())
            }
        })
        .await
    }

    /// Create a dataset item.
    pub async fn create_item(
        &self,
        body: CreateDatasetItemBody,
    ) -> Result<DatasetItem, LangfuseError> {
        let url = format!("{}/dataset-items", self.config.api_base_url());

        let resp = self
            .http_client
            .post(&url)
            .header("Authorization", self.config.basic_auth_header())
            .json(&body)
            .send()
            .await?;

        self.handle_response(resp).await
    }

    /// Get dataset items (paginated).
    pub async fn get_items(
        &self,
        dataset_name: &str,
        page: Option<i32>,
        limit: Option<i32>,
    ) -> Result<Vec<DatasetItem>, LangfuseError> {
        let url = format!("{}/dataset-items", self.config.api_base_url());

        let mut req = self
            .http_client
            .get(&url)
            .header("Authorization", self.config.basic_auth_header())
            .query(&[("datasetName", dataset_name)]);

        if let Some(p) = page {
            req = req.query(&[("page", p.to_string())]);
        }
        if let Some(l) = limit {
            req = req.query(&[("limit", l.to_string())]);
        }

        let resp = req.send().await?;
        let items_resp: DatasetItemsResponse = self.handle_response(resp).await?;
        Ok(items_resp.data)
    }

    /// Get dataset runs.
    pub async fn get_runs(&self, dataset_name: &str) -> Result<Vec<DatasetRun>, LangfuseError> {
        let url = format!(
            "{}/datasets/{}/runs",
            self.config.api_base_url(),
            dataset_name
        );

        let resp = self
            .http_client
            .get(&url)
            .header("Authorization", self.config.basic_auth_header())
            .send()
            .await?;

        let runs_resp: DatasetRunsResponse = self.handle_response(resp).await?;
        Ok(runs_resp.data)
    }

    /// Delete a dataset run.
    ///
    /// Sends `DELETE /api/public/datasets/{dataset_name}/runs/{run_name}` with
    /// retry logic for transient failures.
    pub async fn delete_run(
        &self,
        dataset_name: &str,
        run_name: &str,
    ) -> Result<(), LangfuseError> {
        let url = format!(
            "{}/datasets/{}/runs/{}",
            self.config.api_base_url(),
            dataset_name,
            run_name,
        );
        let client = self.http_client.clone();
        let auth = self.config.basic_auth_header();

        retry_request(3, || {
            let url = url.clone();
            let client = client.clone();
            let auth = auth.clone();
            async move {
                let resp = client
                    .delete(&url)
                    .header("Authorization", auth)
                    .send()
                    .await?;

                let status = resp.status();
                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return Err(LangfuseError::Auth);
                }
                if !status.is_success() {
                    let message = resp.text().await.unwrap_or_default();
                    return Err(LangfuseError::Api {
                        status: status.as_u16(),
                        message,
                    });
                }
                Ok(())
            }
        })
        .await
    }

    /// Run a batched evaluation over all items in a dataset.
    ///
    /// Fetches dataset items in pages, executes the task function on each item
    /// with bounded concurrency, runs evaluators, and collects results.
    ///
    /// If `config.start_after` is set, items with IDs lexicographically before
    /// that value are skipped (resume token support).
    pub async fn run_batched_evaluation<T>(
        &self,
        dataset_name: &str,
        batch_config: BatchedEvaluationConfig,
        task_fn: T,
        evaluators: Vec<Box<dyn Evaluator>>,
    ) -> Result<Vec<ExperimentResult>, LangfuseError>
    where
        T: Fn(DatasetItem) -> Pin<Box<dyn Future<Output = serde_json::Value> + Send>>
            + Send
            + Sync
            + 'static,
    {
        let experiment_config = ExperimentConfig {
            name: batch_config.run_name,
            max_concurrency: batch_config.max_concurrency,
            base_url: self.config.base_url.clone(),
            dataset_name: dataset_name.to_string(),
        };

        // Fetch all items page by page
        let mut all_items = Vec::new();
        let mut page = 1;
        loop {
            let items = self
                .get_items(dataset_name, Some(page), Some(batch_config.page_size))
                .await?;
            let fetched = items.len();
            all_items.extend(items);
            if (fetched as i32) < batch_config.page_size {
                break;
            }
            page += 1;
        }

        // Apply start_after filter (resume token)
        if let Some(ref start_after) = batch_config.start_after {
            all_items.retain(|item| item.id.as_str() > start_after.as_str());
        }

        // Run the experiment with evaluators
        let semaphore = Arc::new(Semaphore::new(experiment_config.max_concurrency));
        let run_url = experiment_config.dataset_run_url();
        let task_fn = Arc::new(task_fn);
        let evaluators: Arc<Vec<Box<dyn Evaluator>>> = Arc::new(evaluators);

        let handles: Vec<_> = all_items
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
                                    "Evaluator failed for item in batched evaluation"
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

        Ok(results)
    }

    /// Handle an HTTP response: check status, parse JSON body.
    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T, LangfuseError> {
        let status = resp.status();

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(LangfuseError::Auth);
        }
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(LangfuseError::Api {
                status: status.as_u16(),
                message,
            });
        }

        let body = resp.json::<T>().await?;
        Ok(body)
    }
}
