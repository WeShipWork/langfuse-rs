//! Score creation and batched submission to the Langfuse API.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use langfuse_core::config::LangfuseConfig;
use langfuse_core::error::LangfuseError;
use langfuse_core::types::{ScoreBody, ScoreValue};

use crate::http::{build_http_client, retry_request};
use crate::scoring::queue::BatchQueue;

/// Manages score creation and batched submission to the Langfuse API.
///
/// Scores are buffered in a [`BatchQueue`] and flushed to the ingestion
/// endpoint either when the buffer reaches `flush_at` or on explicit
/// [`flush`](ScoreManager::flush) / [`shutdown`](ScoreManager::shutdown).
///
/// A background task periodically flushes buffered scores at the configured
/// `flush_interval`.
pub struct ScoreManager {
    config: LangfuseConfig,
    http_client: reqwest::Client,
    queue: Arc<BatchQueue>,
    flush_at: usize,
    cancelled: Arc<AtomicBool>,
    flush_handle: Option<tokio::task::JoinHandle<()>>,
}

impl std::fmt::Debug for ScoreManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScoreManager")
            .field("flush_at", &self.flush_at)
            .field("pending_count", &self.queue.len())
            .finish()
    }
}

impl ScoreManager {
    /// Create a new `ScoreManager` from the given configuration.
    ///
    /// Spawns a background task that auto-flushes buffered scores at the
    /// configured `flush_interval`.
    pub fn new(config: &LangfuseConfig) -> Self {
        let http_client = build_http_client(config);
        let queue = Arc::new(BatchQueue::new(100_000));
        let cancelled = Arc::new(AtomicBool::new(false));

        let flush_config = config.clone();
        let flush_client = http_client.clone();
        let flush_queue = Arc::clone(&queue);
        let flush_cancelled = Arc::clone(&cancelled);
        let flush_interval = config.flush_interval;

        let flush_handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(flush_interval).await;
                if flush_cancelled.load(Ordering::Relaxed) {
                    break;
                }
                let _ = Self::flush_inner(&flush_config, &flush_client, &flush_queue).await;
            }
        });

        Self {
            config: config.clone(),
            http_client,
            queue,
            flush_at: config.flush_at,
            cancelled,
            flush_handle: Some(flush_handle),
        }
    }

    /// Create a score and add it to the batch queue.
    ///
    /// If the queue reaches `flush_at`, a flush would be triggered
    /// (currently buffered for explicit flush).
    pub fn score(&self, body: ScoreBody) {
        let should_flush = self.queue.push(body);
        if should_flush || self.queue.len() >= self.flush_at {
            // Auto-flush background task handles periodic flushing.
        }
    }

    /// Score a specific observation within a trace.
    pub fn score_observation(
        &self,
        trace_id: &str,
        observation_id: &str,
        name: &str,
        value: ScoreValue,
    ) {
        self.score(ScoreBody {
            name: name.to_string(),
            value,
            trace_id: Some(trace_id.to_string()),
            observation_id: Some(observation_id.to_string()),
            comment: None,
            metadata: None,
            config_id: None,
            data_type: None,
        });
    }

    /// Score a specific observation with a full [`ScoreBody`].
    ///
    /// This is the rich variant of [`score_observation`](ScoreManager::score_observation)
    /// that accepts a pre-built body with optional comment, metadata, etc.
    pub fn score_observation_with(&self, body: ScoreBody) {
        self.score(body);
    }

    /// Score a trace.
    pub fn score_trace(&self, trace_id: &str, name: &str, value: ScoreValue) {
        self.score(ScoreBody {
            name: name.to_string(),
            value,
            trace_id: Some(trace_id.to_string()),
            observation_id: None,
            comment: None,
            metadata: None,
            config_id: None,
            data_type: None,
        });
    }

    /// Flush all buffered scores to the Langfuse ingestion API.
    ///
    /// Drains the buffer and POSTs a batch of `score-create` events.
    /// Returns `Ok(())` immediately if the buffer is empty.
    pub async fn flush(&self) -> Result<(), LangfuseError> {
        Self::flush_inner(&self.config, &self.http_client, &self.queue).await
    }

    /// Internal flush implementation shared between the public method and
    /// the background auto-flush task.
    async fn flush_inner(
        config: &LangfuseConfig,
        http_client: &reqwest::Client,
        queue: &BatchQueue,
    ) -> Result<(), LangfuseError> {
        let scores = queue.drain();
        if scores.is_empty() {
            return Ok(());
        }

        let url = format!("{}/ingestion", config.api_base_url());
        let batch_body = serde_json::json!({
            "batch": scores.iter().map(|s| {
                serde_json::json!({
                    "id": uuid::Uuid::new_v4().to_string(),
                    "type": "score-create",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "body": s,
                })
            }).collect::<Vec<_>>()
        });

        let max_retries = config.max_retries;
        let client = http_client.clone();
        let auth = config.basic_auth_header();
        let url_clone = url.clone();
        let body_clone = batch_body.clone();

        retry_request(max_retries, || {
            let client = client.clone();
            let auth = auth.clone();
            let url = url_clone.clone();
            let body = body_clone.clone();
            async move {
                let resp = client
                    .post(&url)
                    .header("Authorization", &auth)
                    .header("Content-Type", "application/json")
                    .json(&body)
                    .send()
                    .await
                    .map_err(LangfuseError::Network)?;

                if !resp.status().is_success() {
                    let status = resp.status().as_u16();
                    let message = resp.text().await.unwrap_or_default();
                    return Err(LangfuseError::Api { status, message });
                }

                Ok(())
            }
        })
        .await
    }

    /// Flush all buffered scores and shut down the background task.
    pub async fn shutdown(&self) -> Result<(), LangfuseError> {
        self.cancelled.store(true, Ordering::Relaxed);
        self.flush().await
    }

    /// Number of scores currently buffered.
    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }
}

impl Drop for ScoreManager {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        if let Some(handle) = self.flush_handle.take() {
            handle.abort();
        }
    }
}
