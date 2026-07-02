//! Hash helpers for artifact bytes.

/// Returns a lowercase BLAKE3 hash for artifact bytes.
pub fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::hash_bytes;

    #[test]
    fn same_bytes_produce_same_hash() {
        let first = hash_bytes(b"artifact");
        let second = hash_bytes(b"artifact");

        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
    }

    #[test]
    fn different_bytes_produce_different_hashes() {
        let first = hash_bytes(b"artifact-a");
        let second = hash_bytes(b"artifact-b");

        assert_ne!(first, second);
    }
}
