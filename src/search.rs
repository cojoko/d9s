pub fn fuzzy_match(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }

    let haystack = haystack.to_lowercase();
    let needle = needle.to_lowercase();

    let mut haystack_chars = haystack.chars().peekable();

    for needle_char in needle.chars() {
        // Find the next matching character in the haystack
        let found = loop {
            match haystack_chars.peek() {
                Some(&c) if c == needle_char => {
                    haystack_chars.next(); // Consume the character
                    break true;
                }
                Some(_) => {
                    haystack_chars.next(); // Skip this character
                }
                None => break false, // Reached the end without finding a match
            }
        };

        if !found {
            return false; // Couldn't find this character in the remaining haystack
        }
    }

    true // All characters were found in order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match() {
        // Exact matches
        assert!(fuzzy_match("abc", "abc"));
        assert!(fuzzy_match("ABC", "abc"));

        // Fuzzy matches (characters in order but not consecutive)
        assert!(fuzzy_match("abcdef", "ace"));
        assert!(fuzzy_match("Rust Programming", "rp"));

        // Non-matches (characters out of order or missing)
        assert!(!fuzzy_match("abc", "ca"));
        assert!(!fuzzy_match("hello", "world"));

        // Empty cases
        assert!(fuzzy_match("abc", ""));
        assert!(!fuzzy_match("", "abc"));
    }
}
