/// Generate a safe 8-character ID
/// Uses alphanumeric characters only (no dashes or underscores)
/// to avoid conflicts with CLI flag parsing
pub fn generate_id() -> String {
    const ALPHABET: [char; 62] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H',
        'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];

    nanoid::nanoid!(8, &ALPHABET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        for _ in 0..100 {
            let id = generate_id();
            assert_eq!(id.len(), 8);
            assert!(!id.starts_with('-'));
            assert!(!id.starts_with('_'));
            assert!(!id.contains('-'));
            assert!(!id.contains('_'));
        }
    }
}
