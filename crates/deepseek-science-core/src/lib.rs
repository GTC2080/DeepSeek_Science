#![forbid(unsafe_code)]
//! Domain-neutral kernel types for the headless Science Agent runtime.
//!
//! This crate owns the language of projects, threads, runs, steps, artifacts,
//! and events. It intentionally has no model provider, UI, or science-domain
//! dependency so future domains can plug into the same replayable kernel.

pub mod error;
pub mod events;
pub mod ids;
pub mod project;
pub mod run;
pub mod thread;

pub use error::CoreError;
pub use events::{CoreEvent, CoreEventEnvelope, EventSequence};
pub use ids::{ArtifactId, ProjectId, RunId, StepId, ThreadId};
pub use project::Project;
pub use run::{AgentRun, RunState, RunStep};
pub use thread::Thread;
