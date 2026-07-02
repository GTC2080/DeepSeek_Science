//! Normalized token and cost accounting.

use serde::{Deserialize, Serialize};

/// Provider-neutral accounting for model usage.
///
/// Cache hit and miss tokens are tracked separately because long-context
/// scientific workflows should be able to audit prompt-prefix reuse.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedUsage {
    /// Prompt tokens reported by the provider or local estimator.
    pub input_tokens: u64,
    /// Completion tokens reported by the provider or local estimator.
    pub output_tokens: u64,
    /// Input tokens served from cache.
    pub cache_hit_tokens: u64,
    /// Input tokens billed or processed as cache misses.
    pub cache_miss_tokens: u64,
    /// Total input plus output tokens.
    pub total_tokens: u64,
    /// Estimated request cost in US dollars.
    pub estimated_cost_usd: f64,
}

impl NormalizedUsage {
    /// Creates usage and derives the total token count.
    pub fn new(
        input_tokens: u64,
        output_tokens: u64,
        cache_hit_tokens: u64,
        cache_miss_tokens: u64,
        estimated_cost_usd: f64,
    ) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cache_hit_tokens,
            cache_miss_tokens,
            total_tokens: input_tokens + output_tokens,
            estimated_cost_usd,
        }
    }

    /// Returns the cache hit rate across cache-accounted input tokens.
    pub fn cache_hit_rate(&self) -> f64 {
        let cache_accounted = self.cache_hit_tokens + self.cache_miss_tokens;
        if cache_accounted == 0 {
            return 0.0;
        }

        self.cache_hit_tokens as f64 / cache_accounted as f64
    }
}

#[cfg(test)]
mod tests {
    use super::NormalizedUsage;

    #[test]
    fn usage_cache_hit_rate_calculation() {
        let usage = NormalizedUsage::new(100, 20, 75, 25, 0.01);

        assert_eq!(usage.total_tokens, 120);
        assert!((usage.cache_hit_rate() - 0.75).abs() < f64::EPSILON);
    }
}
