use crate::error::MMRError;
use crate::utils::{
    hash::hash_to_parent,
    range::{decompose, get_expected_num_peaks},
};
use alloy_primitives::B256;

/// Implementation of a stateless Merkle Mountain Range (MMR)
#[derive(Debug)]
pub struct MMR {
    start: u64,
    end: u64,
    peaks: Vec<B256>,
}

impl PartialEq for MMR {
    fn eq(&self, other: &Self) -> bool {
        self.start() == other.start() && self.end() == other.end() && self.peaks() == other.peaks()
    }
}

impl Default for MMR {
    fn default() -> Self {
        Self::new()
    }
}

impl MMR {
    /// Creates a new empty MMR
    pub fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            peaks: vec![],
        }
    }

    pub fn from_leaves(leaves: &Vec<B256>) -> Self {
        let mut mmr = Self::new();
        // TODO(sina) update with a better implementation
        // Can merklize each "perfect" subtree in parallel
        // Each subtree's merklization can be further parallelized
        for leaf in leaves {
            mmr.append(*leaf);
        }
        mmr
    }

    /// Creates a new MMR from the given parameters, validating the input
    pub fn from_params(start: u64, end: u64, peaks: Vec<B256>) -> Result<Self, MMRError> {
        if start > end {
            return Err(MMRError::StartGreaterThanEnd);
        }
        if get_expected_num_peaks(start, end) != peaks.len() as u64 {
            return Err(MMRError::InvalidNumberOfPeaks);
        }

        Ok(Self { start, end, peaks })
    }

    pub fn size(&self) -> u64 {
        self.end - self.start
    }

    pub fn get_root(&self) -> B256 {
        if self.peaks.is_empty() {
            return B256::ZERO;
        }

        let (left, _) = decompose(self.start, self.end);

        // Bag the peaks for the left side
        let left_root = self.peaks[..left.count_ones() as usize]
            .iter()
            .fold(None, |acc, &peak| match acc {
                None => Some(peak),
                Some(prev) => Some(hash_to_parent(&prev, &peak)),
            })
            .unwrap_or(B256::ZERO);

        // Bag the peaks for the right side
        let right_root = self.peaks[left.count_ones() as usize..]
            .iter()
            .rfold(None, |acc, &peak| match acc {
                None => Some(peak),
                Some(prev) => Some(hash_to_parent(&peak, &prev)),
            })
            .unwrap_or(B256::ZERO);

        // Combine the left and right roots
        if left_root == B256::ZERO {
            right_root
        } else if right_root == B256::ZERO {
            left_root
        } else {
            hash_to_parent(&left_root, &right_root)
        }
    }

    pub fn append(&mut self, element: B256) {
        // Leaf is being inserted at index `self.end`.
        // Knowing this, we can follow its merge path from the leaf along the range for as long as it left-merges.
        // Once we encounter a right-merge, we know to stop, and insert the current node as a peak.

        // First, we calculate where the first right-merge will happen, via finding the least-significant unset bit in the new leaf's merge path.
        // We use the right component of the decomposed representation of the tree
        // to account for any offset that may be caused by a non-zero start.
        let (_, right) = decompose(self.start, self.end);
        let least_significant_unset_bit_idx = (!right).trailing_zeros() as usize;

        // Calculate the number of peaks to keep
        let peaks_to_keep = self
            .peaks
            .len()
            .saturating_sub(least_significant_unset_bit_idx);

        // Fold the new element into the peaks that need to be merged
        let new_peak = self.peaks[peaks_to_keep..]
            .iter()
            .rfold(element, |acc, &peak| hash_to_parent(&peak, &acc));

        // Truncate the peaks array to keep only the unmerged peaks
        self.peaks.truncate(peaks_to_keep);
        // Add the new peak
        self.peaks.push(new_peak);
        self.end += 1;
    }

    /// Returns the start index of the MMR
    pub fn start(&self) -> u64 {
        self.start
    }

    /// Returns the end index of the MMR
    pub fn end(&self) -> u64 {
        self.end
    }

    /// Returns a reference to the peaks of the MMR
    pub fn peaks(&self) -> &[B256] {
        &self.peaks
    }

    pub fn merge(&self, other: &MMR) -> Result<Self, MMRError> {
        // Ensure the MMRs are bordering.
        if self.end != other.start {
            return Err(MMRError::MergeError);
        }
        // Currently only works for 0-starting MMRs.
        if self.start != 0 {
            return Err(MMRError::MergeError);
        }
        // Start with the rightmost peak of the left MMR as the seed.
        let mut seed = *self.peaks.last().unwrap();
        // Seed height is equal to the index of the lsb of end.
        let mut seed_height = self.end.trailing_zeros();
        let mut seed_index = (self.end - 1) >> seed_height;
        let seed_range_start = seed_index * (1 << seed_height);
        // Zip seed up with left and right along its merge path.
        let mut left_cursor = self.peaks.len() - 1;
        let mut right_cursor = 0;
        while seed_height < 255 {
            let layer_coverage = 1 << seed_height;
            if seed_index & 1 == 0 {
                // Right merge, or break if not possible.
                let merged_range_end = seed_range_start + (layer_coverage << 1);
                if merged_range_end > other.end {
                    break;
                }
                seed = hash_to_parent(&seed, &other.peaks[right_cursor]);
                right_cursor += 1;
            } else {
                // Left merge, or break if not possible.
                if layer_coverage > seed_range_start {
                    break;
                }
                left_cursor -= 1;
                seed = hash_to_parent(&self.peaks[left_cursor], &seed);
            }
            seed_index >>= 1;
            seed_height += 1;
        }

        return Ok(Self {
            start: self.start,
            end: other.end,
            peaks: self.peaks[..left_cursor]
                .iter()
                .chain(std::iter::once(&seed))
                .chain(other.peaks[right_cursor..].iter())
                .cloned()
                .collect(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::hash::get_random_hash;
    use alloy_primitives::{b256, U256};

    #[test]
    fn test_empty_mmr_creation() {
        let mmr = MMR::new();
        assert_eq!(mmr.start, 0);
        assert_eq!(mmr.end, 0);
        assert_eq!(mmr.peaks.len(), 0);
        assert_eq!(mmr.size(), 0);
        // Empty MMR's root returns a zero hash.
        assert_eq!(mmr.get_root(), B256::ZERO);
    }

    #[test]
    fn test_mmr_creation_invalid_params() {
        // Should fail due to start > end
        let mmr = MMR::from_params(1, 0, vec![get_random_hash()]);
        assert!(matches!(mmr.err().unwrap(), MMRError::StartGreaterThanEnd));

        // Should fail due to invalid number of peaks
        let mmr = MMR::from_params(0, 1, vec![get_random_hash(), get_random_hash()]);
        assert!(matches!(mmr.err().unwrap(), MMRError::InvalidNumberOfPeaks));
    }

    #[test]
    fn test_get_root() {
        let element = get_random_hash();
        let mmr = MMR::from_params(0, 1, vec![element]).unwrap();
        assert_eq!(mmr.get_root(), element);

        let element2 = get_random_hash();
        let mmr = MMR::from_params(0, 3, vec![element, element2]).unwrap();
        assert_eq!(mmr.get_root(), hash_to_parent(&element, &element2));
    }

    #[test]
    fn test_get_root_nonzero_start() {
        let element1 = get_random_hash();
        let element2 = get_random_hash();
        let mmr = MMR::from_params(1, 3, vec![element1, element2]).unwrap();
        assert_eq!(mmr.get_root(), hash_to_parent(&element1, &element2));

        let element3 = get_random_hash();
        let mmr = MMR::from_params(1, 5, vec![element1, element2, element3]).unwrap();
        assert_eq!(
            mmr.get_root(),
            hash_to_parent(&hash_to_parent(&element1, &element2), &element3)
        );
    }

    #[test]
    fn test_append_from_empty() {
        let mut mmr = MMR::new();
        let element = get_random_hash();
        mmr.append(element);
        assert_eq!(mmr, MMR::from_params(0, 1, vec![element]).unwrap());

        let element2 = get_random_hash();
        mmr.append(element2);
        let root_1_0 = hash_to_parent(&element, &element2);
        assert_eq!(mmr, MMR::from_params(0, 2, vec![root_1_0]).unwrap());

        let element3 = get_random_hash();
        mmr.append(element3);
        assert_eq!(
            mmr,
            MMR::from_params(0, 3, vec![root_1_0, element3]).unwrap()
        );

        let element4 = get_random_hash();
        mmr.append(element4);
        let root_1_1 = hash_to_parent(&element3, &element4);
        let root_0_2 = hash_to_parent(&root_1_0, &root_1_1);
        assert_eq!(mmr, MMR::from_params(0, 4, vec![root_0_2]).unwrap());
    }

    #[test]
    fn test_append_nonzero_start() {
        let mut mmr = MMR::from_params(1, 1, vec![]).unwrap();
        let element_1 = get_random_hash();
        mmr.append(element_1);
        assert_eq!(mmr, MMR::from_params(1, 2, vec![element_1]).unwrap());

        let element_2 = get_random_hash();
        mmr.append(element_2);
        assert_eq!(
            mmr,
            MMR::from_params(1, 3, vec![element_1, element_2]).unwrap()
        );

        let element_3 = get_random_hash();
        mmr.append(element_3);
        let node_1_1 = hash_to_parent(&element_2, &element_3);
        assert_eq!(
            mmr,
            MMR::from_params(1, 4, vec![element_1, node_1_1]).unwrap()
        );

        let element_4 = get_random_hash();
        mmr.append(element_4);
        assert_eq!(
            mmr,
            MMR::from_params(1, 5, vec![element_1, node_1_1, element_4]).unwrap()
        );
    }

    #[test]
    fn test_append_large_range() {
        let element = get_random_hash();
        let mut mmr = MMR::from_params(1 << 19, 1 << 20, vec![element]).unwrap();

        let element_2 = get_random_hash();
        mmr.append(element_2);
        assert_eq!(
            mmr,
            MMR::from_params(1 << 19, (1 << 20) + 1, vec![element, element_2]).unwrap()
        );
    }

    #[test]
    fn test_append_near_u64_max() {
        let element = get_random_hash();
        let mut mmr = MMR::from_params(u64::MAX - 2, u64::MAX - 1, vec![element]).unwrap();
        let element_2 = get_random_hash();
        mmr.append(element_2);
        assert_eq!(
            mmr,
            MMR::from_params(u64::MAX - 2, u64::MAX, vec![element, element_2]).unwrap()
        );
        assert_eq!(mmr.get_root(), hash_to_parent(&element, &element_2));
    }

    #[test]
    fn test_append_conformance() {
        let mut mmr = MMR::new();
        let num_leaves = (1 << 10) + 12345;
        for i in 0..num_leaves {
            mmr.append(U256::from(i).into());
        }

        // Matches hard-coded values from plasma-lib conformance test.
        assert_eq!(
            mmr.get_root(),
            b256!("f20ad78c9e954b1ab6f4e3d4d45d5eb2c3092e6d49c284403adc63f1ec4bd94a")
        );
        assert_eq!(
            mmr.peaks(),
            &[
                b256!("9cd2165f9ca0b9f495678716ecef463c15442c5078b35d1afa4feb2730f93af1"),
                b256!("e9c7c8c1f62832a1aeca64cfdf95b47563e048d98fc668c9f7c0da3fa0c349d7"),
                b256!("8d4c7f591cbcc0333a106c16fdcd176c69f506706e81bc7578eeed49fb161f65"),
                b256!("5f5270c99f31d41394adc86ace55db213cb1441baaa3d90d42ce6f59431407de"),
                b256!("9b605c9eccb93ad289b8b91a2691a1417b01a45beadab0f0c3847af1e058533b"),
                b256!("e2d9d763b82d01e7b716f6526e8c85cc860c60fdf3553bb245337a614249e3d7"),
                b256!("0000000000000000000000000000000000000000000000000000000000003438"),
            ]
        );
    }

    #[test]
    fn test_merge_errors() {
        // Non-bordering MMRs error.
        let mmr1 = MMR::from_params(0, 1, vec![get_random_hash()]).unwrap();
        let mmr2 = MMR::from_params(2, 4, vec![get_random_hash()]).unwrap();
        assert!(matches!(mmr1.merge(&mmr2), Err(MMRError::MergeError)));

        // Non-zero start MMRs error.
        let mmr1 = MMR::from_params(1, 2, vec![get_random_hash()]).unwrap();
        let mmr2 = MMR::from_params(2, 4, vec![get_random_hash()]).unwrap();
        assert!(matches!(mmr1.merge(&mmr2), Err(MMRError::MergeError)));
    }

    #[test]
    fn test_merge() {
        let element_1 = get_random_hash();
        let mmr1 = MMR {
            start: 0,
            end: 4,
            peaks: vec![element_1],
        };

        let element_2 = get_random_hash();
        let mmr2 = MMR {
            start: 4,
            end: 8,
            peaks: vec![element_2],
        };

        assert_eq!(
            mmr1.merge(&mmr2).unwrap(),
            MMR::from_params(0, 8, vec![hash_to_parent(&element_1, &element_2)]).unwrap()
        );
    }

    #[test]
    fn test_from_leaves() {
        let leaves = vec![get_random_hash(), get_random_hash(), get_random_hash()];
        let mmr = MMR::from_leaves(&leaves);
        assert_eq!(
            mmr,
            MMR {
                start: 0,
                end: 3,
                peaks: vec![hash_to_parent(&leaves[0], &leaves[1]), leaves[2]],
            }
        );
    }
}
