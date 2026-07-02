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

    /// Returns a display name for diagnostics and future UI shells.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Chat => "DeepSeek Chat",
            Self::Reasoner => "DeepSeek Reasoner",
        }
    }

    /// Returns a provider-neutral descriptor.
    pub fn descriptor(self) -> ModelDescriptor {
        let mut capabilities = ModelCapabilities::text_only(None);
        capabilities.supports_prompt_cache = true;

        ModelDescriptor::new("deepseek", self.model_id(), capabilities)
            .with_display_name(self.display_name())
            .with_notes("Phase 1 placeholder descriptor; no network client or API key handling.")
    }
}

#[cfg(test)]
mod tests {
    use super::DeepSeekModel;

    #[test]
    fn placeholder_descriptor_can_be_constructed() {
        let descriptor = DeepSeekModel::Reasoner.descriptor();

        assert_eq!(descriptor.provider, "deepseek");
        assert_eq!(descriptor.model, "deepseek-reasoner");
        assert_eq!(descriptor.display_name, "DeepSeek Reasoner");
        assert!(descriptor.capabilities.supports_prompt_cache);
        assert!(descriptor.notes.is_some());
    }
}
