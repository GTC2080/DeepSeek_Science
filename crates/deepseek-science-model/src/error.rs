//! Error types for provider-neutral model calls.

use thiserror::Error;

/// Errors returned by model providers or gateway validation.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ModelError {
    /// The selected provider is not available in the current runtime.
    #[error("model provider is unavailable: {provider}")]
    ProviderUnavailable {
        /// Provider identifier.
        provider: String,
    },

    /// The request cannot be represented by the selected model.
    #[error("model request is invalid: {reason}")]
    InvalidRequest {
        /// Human-readable validation reason.
        reason: String,
    },

    /// The provider returned an error without a provider-specific adapter.
    #[error("model provider failed: {reason}")]
    ProviderFailed {
        /// Human-readable failure reason.
        reason: String,
    },
}
