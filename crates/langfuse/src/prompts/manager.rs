//! Prompt manager: fetching from the Langfuse API, caching, and compilation.

use std::collections::HashMap;
use std::time::Duration;

use futures::future::join_all;
use langfuse_core::LangfuseConfig;
use langfuse_core::error::LangfuseError;
use langfuse_core::types::{ChatMessage, PromptType};
use serde::Deserialize;

use crate::prompts::cache::PromptCache;
use crate::prompts::chat::ChatPromptClient;
use crate::prompts::text::TextPromptClient;
use crate::prompts::types::Prompt;

/// Default cache TTL: 60 seconds.
const DEFAULT_CACHE_TTL_SECS: u64 = 60;

/// Raw API response for a prompt.
#[derive(Debug, Deserialize)]
struct PromptApiResponse {
    name: String,
    version: i32,
    prompt: serde_json::Value,
    #[serde(default)]
    config: serde_json::Value,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(rename = "type")]
    prompt_type: PromptType,
}

/// Manages prompt fetching, caching, and compilation.
pub struct PromptManager {
    config: LangfuseConfig,
    http_client: reqwest::Client,
    cache: PromptCache,
}

impl std::fmt::Debug for PromptManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PromptManager")
            .field("config", &self.config)
            .finish()
    }
}

impl PromptManager {
    /// Create a new `PromptManager` from the given configuration.
    pub fn new(config: &LangfuseConfig) -> Self {
        let http_client = crate::http::build_http_client(config);

        Self {
            config: config.clone(),
            http_client,
            cache: PromptCache::new(Duration::from_secs(DEFAULT_CACHE_TTL_SECS)),
        }
    }

    /// Build the cache key for a prompt lookup.
    fn cache_key(name: &str, version: Option<i32>, label: Option<&str>) -> String {
        match (version, label) {
            (Some(v), _) => format!("{name}:{v}"),
            (None, Some(l)) => format!("{name}:{l}"),
            (None, None) => format!("{name}:latest"),
        }
    }

    /// Fetch a text prompt from the API (with caching).
    ///
    /// 1. Check the cache.
    /// 2. On miss, `GET /api/public/v2/prompts/{name}` with optional `version` / `label` query
    ///    params.
    /// 3. Parse the response into a [`TextPromptClient`].
    /// 4. Store in cache and return.
    pub async fn get_text_prompt(
        &self,
        name: &str,
        version: Option<i32>,
        label: Option<&str>,
    ) -> Result<TextPromptClient, LangfuseError> {
        let key = Self::cache_key(name, version, label);

        if let Some(cached) = self.cache.get_text(&key) {
            return Ok(cached);
        }

        let resp = match self.fetch_prompt(name, version, label).await {
            Ok(resp) => resp,
            Err(err) => {
                // Fallback: return expired cache entry if available.
                if let Some(mut cached) = self.cache.get_text_expired(&key) {
                    cached.is_fallback = true;
                    return Ok(cached);
                }
                return Err(err);
            }
        };

        if resp.prompt_type != PromptType::Text {
            return Err(LangfuseError::PromptNotFound {
                name: name.to_owned(),
            });
        }

        let template = resp
            .prompt
            .as_str()
            .ok_or_else(|| LangfuseError::PromptNotFound {
                name: name.to_owned(),
            })?
            .to_owned();

        let prompt = TextPromptClient {
            name: resp.name,
            version: resp.version,
            template,
            config: resp.config,
            labels: resp.labels,
            tags: resp.tags,
            is_fallback: false,
        };

        self.cache.put_text(&key, prompt.clone());
        Ok(prompt)
    }

    /// Fetch a chat prompt from the API (with caching).
    pub async fn get_chat_prompt(
        &self,
        name: &str,
        version: Option<i32>,
        label: Option<&str>,
    ) -> Result<ChatPromptClient, LangfuseError> {
        let key = Self::cache_key(name, version, label);

        if let Some(cached) = self.cache.get_chat(&key) {
            return Ok(cached);
        }

        let resp = match self.fetch_prompt(name, version, label).await {
            Ok(resp) => resp,
            Err(err) => {
                // Fallback: return expired cache entry if available.
                if let Some(mut cached) = self.cache.get_chat_expired(&key) {
                    cached.is_fallback = true;
                    return Ok(cached);
                }
                return Err(err);
            }
        };

        if resp.prompt_type != PromptType::Chat {
            return Err(LangfuseError::PromptNotFound {
                name: name.to_owned(),
            });
        }

        let messages: Vec<ChatMessage> =
            serde_json::from_value(resp.prompt.clone()).map_err(|_| {
                LangfuseError::PromptNotFound {
                    name: name.to_owned(),
                }
            })?;

        let prompt = ChatPromptClient {
            name: resp.name,
            version: resp.version,
            messages,
            config: resp.config,
            labels: resp.labels,
            tags: resp.tags,
            is_fallback: false,
        };

        self.cache.put_chat(&key, prompt.clone());
        Ok(prompt)
    }

    /// Clear the prompt cache.
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    // ── CRUD operations ──────────────────────────────────────────────

    /// Create a new text prompt via the Langfuse API.
    ///
    /// POSTs to `/v2/prompts` and invalidates any cached entries for this prompt name.
    pub async fn create_text_prompt(
        &self,
        name: &str,
        template: &str,
        labels: Option<&[&str]>,
        tags: Option<&[&str]>,
        config: Option<&serde_json::Value>,
    ) -> Result<TextPromptClient, LangfuseError> {
        let url = format!("{}/v2/prompts", self.config.api_base_url());

        let mut body = serde_json::json!({
            "name": name,
            "prompt": template,
            "type": "text"
        });

        if let Some(l) = labels {
            body["labels"] = serde_json::json!(l);
        }
        if let Some(t) = tags {
            body["tags"] = serde_json::json!(t);
        }
        if let Some(c) = config {
            body["config"] = c.clone();
        }

        let resp = self
            .http_client
            .post(&url)
            .header("Authorization", self.config.basic_auth_header())
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(LangfuseError::Auth);
        }
        if !status.is_success() {
            return Err(LangfuseError::Api {
                status: status.as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let api_resp = resp.json::<PromptApiResponse>().await?;

        let result_template = api_resp.prompt.as_str().unwrap_or(template).to_owned();

        let prompt = TextPromptClient {
            name: api_resp.name,
            version: api_resp.version,
            template: result_template,
            config: api_resp.config,
            labels: api_resp.labels,
            tags: api_resp.tags,
            is_fallback: false,
        };

        // Invalidate all cached entries for this prompt name.
        self.cache.invalidate_by_prefix(&format!("{name}:"));

        Ok(prompt)
    }

    /// Create a new chat prompt via the Langfuse API.
    ///
    /// POSTs to `/v2/prompts` and invalidates any cached entries for this prompt name.
    pub async fn create_chat_prompt(
        &self,
        name: &str,
        messages: &[ChatMessage],
        labels: Option<&[&str]>,
        tags: Option<&[&str]>,
        config: Option<&serde_json::Value>,
    ) -> Result<ChatPromptClient, LangfuseError> {
        let url = format!("{}/v2/prompts", self.config.api_base_url());

        let mut body = serde_json::json!({
            "name": name,
            "prompt": messages,
            "type": "chat"
        });

        if let Some(l) = labels {
            body["labels"] = serde_json::json!(l);
        }
        if let Some(t) = tags {
            body["tags"] = serde_json::json!(t);
        }
        if let Some(c) = config {
            body["config"] = c.clone();
        }

        let resp = self
            .http_client
            .post(&url)
            .header("Authorization", self.config.basic_auth_header())
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(LangfuseError::Auth);
        }
        if !status.is_success() {
            return Err(LangfuseError::Api {
                status: status.as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let api_resp = resp.json::<PromptApiResponse>().await?;

        let result_messages: Vec<ChatMessage> =
            serde_json::from_value(api_resp.prompt.clone()).unwrap_or_else(|_| messages.to_vec());

        let prompt = ChatPromptClient {
            name: api_resp.name,
            version: api_resp.version,
            messages: result_messages,
            config: api_resp.config,
            labels: api_resp.labels,
            tags: api_resp.tags,
            is_fallback: false,
        };

        // Invalidate all cached entries for this prompt name.
        self.cache.invalidate_by_prefix(&format!("{name}:"));

        Ok(prompt)
    }

    /// Update a prompt's labels via the Langfuse API.
    ///
    /// PATCHes `/v2/prompts/{name}` and invalidates cached entries for this prompt name.
    pub async fn update_prompt(
        &self,
        name: &str,
        version: i32,
        new_labels: &[&str],
    ) -> Result<(), LangfuseError> {
        let url = format!("{}/v2/prompts/{}", self.config.api_base_url(), name);

        let body = serde_json::json!({
            "version": version,
            "labels": new_labels
        });

        let resp = self
            .http_client
            .patch(&url)
            .header("Authorization", self.config.basic_auth_header())
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(LangfuseError::Auth);
        }
        if !status.is_success() {
            return Err(LangfuseError::Api {
                status: status.as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        // Invalidate all cached entries for this prompt name.
        self.cache.invalidate_by_prefix(&format!("{name}:"));

        Ok(())
    }

    /// Fetch a prompt (text or chat) and return it wrapped in the [`Prompt`] enum.
    ///
    /// Checks the cache first, then fetches from the API. The response type field
    /// determines whether a [`Prompt::Text`] or [`Prompt::Chat`] is returned.
    pub async fn get_prompt(
        &self,
        name: &str,
        version: Option<i32>,
        label: Option<&str>,
    ) -> Result<Prompt, LangfuseError> {
        let key = Self::cache_key(name, version, label);

        // Check text cache first, then chat cache.
        if let Some(cached) = self.cache.get_text(&key) {
            return Ok(Prompt::Text(cached));
        }
        if let Some(cached) = self.cache.get_chat(&key) {
            return Ok(Prompt::Chat(cached));
        }

        let result = self
            .fetch_and_cache_prompt(name, version, label, &key)
            .await;

        match result {
            Ok(prompt) => Ok(prompt),
            Err(err) => {
                // Fallback: try expired cache entries.
                if let Some(mut cached) = self.cache.get_text_expired(&key) {
                    cached.is_fallback = true;
                    return Ok(Prompt::Text(cached));
                }
                if let Some(mut cached) = self.cache.get_chat_expired(&key) {
                    cached.is_fallback = true;
                    return Ok(Prompt::Chat(cached));
                }
                Err(err)
            }
        }
    }

    /// Fetch multiple prompts concurrently, returning those that succeed.
    ///
    /// Failed fetches are silently excluded from the result.
    pub async fn fetch_prompts(&self, names: &[&str]) -> HashMap<String, Prompt> {
        let futures: Vec<_> = names
            .iter()
            .map(|name| async move {
                let result = self.get_prompt(name, None, None).await;
                ((*name).to_owned(), result)
            })
            .collect();

        let results = join_all(futures).await;

        results
            .into_iter()
            .filter_map(|(name, result)| result.ok().map(|prompt| (name, prompt)))
            .collect()
    }

    // ── Private helpers ──────────────────────────────────────────────

    /// Fetch a prompt from the API, parse it, cache it, and return as [`Prompt`].
    async fn fetch_and_cache_prompt(
        &self,
        name: &str,
        version: Option<i32>,
        label: Option<&str>,
        key: &str,
    ) -> Result<Prompt, LangfuseError> {
        let resp = self.fetch_prompt(name, version, label).await?;

        match resp.prompt_type {
            PromptType::Text => {
                let template = resp
                    .prompt
                    .as_str()
                    .ok_or_else(|| LangfuseError::PromptNotFound {
                        name: name.to_owned(),
                    })?
                    .to_owned();

                let prompt = TextPromptClient {
                    name: resp.name,
                    version: resp.version,
                    template,
                    config: resp.config,
                    labels: resp.labels,
                    tags: resp.tags,
                    is_fallback: false,
                };

                self.cache.put_text(key, prompt.clone());
                Ok(Prompt::Text(prompt))
            }
            PromptType::Chat => {
                let messages: Vec<ChatMessage> = serde_json::from_value(resp.prompt.clone())
                    .map_err(|_| LangfuseError::PromptNotFound {
                        name: name.to_owned(),
                    })?;

                let prompt = ChatPromptClient {
                    name: resp.name,
                    version: resp.version,
                    messages,
                    config: resp.config,
                    labels: resp.labels,
                    tags: resp.tags,
                    is_fallback: false,
                };

                self.cache.put_chat(key, prompt.clone());
                Ok(Prompt::Chat(prompt))
            }
        }
    }

    /// Perform the HTTP request to the Langfuse prompt API.
    async fn fetch_prompt(
        &self,
        name: &str,
        version: Option<i32>,
        label: Option<&str>,
    ) -> Result<PromptApiResponse, LangfuseError> {
        let url = format!("{}/v2/prompts/{}", self.config.api_base_url(), name);

        let mut req = self
            .http_client
            .get(&url)
            .header("Authorization", self.config.basic_auth_header());

        if let Some(v) = version {
            req = req.query(&[("version", v.to_string())]);
        }
        if let Some(l) = label {
            req = req.query(&[("label", l)]);
        }

        let resp = req.send().await?;

        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(LangfuseError::PromptNotFound {
                name: name.to_owned(),
            });
        }
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(LangfuseError::Auth);
        }
        if !status.is_success() {
            return Err(LangfuseError::Api {
                status: status.as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let body = resp.json::<PromptApiResponse>().await?;
        Ok(body)
    }
}
