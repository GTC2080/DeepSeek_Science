#![forbid(unsafe_code)]
//! Provider-neutral model gateway types.
//!
//! The gateway describes requests, responses, usage accounting, capabilities,
//! and routing decisions without depending on any concrete provider SDK.

pub mod capabilities;
pub mod error;
pub mod provider;
pub mod request;
pub mod response;
pub mod routing;
pub mod usage;

pub use capabilities::{Modality, ModelCapabilities, ModelDescriptor};
pub use error::ModelError;
pub use provider::ModelProvider;
pub use request::{
    CachePolicy, MessagePart, MessageRole, ModelMessage, ModelRequest, PrivacyPolicy,
};
pub use response::ModelResponse;
pub use routing::{ModelRoute, RoutingDecision};
pub use usage::NormalizedUsage;
