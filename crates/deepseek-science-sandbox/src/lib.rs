#![forbid(unsafe_code)]
//! Permission and sandbox interface placeholders.
//!
//! This crate does not execute arbitrary commands in Phase 1. It only models
//! the policy boundary that future runners must honor.

pub mod error;
pub mod policy;
pub mod runner;

pub use error::SandboxError;
pub use policy::{ExecutionPermission, SandboxPolicy};
pub use runner::{SandboxRequest, SandboxResult, SandboxRunner};
