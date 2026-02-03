/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1.chars().nth(i - 1) == s2.chars().nth(j - 1) {
                0
            } else {
                1
            };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(matrix[i - 1][j] + 1, matrix[i][j - 1] + 1),
                matrix[i - 1][j - 1] + cost,
            );
        }
    }

    matrix[len1][len2]
}

/// Find the most similar ID from a list of candidates
pub fn find_similar_id(target: &str, candidates: &[String]) -> Option<String> {
    if candidates.is_empty() {
        return None;
    }

    let mut best_match: Option<(String, usize)> = None;

    for candidate in candidates {
        let distance = levenshtein_distance(target, candidate);

        // Only suggest if distance is small (1-2 character difference)
        if distance <= 2 {
            if let Some((_, best_distance)) = &best_match {
                if distance < *best_distance {
                    best_match = Some((candidate.clone(), distance));
                }
            } else {
                best_match = Some((candidate.clone(), distance));
            }
        }
    }

    best_match.map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
        assert_eq!(levenshtein_distance("hello", "hallo"), 1);
        assert_eq!(levenshtein_distance("hello", "hall"), 2);
        assert_eq!(levenshtein_distance("t8zwaRO1", "t8zwaROl"), 1);
    }

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
