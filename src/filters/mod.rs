pub mod git;

/// Estimate token count from byte length — O(1), no allocation.
/// Uses the ~4 bytes/token heuristic common for English/code.
pub fn estimate_tokens(byte_count: usize) -> u64 {
    (byte_count as u64).div_ceil(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_tokens_rounds_up() {
        assert_eq!(estimate_tokens(0), 0);
        assert_eq!(estimate_tokens(1), 1);
        assert_eq!(estimate_tokens(4), 1);
        assert_eq!(estimate_tokens(5), 2);
        assert_eq!(estimate_tokens(100), 25);
    }
}
