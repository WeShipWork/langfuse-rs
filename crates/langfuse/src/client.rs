//! The top-level Langfuse client that ties together all managers and the
//! tracing pipeline.

use dashmap::DashMap;
use std::sync::OnceLock;

use langfuse_core::config::LangfuseConfig;
use langfuse_core::error::LangfuseError;
use langfuse_core::types::ObservationType;
use serde::Serialize;

use crate::datasets::manager::DatasetManager;
use crate::langfuse_tracing::exporter::LangfuseTracing;
use crate::langfuse_tracing::generation::LangfuseGeneration;
use crate::langfuse_tracing::span::LangfuseSpan;
use crate::media::manager::MediaManager;
use crate::prompts::manager::PromptManager;
use crate::scoring::manager::ScoreManager;

/// The main Langfuse client. Holds all managers and the tracing pipeline.
pub struct Langfuse {
    config: LangfuseConfig,
    tracing: Option<LangfuseTracing>,
    /// Prompt management: fetching, caching, and compilation.
    pub prompts: PromptManager,
    /// Score creation and batched submission.
    pub scores: ScoreManager,
    /// Dataset CRUD operations.
    pub datasets: DatasetManager,
    /// Media upload and fetch.
    pub media: MediaManager,
}

impl std::fmt::Debug for Langfuse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Langfuse")
            .field("config", &self.config)
            .field("tracing", &self.tracing.is_some())
            .finish()
    }
}

/// Global singleton.
static INSTANCE: OnceLock<Langfuse> = OnceLock::new();

/// Named instances for multi-environment or multi-project usage.
static NAMED_INSTANCES: OnceLock<DashMap<String, Langfuse>> = OnceLock::new();

impl Langfuse {
    /// Create a new Langfuse client from config.
    pub fn new(config: LangfuseConfig) -> Result<Self, LangfuseError> {
        let tracing = if config.tracing_enabled {
            Some(LangfuseTracing::builder(&config).build()?)
        } else {
            None
        };

        Ok(Self {
            prompts: PromptManager::new(&config),
            scores: ScoreManager::new(&config),
            datasets: DatasetManager::new(&config),
            media: MediaManager::new(&config),
            tracing,
            config,
        })
    }

    /// Create from environment variables.
    pub fn from_env() -> Result<Self, LangfuseError> {
        let config = LangfuseConfig::from_env()?;
        Self::new(config)
    }

    /// Initialize the global singleton.
    pub fn init(config: LangfuseConfig) -> Result<&'static Langfuse, LangfuseError> {
        let instance = Self::new(config)?;
        INSTANCE.set(instance).map_err(|_| {
            LangfuseError::Config(langfuse_core::error::ConfigError::InvalidValue {
                field: "global".into(),
                message: "Langfuse already initialized".into(),
            })
        })?;
        Ok(INSTANCE.get().unwrap())
    }

    /// Get the global singleton (panics if not initialized).
    pub fn get() -> &'static Langfuse {
        INSTANCE
            .get()
            .expect("Langfuse not initialized. Call Langfuse::init() first.")
    }

    /// Try to get the global singleton.
    pub fn try_get() -> Option<&'static Langfuse> {
        INSTANCE.get()
    }

    /// Initialize a named instance.
    ///
    /// Named instances are independent of the global singleton and allow
    /// multiple Langfuse clients (e.g. for different projects or environments)
    /// to coexist.
    pub fn init_named(name: &str, config: LangfuseConfig) -> Result<(), LangfuseError> {
        let instance = Self::new(config)?;
        let map = NAMED_INSTANCES.get_or_init(DashMap::new);
        map.insert(name.to_string(), instance);
        Ok(())
    }

    /// Get a named instance by name.
    ///
    /// Returns `None` if no instance with the given name has been initialized.
    pub fn get_named(name: &str) -> Option<dashmap::mapref::one::Ref<'static, String, Langfuse>> {
        NAMED_INSTANCES.get().and_then(|map| map.get(name))
    }

    /// Try to get a named instance, returning an error if not found.
    pub fn try_get_named(
        name: &str,
    ) -> Result<dashmap::mapref::one::Ref<'static, String, Langfuse>, LangfuseError> {
        Self::get_named(name).ok_or_else(|| {
            LangfuseError::Config(langfuse_core::error::ConfigError::InvalidValue {
                field: "name".into(),
                message: format!("Named instance '{name}' not initialized"),
            })
        })
    }

    /// Get the config.
    pub fn config(&self) -> &LangfuseConfig {
        &self.config
    }

    /// Check if authentication is valid by making a test API call.
    pub async fn auth_check(&self) -> Result<(), LangfuseError> {
        let url = format!("{}/projects", self.config.api_base_url());
        let resp = reqwest::Client::new()
            .get(&url)
            .header("Authorization", self.config.basic_auth_header())
            .send()
            .await
            .map_err(LangfuseError::Network)?;

        if resp.status() == 401 || resp.status() == 403 {
            return Err(LangfuseError::Auth);
        }
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(LangfuseError::Api { status, message });
        }
        Ok(())
    }

    /// Flush all pending data (scores, traces).
    pub async fn flush(&self) -> Result<(), LangfuseError> {
        self.scores.flush().await?;
        if let Some(ref tracing) = self.tracing {
            tracing
                .shutdown()
                .map_err(|e| LangfuseError::Otel(e.to_string()))?;
        }
        Ok(())
    }

    /// Shut down the client, flushing all pending data.
    pub async fn shutdown(&self) -> Result<(), LangfuseError> {
        self.flush().await
    }

    // ------------------------------------------------------------------
    // Convenience tracing methods
    // ------------------------------------------------------------------

    /// Start a new span.
    pub fn start_span(&self, name: &str) -> LangfuseSpan {
        LangfuseSpan::start(name)
    }

    /// Start a new generation.
    pub fn start_generation(&self, name: &str) -> LangfuseGeneration {
        LangfuseGeneration::start(name)
    }

    /// Start a new span with a specific observation type.
    pub fn start_span_with_type(&self, name: &str, obs_type: ObservationType) -> LangfuseSpan {
        LangfuseSpan::start_with_type(name, obs_type)
    }

    /// Create a standalone root-level event observation.
    ///
    /// Events are zero-duration observations that carry input data.
    /// The event span is created and immediately ended.
    pub fn create_event(&self, name: &str, input: &impl Serialize) {
        let span = LangfuseSpan::start_with_type(name, ObservationType::Event);
        span.set_input(input);
        span.end();
    }

    /// Generate the Langfuse UI URL for a trace.
    pub fn get_trace_url(&self, trace_id: &str) -> String {
        format!(
            "{}/trace/{}",
            self.config.base_url.trim_end_matches('/'),
            trace_id
        )
    }

    /// Register the internal tracer provider as the OpenTelemetry global provider.
    ///
    /// This **must** be called once after [`Langfuse::new`] for spans created via
    /// [`LangfuseSpan::start`] (which uses `opentelemetry::global::tracer("langfuse")`)
    /// to be exported through the Langfuse OTLP pipeline.
    ///
    /// Calling this more than once replaces the previous global provider.
    pub fn register_tracing(&self) {
        if let Some(ref tracing) = self.tracing {
            opentelemetry::global::set_tracer_provider(tracing.provider().clone());
        }
    }
}

impl Drop for Langfuse {
    fn drop(&mut self) {
        // Attempt async flush using block_in_place if in multi-thread runtime.
        // Skip gracefully if in current-thread runtime or no runtime.
        if let Ok(handle) = tokio::runtime::Handle::try_current()
            && handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread
        {
            tokio::task::block_in_place(|| {
                handle.block_on(async {
                    let _ = self.scores.shutdown().await;
                    // Tracing provider shutdown handled separately
                });
            });
        }
    }
}
