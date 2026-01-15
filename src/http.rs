//! Shared HTTP client for API requests
//!
//! Provides a singleton HTTP agent with connection pooling and timeout configuration.

use std::sync::LazyLock;
use std::time::Duration;

/// Global shared HTTP agent with connection pooling
///
/// Using a static agent allows connection reuse between requests,
/// significantly improving performance for multiple API calls.
pub static HTTP_AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .new_agent()
});

/// Get a reference to the shared HTTP agent
#[inline]
pub fn agent() -> &'static ureq::Agent {
    &HTTP_AGENT
}
