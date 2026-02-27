use reqwest::Client;
use std::time::Duration;

/// Build an HTTP client that accepts self-signed certs.
pub fn build_client() -> Client {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .pool_max_idle_per_host(100)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build reqwest client")
}
