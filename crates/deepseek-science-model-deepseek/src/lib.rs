#![forbid(unsafe_code)]
//! Placeholder DeepSeek model descriptors.
//!
//! This crate contains no network client and does not read API keys. It only
//! gives the future provider adapter a typed place to describe models and mock
//! pricing behavior.

pub mod descriptor;
pub mod pricing;

pub use descriptor::DeepSeekModel;
pub use pricing::DeepSeekPricing;
