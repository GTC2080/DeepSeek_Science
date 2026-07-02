//! DeepSeek model descriptors for future provider integration.

use deepseek_science_model::{ModelCapabilities, ModelDescriptor};
use serde::{Deserialize, Serialize};

/// DeepSeek models known to the Phase 1 descriptor layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DeepSeekModel {
    /// General chat model descriptor.
    Chat,
    /// Reasoning-oriented model descriptor.
    Reasoner,
}

impl DeepSeekModel {
    /// Returns the provider-specific model identifier.
    pub fn model_id(self) -> &'static str {
        match self {
            Self::Chat => "deepseek-chat",
            Self::Reasoner => "deepseek-reasoner",
        }
    }

    /// Returns a provider-neutral descriptor.
    pub fn descriptor(self) -> ModelDescriptor {
        ModelDescriptor::new(
            "deepseek",
            self.model_id(),
            ModelCapabilities {
                modalities: vec![deepseek_science_model::Modality::Text],
                supports_prompt_cache: true,
                max_context_tokens: None,
            },
        )
    }
}
