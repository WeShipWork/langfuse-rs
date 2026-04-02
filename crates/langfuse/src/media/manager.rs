//! Media upload and fetch manager for the Langfuse API.

use std::sync::Arc;

use langfuse_core::config::LangfuseConfig;
use langfuse_core::error::LangfuseError;
use tokio::sync::Semaphore;

use crate::media::types::LangfuseMedia;

/// Shared inner state for [`MediaManager`].
struct MediaManagerInner {
    config: LangfuseConfig,
    http_client: reqwest::Client,
    upload_semaphore: Semaphore,
}

/// Manages media uploads and downloads against the Langfuse API.
///
/// Cheaply cloneable (backed by `Arc`). Background uploads are bounded
/// by a semaphore whose size is controlled by
/// [`LangfuseConfig::media_upload_thread_count`].
#[derive(Clone)]
pub struct MediaManager {
    inner: Arc<MediaManagerInner>,
}

impl std::fmt::Debug for MediaManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MediaManager").finish()
    }
}

impl MediaManager {
    /// Create a new `MediaManager` from the given configuration.
    pub fn new(config: &LangfuseConfig) -> Self {
        Self {
            inner: Arc::new(MediaManagerInner {
                http_client: crate::http::build_http_client(config),
                upload_semaphore: Semaphore::new(config.media_upload_thread_count),
                config: config.clone(),
            }),
        }
    }

    /// Upload media and return the media ID.
    ///
    /// This performs a two-step upload:
    /// 1. Request a presigned upload URL from the Langfuse API.
    /// 2. Upload the raw bytes to the presigned URL.
    pub async fn upload(
        &self,
        trace_id: &str,
        observation_id: Option<&str>,
        field: &str,
        media: &LangfuseMedia,
    ) -> Result<String, LangfuseError> {
        // Step 1: Request upload URL from Langfuse API
        let url = format!("{}/media", self.inner.config.api_base_url());
        let body = serde_json::json!({
            "traceId": trace_id,
            "observationId": observation_id,
            "field": field,
            "contentType": media.content_type,
            "contentLength": media.size(),
        });

        let resp = self
            .inner
            .http_client
            .post(&url)
            .header("Authorization", self.inner.config.basic_auth_header())
            .json(&body)
            .send()
            .await
            .map_err(LangfuseError::Network)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(LangfuseError::Api { status, message });
        }

        let resp_body: serde_json::Value = resp.json().await.map_err(LangfuseError::Network)?;

        let media_id = resp_body["mediaId"]
            .as_str()
            .ok_or_else(|| LangfuseError::Media("Missing mediaId in response".into()))?
            .to_string();
        let upload_url = resp_body["uploadUrl"]
            .as_str()
            .ok_or_else(|| LangfuseError::Media("Missing uploadUrl in response".into()))?;

        // Step 2: Upload to the presigned URL
        self.inner
            .http_client
            .put(upload_url)
            .header("Content-Type", &media.content_type)
            .body(media.data.clone())
            .send()
            .await
            .map_err(LangfuseError::Network)?;

        Ok(media_id)
    }

    /// Upload media in a background task.
    ///
    /// Spawns a `tokio` task that acquires a permit from the upload semaphore
    /// (bounded by `media_upload_thread_count`) before performing the upload.
    /// Errors are logged via `tracing::warn` and otherwise silently dropped.
    pub fn upload_background(
        &self,
        trace_id: String,
        observation_id: Option<String>,
        field: String,
        media: LangfuseMedia,
    ) {
        let manager = self.clone();
        tokio::spawn(async move {
            let _permit = match manager.inner.upload_semaphore.acquire().await {
                Ok(permit) => permit,
                Err(_) => {
                    tracing::warn!("Media upload semaphore closed");
                    return;
                }
            };
            if let Err(e) = manager
                .upload(&trace_id, observation_id.as_deref(), &field, &media)
                .await
            {
                tracing::warn!("Background media upload failed: {e}");
            }
        });
    }

    /// Fetch media by ID.
    ///
    /// Retrieves the media metadata from the Langfuse API, then downloads
    /// the actual content from the returned URL.
    pub async fn fetch(&self, media_id: &str) -> Result<LangfuseMedia, LangfuseError> {
        let url = format!("{}/media/{media_id}", self.inner.config.api_base_url());
        let resp = self
            .inner
            .http_client
            .get(&url)
            .header("Authorization", self.inner.config.basic_auth_header())
            .send()
            .await
            .map_err(LangfuseError::Network)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(LangfuseError::Api { status, message });
        }

        let resp_body: serde_json::Value = resp.json().await.map_err(LangfuseError::Network)?;

        let content_type = resp_body["contentType"]
            .as_str()
            .unwrap_or("application/octet-stream")
            .to_string();
        let download_url = resp_body["url"]
            .as_str()
            .ok_or_else(|| LangfuseError::Media("Missing url in response".into()))?;

        // Download the actual content
        let data_resp = self
            .inner
            .http_client
            .get(download_url)
            .send()
            .await
            .map_err(LangfuseError::Network)?;
        let data = data_resp
            .bytes()
            .await
            .map_err(LangfuseError::Network)?
            .to_vec();

        Ok(LangfuseMedia { content_type, data })
    }
}
