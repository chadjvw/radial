use strsim::levenshtein;

/// Find the most similar ID from a list of candidates
pub fn find_similar_id(target: &str, candidates: &[String]) -> Option<String> {
    candidates
        .iter()
        .map(|candidate| (candidate.clone(), levenshtein(target, candidate)))
        .filter(|(_, distance)| *distance <= 2)
        .min_by_key(|(_, distance)| *distance)
        .map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_similar_id() {
        let candidates = vec![
            "t8zwaROl".to_string(),
            "xYz9Kp2m".to_string(),
            "V1StGXR8".to_string(),
        ];

        assert_eq!(
            find_similar_id("t8zwaRO1", &candidates),
            Some("t8zwaROl".to_string())
        );

        assert_eq!(
            find_similar_id("xYz9Kp2n", &candidates),
            Some("xYz9Kp2m".to_string())
        );

        // Very different ID should return None
        assert_eq!(find_similar_id("zzzzz", &candidates), None);
    }
}
