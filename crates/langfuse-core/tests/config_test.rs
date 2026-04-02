use langfuse_core::config::LangfuseConfig;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

const CONFIG_ENV_KEYS: [&str; 5] = [
    "LANGFUSE_PUBLIC_KEY",
    "LANGFUSE_SECRET_KEY",
    "LANGFUSE_BASE_URL",
    "LANGFUSE_TIMEOUT",
    "LANGFUSE_SAMPLE_RATE",
];

struct EnvGuard {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn capture(keys: &'static [&'static str]) -> Self {
        let saved = keys
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            unsafe {
                if let Some(value) = value {
                    std::env::set_var(key, value);
                } else {
                    std::env::remove_var(key);
                }
            }
        }
    }
}

#[test]
fn test_config_defaults() {
    let config = LangfuseConfig::builder()
        .public_key("pk-lf-test")
        .secret_key("sk-lf-test")
        .build()
        .unwrap();

    assert_eq!(config.public_key, "pk-lf-test");
    assert_eq!(config.secret_key, "sk-lf-test");
    assert_eq!(config.base_url, "https://cloud.langfuse.com");
    assert_eq!(config.timeout, Duration::from_secs(5));
    assert_eq!(config.flush_at, 512);
    assert_eq!(config.flush_interval, Duration::from_secs(5));
    assert_eq!(config.sample_rate, 1.0);
    assert!(config.tracing_enabled);
    assert!(!config.debug);
}

#[test]
fn test_config_from_env() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env_guard = EnvGuard::capture(&CONFIG_ENV_KEYS);

    // Set env vars
    unsafe {
        std::env::set_var("LANGFUSE_PUBLIC_KEY", "pk-lf-env");
        std::env::set_var("LANGFUSE_SECRET_KEY", "sk-lf-env");
        std::env::set_var("LANGFUSE_BASE_URL", "https://custom.langfuse.com");
        std::env::set_var("LANGFUSE_TIMEOUT", "10");
        std::env::set_var("LANGFUSE_SAMPLE_RATE", "0.5");
    }

    let config = LangfuseConfig::from_env().unwrap();
    assert_eq!(config.public_key, "pk-lf-env");
    assert_eq!(config.secret_key, "sk-lf-env");
    assert_eq!(config.base_url, "https://custom.langfuse.com");
    assert_eq!(config.timeout, Duration::from_secs(10));
    assert_eq!(config.sample_rate, 0.5);
}

#[test]
fn test_config_missing_keys() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _env_guard = EnvGuard::capture(&CONFIG_ENV_KEYS);

    unsafe {
        std::env::remove_var("LANGFUSE_PUBLIC_KEY");
        std::env::remove_var("LANGFUSE_SECRET_KEY");
    }
    let result = LangfuseConfig::from_env();
    assert!(result.is_err());
}

#[test]
fn test_config_basic_auth_header() {
    let config = LangfuseConfig::builder()
        .public_key("pk-lf-test")
        .secret_key("sk-lf-test")
        .build()
        .unwrap();

    let header = config.basic_auth_header();
    // Basic base64("pk-lf-test:sk-lf-test")
    assert!(header.starts_with("Basic "));
}

#[test]
fn test_config_otel_endpoint() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .base_url("https://example.com")
        .build()
        .unwrap();

    assert_eq!(
        config.otel_endpoint(),
        "https://example.com/api/public/otel/v1/traces"
    );
}

#[test]
fn test_config_otel_endpoint_trailing_slash() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .base_url("https://example.com/")
        .build()
        .unwrap();

    assert_eq!(
        config.otel_endpoint(),
        "https://example.com/api/public/otel/v1/traces"
    );
}

#[test]
fn test_config_api_base_url() {
    let config = LangfuseConfig::builder()
        .public_key("pk")
        .secret_key("sk")
        .build()
        .unwrap();

    assert_eq!(
        config.api_base_url(),
        "https://cloud.langfuse.com/api/public"
    );
}
