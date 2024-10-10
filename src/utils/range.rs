/*
Reference MMR with 8 leaves:

Level 3 |                               [0]
        |                              /   \
        |                             /     \
        |                            /       \
        |                           /         \
        |                          /           \
        |                         /             \
Level 2 |                    [0]                 [1]
        |                   /   \               /   \
        |                  /     \             /     \
        |                 /       \           /       \
        |                /         \         /         \
        |               /           \       /           \
Level 1 |          [0]               [1]   [2]           [3]
        |         /   \             /   \ /   \         /   \
        |        /     \           /     X     \     /       \
        |       /       \         /     / \     \     /       \
Level 0 |    [0]         [1]    [2]    [3] [4]   [5] [6]       [7]

Node indices per level:
Level 0 (Leaves): 0-7 (8 nodes)
Level 1: 0-3 (4 nodes)
Level 2: 0-1 (2 nodes)
Level 3 (Root): 0 (1 node)

Total nodes: 15

Nodes are referenced as a (level, index) tuple.
*/

/// Decomposes a non-zero-starting interval into two parts that represent
/// the compact range needed to express the interval.
///
/// # Arguments
///
/// * `begin` - The start of the interval (inclusive)
/// * `end` - The end of the interval (exclusive)
///
/// # Returns
///
/// A tuple `(left, right)` where:
///
/// * `left` - Bitmap representing the left part of the interval
/// * `right` - Bitmap representing the right part of the interval
///
/// # Examples
///
/// ```
/// use rust_mmr::utils::range::decompose;
///
/// let (left, right) = decompose(3, 7);
/// assert_eq!(left, 1);
/// assert_eq!(right, 3);
/// ```
pub fn decompose(begin: u64, end: u64) -> (u64, u64) {
    if begin == 0 {
        return (0, end);
    }
    // The index before 'begin' represents the last node in the complementary "zero-index-starting" interval
    let x_begin = begin - 1;
    // Find the highest bit where x_begin and end differ, which indicates the difference between the left merge path
    // (which represents a tree of maximum size `end`) and the right merge path (which can merge into a much larger tree)
    let diverge = (x_begin ^ end).ilog2();
    // Create a mask with 'diverge' number of 1s
    let mask = (1 << diverge) - 1;
    // Left part: nodes that will be merged into the complementary interval, capped by mask
    // Right part: right-merges of 'end', capped by mask
    (!x_begin & mask, end & mask)
}

/// Calculates the expected number of peaks for a range given its begin and end leaf indices.
///
/// # Arguments
///
/// * `begin` - The start of the interval (inclusive)
/// * `end` - The end of the interval (exclusive)
///
/// # Returns
///
/// The number of peaks expected for the given range.
///
/// # Examples
///
/// ```
/// use rust_mmr::utils::range::get_expected_num_peaks;
///
/// let range_start = 3;
/// let range_end = 7;
/// let num_peaks = get_expected_num_peaks(range_start, range_end);
/// assert_eq!(num_peaks, 3);
/// ```
pub fn get_expected_num_peaks(begin: u64, end: u64) -> u64 {
    let (left, right) = decompose(begin, end);
    (left.count_ones() + right.count_ones()) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompose_zero_start() {
        let (left, right) = decompose(0, 5);
        assert_eq!(left, 0);
        assert_eq!(right, 5);
    }

    #[test]
    fn test_decompose_non_zero_start_0() {
        let (left, right) = decompose(1, 4);
        assert_eq!(left, 3);
        assert_eq!(right, 0);
    }

    #[test]
    fn test_decompose_non_zero_start_1() {
        let (left, right) = decompose(15, 17);
        assert_eq!(left, 1);
        assert_eq!(right, 1);
    }

    #[test]
    fn test_decompose_non_zero_start_2() {
        let (left, right) = decompose(3, 7);
        assert_eq!(left, 1);
        assert_eq!(right, 3);
    }

    #[test]
    fn test_decompose_adjacent_numbers() {
        let (left, right) = decompose(7, 8);
        assert_eq!(left, 1);
        assert_eq!(right, 0);
    }

    #[test]
    fn test_decompose_power_of_two_interval() {
        let (left, right) = decompose(8, 16);
        assert_eq!(left, 8);
        assert_eq!(right, 0);
    }

    #[test]
    fn test_decompose_power_of_two_interval_2() {
        let (left, right) = decompose(8, 32);
        assert_eq!(left, 24);
        assert_eq!(right, 0);
    }

    #[test]
    fn test_decompose_large_interval() {
        let (left, right) = decompose(1000, 2000);
        assert_eq!(left, 24);
        assert_eq!(right, 976);
    }

    #[test]
    fn test_decompose_max_u64_interval() {
        let (left, right) = decompose(u64::MAX - 1, u64::MAX);
        assert_eq!(left, 0);
        assert_eq!(right, 1);
    }

    #[test]
    fn test_decompose_many_cases() {
        // Cases referenced from https://github.com/transparency-dev/merkle/blob/main/compact/range_test.go#L497
        assert_eq!(decompose(0, 0), (0, 0)); // subtree sizes [],[]
        assert_eq!(decompose(0, 2), (0, 2)); // subtree sizes [], [2]
        assert_eq!(decompose(0, 4), (0, 4)); // subtree sizes [], [4]
        assert_eq!(decompose(1, 3), (1, 1)); // subtree sizes [1], [1]
        assert_eq!(decompose(3, 7), (1, 3)); // subtree sizes [1], [2, 1]
        assert_eq!(decompose(3, 17), (13, 1)); // subtree sizes [1, 4, 8], [1]
        assert_eq!(decompose(4, 28), (12, 12)); // subtree sizes [4, 8], [8, 4]
        assert_eq!(decompose(8, 24), (8, 8)); // subtree sizes [8], [8]
        assert_eq!(decompose(8, 28), (8, 12)); // subtree sizes [8], [8, 4]
        assert_eq!(decompose(11, 25), (5, 9)); // subtree sizes [1, 4], [8, 1]
        assert_eq!(decompose(31, 45), (1, 13)); // subtree sizes [1], [8, 4, 1]
    }

    #[test]
    fn test_get_expected_num_peaks() {
        assert_eq!(get_expected_num_peaks(0, 8), 1);
        assert_eq!(get_expected_num_peaks(0, 9), 2);
        assert_eq!(get_expected_num_peaks(0, 10), 2);
        assert_eq!(get_expected_num_peaks(0, 11), 3);
        assert_eq!(get_expected_num_peaks(0, 12), 2);
        assert_eq!(get_expected_num_peaks(0, 13), 3);

        assert_eq!(get_expected_num_peaks(2, 7), 3);
        assert_eq!(get_expected_num_peaks(3, 7), 3);
        assert_eq!(get_expected_num_peaks(3, 8), 2);
        assert_eq!(get_expected_num_peaks(1, 4), 2);
        assert_eq!(get_expected_num_peaks(15, 17), 2);
        assert_eq!(get_expected_num_peaks(8, 16), 1);
        assert_eq!(get_expected_num_peaks(1000, 2000), 7);
    }

    #[test]
    fn test_get_expected_num_peaks_edge_cases() {
        assert_eq!(get_expected_num_peaks(0, 0), 0);
        assert_eq!(get_expected_num_peaks(0, 1), 1);
        assert_eq!(get_expected_num_peaks(1, 1), 0);
        assert_eq!(get_expected_num_peaks(1, 2), 1);
        assert_eq!(get_expected_num_peaks(0, u64::MAX), 64);
        assert_eq!(get_expected_num_peaks(u64::MAX - 1, u64::MAX), 1);
    }

    #[test]
    fn test_get_expected_num_peaks_large_ranges() {
        assert_eq!(get_expected_num_peaks(0, 1 << 20), 1);
        assert_eq!(get_expected_num_peaks(1 << 20, 1 << 21), 1);
        assert_eq!(
            get_expected_num_peaks(1 << 20, (1 << 20) + (1 << 19)) + 1,
            2
        );
    }
}
