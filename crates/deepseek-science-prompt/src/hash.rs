//! Stable hashing utilities for compiled prompt prefixes.

/// Returns a lowercase BLAKE3 hash for stable prompt-prefix content.
pub fn hash_stable_prefix(prefix: &str) -> String {
    blake3::hash(prefix.as_bytes()).to_hex().to_string()
}
