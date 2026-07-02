//! Error types for provider-neutral model calls.

use crate::PrivacyPolicy;
use thiserror::Error;

/// Errors returned by model providers or gateway validation.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ModelError {
    /// The selected model does not support a required capability.
    #[error("model does not support required capability: {provider}/{model} missing {capability}")]
    UnsupportedCapability {
        /// Provider identifier.
        provider: String,
        /// Model identifier.
        model: String,
        /// Required capability name.
        capability: String,
    },

    /// No route can satisfy the request and routing constraints.
    #[error("no model route available: {reason}")]
    NoRouteAvailable {
        /// Human-readable routing reason.
        reason: String,
    },

    /// Routing would violate the request privacy policy.
    #[error("privacy policy violation for {policy:?}: {reason}")]
    PrivacyPolicyViolation {
        /// Privacy policy that blocked the route.
        policy: PrivacyPolicy,
        /// Human-readable violation reason.
        reason: String,
    },

    /// Provider or estimator usage data is internally inconsistent.
    #[error("invalid model usage data: {reason}")]
    InvalidUsageData {
        /// Human-readable validation reason.
        reason: String,
    },

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
