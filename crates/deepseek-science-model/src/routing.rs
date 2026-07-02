//! Routing decision metadata.

use crate::ModelCapabilities;
use serde::{Deserialize, Serialize};

/// Provider and model pair that may be selected by routing.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelRoute {
    /// Selected provider identifier.
    pub provider: String,
    /// Selected model identifier.
    pub model: String,
}

/// Explanation of why a model was selected for a request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Selected provider identifier.
    pub selected_provider: String,
    /// Selected model identifier.
    pub selected_model: String,
    /// Short audit reason for the selection.
    pub reason: String,
    /// Ordered fallback routes considered acceptable by the router.
    pub fallback_routes: Vec<ModelRoute>,
    /// Capabilities matched by the route, when routing already computed them.
    pub matched_capabilities: Option<ModelCapabilities>,
}

impl RoutingDecision {
    /// Creates a routing decision with an audit reason.
    pub fn new(
        provider: impl Into<String>,
        model: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            selected_provider: provider.into(),
            selected_model: model.into(),
            reason: reason.into(),
            fallback_routes: Vec::new(),
            matched_capabilities: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RoutingDecision;

    #[test]
    fn routing_decision_preserves_selected_provider_model_and_reason() {
        let decision = RoutingDecision::new(
            "deepseek",
            "deepseek-reasoner",
            "matched cached text reasoning",
        );

        assert_eq!(decision.selected_provider, "deepseek");
        assert_eq!(decision.selected_model, "deepseek-reasoner");
        assert_eq!(decision.reason, "matched cached text reasoning");
        assert!(decision.fallback_routes.is_empty());
    }
}
