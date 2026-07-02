//! Mock pricing helpers for DeepSeek usage accounting.

use deepseek_science_model::NormalizedUsage;
use serde::{Deserialize, Serialize};

/// Placeholder pricing table used only for deterministic local cost estimates.
///
/// Values are not authoritative provider prices. Real billing integration must
/// fetch or configure current pricing in a future provider adapter.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeepSeekPricing {
    /// Price per one million cache-hit input tokens.
    pub cache_hit_input_per_million_usd: f64,
    /// Price per one million cache-miss input tokens.
    pub cache_miss_input_per_million_usd: f64,
    /// Price per one million output tokens.
    pub output_per_million_usd: f64,
}

impl DeepSeekPricing {
    /// Returns a mock pricing table for local tests and examples.
    pub fn mock() -> Self {
        Self {
            cache_hit_input_per_million_usd: 0.01,
            cache_miss_input_per_million_usd: 0.10,
            output_per_million_usd: 0.20,
        }
    }

    /// Estimates request cost from normalized usage.
    pub fn estimate_cost_usd(&self, usage: &NormalizedUsage) -> f64 {
        let hit_cost =
            usage.cache_hit_tokens as f64 * self.cache_hit_input_per_million_usd / 1_000_000.0;
        let miss_cost =
            usage.cache_miss_tokens as f64 * self.cache_miss_input_per_million_usd / 1_000_000.0;
        let output_cost = usage.output_tokens as f64 * self.output_per_million_usd / 1_000_000.0;

        hit_cost + miss_cost + output_cost
    }
}

#[cfg(test)]
mod tests {
    use super::DeepSeekPricing;
    use deepseek_science_model::NormalizedUsage;

    #[test]
    fn estimated_cost_uses_cache_hit_and_miss_tokens() {
        let usage = NormalizedUsage::new(30, 10, 20, 10, 0.0);
        let pricing = DeepSeekPricing::mock();

        let cost = pricing.estimate_cost_usd(&usage);

        assert!((cost - 0.0000032).abs() < f64::EPSILON);
    }
}
