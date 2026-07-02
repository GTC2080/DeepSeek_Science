//! Model descriptors and capability metadata.

use serde::{Deserialize, Serialize};

/// Input or output modality supported by a model.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Modality {
    /// Plain text tokens.
    Text,
    /// Image data referenced by an artifact or future media store.
    Image,
    /// Structured tabular data referenced by an artifact.
    Table,
}

/// Provider-neutral capability description for routing and validation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Modalities the model can consume or emit.
    pub modalities: Vec<Modality>,
    /// Whether provider-side prompt-prefix caching is expected to be useful.
    pub supports_prompt_cache: bool,
    /// Optional context window in tokens when known.
    pub max_context_tokens: Option<u64>,
}

impl ModelCapabilities {
    /// Creates a conservative text-only capability profile.
    pub fn text_only(max_context_tokens: Option<u64>) -> Self {
        Self {
            modalities: vec![Modality::Text],
            supports_prompt_cache: false,
            max_context_tokens,
        }
    }
}

/// Stable description of a provider model available to the gateway.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelDescriptor {
    /// Provider identifier, such as `deepseek`.
    pub provider: String,
    /// Provider-specific model identifier.
    pub model: String,
    /// Model capabilities used by routing decisions.
    pub capabilities: ModelCapabilities,
}

impl ModelDescriptor {
    /// Creates a model descriptor from provider, model, and capabilities.
    pub fn new(
        provider: impl Into<String>,
        model: impl Into<String>,
        capabilities: ModelCapabilities,
    ) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            capabilities,
        }
    }
}
