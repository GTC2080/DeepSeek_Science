//! Hash helpers for artifact bytes.

/// Exact-byte hash algorithm supported by artifact descriptors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactHashAlgorithm {
    /// BLAKE3 over the byte sequence exactly as supplied.
    Blake3,
}

impl ExactHashAlgorithm {
    /// Returns the stable lowercase algorithm label used in envelope JSON.
    pub fn machine_label(self) -> &'static str {
        match self {
            Self::Blake3 => "blake3",
        }
    }
}

/// BLAKE3 verifier for one exact byte sequence.
///
/// This value verifies content bytes. It is neither a semantic hash nor a
/// registered artifact instance identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactByteHash {
    algorithm: ExactHashAlgorithm,
    value: String,
}

impl ExactByteHash {
    /// Hashes the supplied bytes exactly once with BLAKE3.
    pub fn blake3(bytes: &[u8]) -> Self {
        Self {
            algorithm: ExactHashAlgorithm::Blake3,
            value: hash_bytes(bytes),
        }
    }

    /// Returns the typed hash algorithm.
    pub fn algorithm(&self) -> ExactHashAlgorithm {
        self.algorithm
    }

    /// Returns the 64-character lowercase hexadecimal hash value.
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Returns a lowercase BLAKE3 hash for artifact bytes.
pub fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::{hash_bytes, ExactByteHash, ExactHashAlgorithm};

    #[test]
    fn same_bytes_produce_same_hash() {
        let first = hash_bytes(b"artifact");
        let second = hash_bytes(b"artifact");

        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(first
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
    }

    #[test]
    fn different_bytes_produce_different_hashes() {
        let first = hash_bytes(b"artifact-a");
        let second = hash_bytes(b"artifact-b");

        assert_ne!(first, second);
    }

    #[test]
    fn exact_byte_hash_reuses_existing_blake3_hash() {
        let descriptor = ExactByteHash::blake3(b"artifact");

        assert_eq!(descriptor.algorithm(), ExactHashAlgorithm::Blake3);
        assert_eq!(descriptor.algorithm().machine_label(), "blake3");
        assert_eq!(descriptor.value(), hash_bytes(b"artifact"));
    }

    #[test]
    fn exact_byte_hash_is_deterministic_and_content_sensitive() {
        let first = ExactByteHash::blake3(b"artifact-a");
        let repeated = ExactByteHash::blake3(b"artifact-a");
        let different = ExactByteHash::blake3(b"artifact-b");

        assert_eq!(first, repeated);
        assert_ne!(first, different);
        assert_eq!(first.value().len(), 64);
        assert!(first
            .value()
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
    }
}
