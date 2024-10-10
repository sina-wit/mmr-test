use alloy_primitives::{Keccak256, B256};
use rand::Rng;

/// Hashes two B256 values to a single B256 value using Keccak256.
///
/// # Arguments
///
/// * `left` - The left B256 value to be hashed.
/// * `right` - The right B256 value to be hashed.
///
/// # Returns
///
/// A B256 value that is the hash of the two input values.
///
/// # Examples
///
/// ```
/// use alloy_primitives::B256;
/// use rust_mmr::utils::hash::hash_to_parent;
///
/// let left = B256::repeat_byte(0x11);
/// let right = B256::repeat_byte(0x22);
/// let parent = hash_to_parent(&left, &right);
/// assert_ne!(parent, left);
/// assert_ne!(parent, right);
/// ```
pub fn hash_to_parent(left: &B256, right: &B256) -> B256 {
    let mut hasher = Keccak256::new();
    hasher.update(left.as_slice());
    hasher.update(right.as_slice());
    hasher.finalize()
}

/// Generates a random B256 value. Mostly used for testing purposes.
///
/// # Returns
///
/// A B256 value that represents a random value.
///
/// # Examples
///
/// ```
/// use alloy_primitives::B256;
/// use rust_mmr::utils::hash::get_random_hash;
///
/// let hash = get_random_hash();
/// assert_ne!(hash, B256::ZERO);
/// ```
pub fn get_random_hash() -> B256 {
    rand::thread_rng().gen::<[u8; 32]>().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::b256;

    #[test]
    fn test_hash_to_parent() {
        let left = B256::repeat_byte(0x11);
        let right = B256::repeat_byte(0x22);
        let parent = hash_to_parent(&left, &right);

        // Sanity check that the parent is not the same as the left or right.
        assert_ne!(parent, left);
        assert_ne!(parent, right);

        // Check that the parent is the same as the expected value.
        assert_eq!(
            parent,
            b256!("3e92e0db88d6afea9edc4eedf62fffa4d92bcdfc310dccbe943747fe8302e871")
        );
    }

    #[test]
    fn test_get_random_hash() {
        let hash1 = get_random_hash();
        let hash2 = get_random_hash();

        // Check that two consecutive calls produce different hashes.
        assert_ne!(hash1, hash2);
    }
}
