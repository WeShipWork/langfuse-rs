//! HTTP client utilities: shared client builder and retry logic.

use backoff::ExponentialBackoffBuilder;
use langfuse_core::config::LangfuseConfig;
use langfuse_core::error::LangfuseError;
use std::time::Duration;

/// Build a `reqwest::Client` from config, applying timeout and additional headers.
pub fn build_http_client(config: &LangfuseConfig) -> reqwest::Client {
    let mut builder = reqwest::Client::builder().timeout(config.timeout);

    if let Some(ref extra) = config.additional_headers {
        let mut header_map = reqwest::header::HeaderMap::new();
        for (k, v) in extra {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                reqwest::header::HeaderValue::from_str(v),
            ) {
                header_map.insert(name, val);
            }
        }
        builder = builder.default_headers(header_map);
    }

    builder.build().unwrap_or_else(|_| reqwest::Client::new())
}

/// Execute an async operation with exponential backoff retry.
///
/// Retries on:
/// - 5xx server errors
/// - 429 Too Many Requests
/// - Network errors
///
/// Does NOT retry on:
/// - 4xx client errors (except 429)
/// - Auth errors
pub async fn retry_request<F, Fut, T>(max_retries: usize, f: F) -> Result<T, LangfuseError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, LangfuseError>>,
{
    let mut backoff = ExponentialBackoffBuilder::default()
        .with_initial_interval(Duration::from_millis(100))
        .with_max_interval(Duration::from_secs(30))
        .with_max_elapsed_time(None)
        .build();

    let mut attempt = 0;
    loop {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) if should_retry(&e) && attempt < max_retries => {
                attempt += 1;
                if let Some(delay) = backoff::backoff::Backoff::next_backoff(&mut backoff) {
                    tokio::time::sleep(delay).await;
                } else {
                    return Err(e);
                }
            }
            Err(e) => return Err(e),
        }
    }
}

/// Determine if an error is retryable.
fn should_retry(err: &LangfuseError) -> bool {
    match err {
        LangfuseError::Network(_) => true,
        LangfuseError::Api { status, .. } => *status == 429 || *status >= 500,
        _ => false,
    }
}
