//! Error type shared by every platform call.

/// Why a platform call failed. The UI shows it as text and never panics on it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlatformError {
    /// No token, or the platform rejected it (401).
    #[error("authentication failed: {0}")]
    Auth(String),
    /// The token is valid but not allowed to do this (403 without a rate limit).
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// 404: the repo, PR or issue does not exist (or is private).
    #[error("not found: {0}")]
    NotFound(String),
    /// Rate limited. `retry_after_secs` is the wait the platform asked for, if any.
    #[error("rate limited{}", .retry_after_secs.map(|s| format!(", retry in {s}s")).unwrap_or_default())]
    RateLimited {
        /// Seconds to wait before the next call.
        retry_after_secs: Option<u64>,
    },
    /// Any other non-success status.
    #[error("HTTP {status}: {message}")]
    Http {
        /// HTTP status code.
        status: u16,
        /// Short body excerpt.
        message: String,
    },
    /// The connection failed (DNS, TLS, timeout…).
    #[error("network error: {0}")]
    Network(String),
    /// The response was not what the adapter expected.
    #[error("unexpected response: {0}")]
    Parse(String),
    /// This platform does not offer the action.
    #[error("not supported by this platform: {0}")]
    Unsupported(&'static str),
}

/// Result alias used across the crate.
pub type Result<T> = std::result::Result<T, PlatformError>;
