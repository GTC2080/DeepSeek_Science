//! Provider trait for model adapters.

use crate::{ModelDescriptor, ModelError, ModelRequest, ModelResponse};

/// Synchronous model provider interface for the Phase 1 skeleton.
///
/// The trait is deliberately sync to avoid an async runtime dependency before
/// real provider integration exists.
pub trait ModelProvider {
    /// Returns the model this provider instance serves.
    fn descriptor(&self) -> &ModelDescriptor;

    /// Generates a response for a provider-neutral request.
    fn generate(&self, request: ModelRequest) -> Result<ModelResponse, ModelError>;
}
