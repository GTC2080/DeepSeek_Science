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
pub struct CachePolicy {
    /// Stable prefix hash computed by the prompt compiler.
    pub stable_prefix_hash: Option<String>,
    /// Whether a provider-side cache may be used when available.
    pub allow_provider_cache: bool,
}

/// Privacy policy applied before routing a request to a provider.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PrivacyPolicy {
    /// Whether the request may leave the local runtime.
    pub allow_external_provider: bool,
    /// Whether provider log retention is acceptable for this request.
    pub allow_provider_retention: bool,
}

impl PrivacyPolicy {
    /// Creates a local-only policy for sensitive or unapproved requests.
    pub fn local_only() -> Self {
        Self {
            allow_external_provider: false,
            allow_provider_retention: false,
        }
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
