//! Provider-neutral request model.

use crate::ModelDescriptor;
use serde::{Deserialize, Serialize};

/// Role attached to a model message.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MessageRole {
    /// System or developer instruction.
    System,
    /// User-authored input.
    User,
    /// Assistant-authored output.
    Assistant,
    /// Tool result supplied back to the model.
    Tool,
}

/// One content part inside a model message.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MessagePart {
    /// Inline UTF-8 text.
    Text(String),
    /// Reference to an artifact managed outside the request payload.
    ArtifactRef {
        /// Artifact identifier as a string to avoid coupling model requests to storage.
        artifact_id: String,
        /// Declared modality of the referenced artifact.
        modality: crate::Modality,
    },
}

/// A provider-neutral model message.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelMessage {
    /// Message role.
    pub role: MessageRole,
    /// Ordered message parts.
    pub parts: Vec<MessagePart>,
}

impl ModelMessage {
    /// Creates a text message with one text part.
    pub fn text(role: MessageRole, text: impl Into<String>) -> Self {
        Self {
            role,
            parts: vec![MessagePart::Text(text.into())],
        }
    }
}

/// Prompt-cache preferences for a model request.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum CachePolicy {
    /// Let the provider adapter use its normal cache behavior.
    #[default]
    UseProviderDefault,
    /// Prefer provider-side prompt caching when available.
    PreferCache {
        /// Stable prefix hash computed by the prompt compiler, when available.
        stable_prefix_hash: Option<String>,
    },
    /// Do not use provider-side prompt caching for this request.
    BypassCache,
}

/// Privacy policy applied before routing a request to a provider.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum PrivacyPolicy {
    /// Standard provider routing is allowed.
    #[default]
    Standard,
    /// The request must not use external network providers.
    NoExternalNetwork,
    /// The request must stay within local-only model routes.
    LocalOnly,
}

impl PrivacyPolicy {
    /// Creates a local-only policy for sensitive or unapproved requests.
    pub fn local_only() -> Self {
        Self::LocalOnly
    }
}

/// Provider-neutral request passed to a selected model.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelRequest {
    /// Target model descriptor selected by routing.
    pub target: ModelDescriptor,
    /// Ordered messages sent to the model.
    pub messages: Vec<ModelMessage>,
    /// Cache policy for stable prompt prefixes.
    pub cache_policy: CachePolicy,
    /// Privacy policy that routing and adapters must honor.
    pub privacy_policy: PrivacyPolicy,
}

impl ModelRequest {
    /// Creates a request with conservative cache and privacy defaults.
    pub fn new(target: ModelDescriptor, messages: Vec<ModelMessage>) -> Self {
        Self {
            target,
            messages,
            cache_policy: CachePolicy::default(),
            privacy_policy: PrivacyPolicy::local_only(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CachePolicy, PrivacyPolicy};

    #[test]
    fn cache_policy_values_can_be_constructed() {
        assert_eq!(CachePolicy::UseProviderDefault, CachePolicy::default());
        assert_eq!(
            CachePolicy::PreferCache {
                stable_prefix_hash: Some("prefix".to_owned())
            },
            CachePolicy::PreferCache {
                stable_prefix_hash: Some("prefix".to_owned())
            }
        );
        assert_eq!(CachePolicy::BypassCache, CachePolicy::BypassCache);
    }

    #[test]
    fn privacy_policy_values_can_be_constructed() {
        assert_eq!(PrivacyPolicy::Standard, PrivacyPolicy::Standard);
        assert_eq!(
            PrivacyPolicy::NoExternalNetwork,
            PrivacyPolicy::NoExternalNetwork
        );
        assert_eq!(PrivacyPolicy::LocalOnly, PrivacyPolicy::local_only());
    }
}
