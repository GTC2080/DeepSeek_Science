#![forbid(unsafe_code)]
//! Prompt prefix compiler for cache-friendly model requests.
//!
//! The compiler separates stable instruction prefixes from variable user tails
//! so future model providers can reuse long scientific context safely.

pub mod compiler;
pub mod error;
pub mod hash;
pub mod sections;

pub use compiler::{compile_prompt, CompiledPrompt, PromptCompileInput, PromptVersionInfo};
pub use error::PromptError;
pub use sections::{PromptSection, PromptSectionKind};
