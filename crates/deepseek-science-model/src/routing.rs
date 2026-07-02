//! Routing decision metadata.

use crate::ModelDescriptor;
use serde::{Deserialize, Serialize};

/// Explanation of why a model was selected for a request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Selected model descriptor.
    pub selected: ModelDescriptor,
    /// Short audit reason for the selection.
    pub reason: String,
}

impl RoutingDecision {
    /// Creates a routing decision with an audit reason.
    pub fn new(selected: ModelDescriptor, reason: impl Into<String>) -> Self {
        Self {
            selected,
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RoutingDecision;
    use crate::{ModelCapabilities, ModelDescriptor};

    #[test]
    fn routing_decision_can_be_constructed() {
        let descriptor = ModelDescriptor::new(
            "deepseek",
            "deepseek-reasoner",
            ModelCapabilities::text_only(None),
        );

        let decision = RoutingDecision::new(descriptor, "default reasoning model");

        assert_eq!(decision.selected.provider, "deepseek");
        assert_eq!(decision.reason, "default reasoning model");
    }
}
