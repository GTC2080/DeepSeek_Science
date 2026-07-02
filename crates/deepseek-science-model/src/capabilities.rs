//! Model descriptors and capability metadata.

use serde::{Deserialize, Serialize};

/// Input or output modality supported by a model.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Modality {
    /// Plain text tokens.
    Text,
    /// Image data referenced by an artifact or future media store.
    Image,
    /// Audio data referenced by an artifact or future media store.
    Audio,
    /// Video data referenced by an artifact or future media store.
    Video,
    /// Generic file input referenced outside the request payload.
    File,
    /// Provider-neutral structured JSON content.
    StructuredJson,
    /// Structured tabular data referenced by an artifact.
    Table,
}

/// Provider-neutral capability description for routing and validation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Combined input and output modalities for simple summaries.
    pub modalities: Vec<Modality>,
    /// Modalities the model can consume.
    pub input_modalities: Vec<Modality>,
    /// Modalities the model can emit.
    pub output_modalities: Vec<Modality>,
    /// Whether the model can request tool calls.
    pub supports_tool_calling: bool,
    /// Whether the model can produce JSON or structured output.
    pub supports_structured_output: bool,
    /// Whether the provider can stream responses for this model.
    pub supports_streaming: bool,
    /// Whether provider-side prompt-prefix caching is expected to be useful.
    pub supports_prompt_cache: bool,
    /// Whether image understanding is supported.
    pub supports_vision: bool,
    /// Whether audio input or output is supported.
    pub supports_audio: bool,
    /// Whether video input or output is supported.
    pub supports_video: bool,
    /// Optional context window in tokens when known.
    pub max_context_tokens: Option<u64>,
    /// Optional output token limit when known.
    pub max_output_tokens: Option<u64>,
}

impl ModelCapabilities {
    /// Creates a capability profile from supported input and output modalities.
    pub fn new(input_modalities: Vec<Modality>, output_modalities: Vec<Modality>) -> Self {
        let modalities = combined_modalities(&input_modalities, &output_modalities);

        Self {
            supports_structured_output: output_modalities.contains(&Modality::StructuredJson),
            supports_vision: input_modalities.contains(&Modality::Image),
            supports_audio: modalities.contains(&Modality::Audio),
            supports_video: modalities.contains(&Modality::Video),
            modalities,
            input_modalities,
            output_modalities,
            supports_tool_calling: false,
            supports_streaming: false,
            supports_prompt_cache: false,
            max_context_tokens: None,
            max_output_tokens: None,
        }
    }

    /// Creates a conservative text-only capability profile.
    pub fn text_only(max_context_tokens: Option<u64>) -> Self {
        let mut capabilities = Self::new(vec![Modality::Text], vec![Modality::Text]);
        capabilities.max_context_tokens = max_context_tokens;
        capabilities
    }
}

/// Stable description of a provider model available to the gateway.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelDescriptor {
    /// Provider identifier, such as `deepseek`.
    pub provider: String,
    /// Provider-specific model identifier.
    pub model: String,
    /// Human-readable model name for logs, diagnostics, and future UI shells.
    pub display_name: String,
    /// Model capabilities used by routing decisions.
    pub capabilities: ModelCapabilities,
    /// Optional provider-neutral notes about descriptor status or limits.
    pub notes: Option<String>,
}

impl ModelDescriptor {
    /// Creates a model descriptor from provider, model, and capabilities.
    pub fn new(
        provider: impl Into<String>,
        model: impl Into<String>,
        capabilities: ModelCapabilities,
    ) -> Self {
        let model = model.into();
        Self {
            provider: provider.into(),
            display_name: model.clone(),
            model,
            capabilities,
            notes: None,
        }
    }

    /// Sets a human-readable display name.
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = display_name.into();
        self
    }

    /// Sets optional notes for placeholder status, limits, or routing hints.
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

fn combined_modalities(
    input_modalities: &[Modality],
    output_modalities: &[Modality],
) -> Vec<Modality> {
    let mut modalities = input_modalities.to_vec();
    for modality in output_modalities {
        if !modalities.contains(modality) {
            modalities.push(modality.clone());
        }
    }
    modalities
}

#[cfg(test)]
mod tests {
    use super::{Modality, ModelCapabilities, ModelDescriptor};

    #[test]
    fn capabilities_can_represent_cached_tool_capable_text_model() {
        let capabilities = ModelCapabilities {
            modalities: vec![Modality::Text],
            input_modalities: vec![Modality::Text],
            output_modalities: vec![Modality::Text],
            supports_tool_calling: true,
            supports_structured_output: true,
            supports_streaming: true,
            supports_prompt_cache: true,
            supports_vision: false,
            supports_audio: false,
            supports_video: false,
            max_context_tokens: Some(128_000),
            max_output_tokens: Some(8_000),
        };

        assert_eq!(capabilities.input_modalities, vec![Modality::Text]);
        assert_eq!(capabilities.output_modalities, vec![Modality::Text]);
        assert!(capabilities.supports_tool_calling);
        assert!(capabilities.supports_prompt_cache);
        assert_eq!(capabilities.max_context_tokens, Some(128_000));
        assert_eq!(capabilities.max_output_tokens, Some(8_000));
    }

    #[test]
    fn capabilities_can_represent_multimodal_vision_model() {
        let capabilities = ModelCapabilities::new(
            vec![Modality::Text, Modality::Image],
            vec![Modality::Text, Modality::StructuredJson],
        );

        assert_eq!(
            capabilities.modalities,
            vec![Modality::Text, Modality::Image, Modality::StructuredJson]
        );
        assert!(capabilities.input_modalities.contains(&Modality::Image));
        assert!(capabilities
            .output_modalities
            .contains(&Modality::StructuredJson));
        assert!(capabilities.supports_structured_output);
        assert!(capabilities.supports_vision);
    }

    #[test]
    fn descriptor_records_display_name_and_notes() {
        let descriptor = ModelDescriptor::new(
            "provider",
            "model-id",
            ModelCapabilities::text_only(Some(16_000)),
        )
        .with_display_name("Readable Model")
        .with_notes("placeholder");

        assert_eq!(descriptor.provider, "provider");
        assert_eq!(descriptor.model, "model-id");
        assert_eq!(descriptor.display_name, "Readable Model");
        assert_eq!(descriptor.notes, Some("placeholder".to_owned()));
    }
}
