use rust_stemmers::{Algorithm, Stemmer};
use unicode_segmentation::UnicodeSegmentation;
use std::collections::HashSet;

pub struct TextNormalizer {
    stemmer: Stemmer,
    stop_words: HashSet<String>,
}

impl TextNormalizer {
    pub fn new() -> Self {
        Self {
            stemmer: Stemmer::create(Algorithm::English),
            stop_words: Self::create_stop_words(),
        }
    }

    fn create_stop_words() -> HashSet<String> {
        [
            "the", "a", "an", "and", "or", "but", "in", "on", "at",
            "to", "for", "of", "with", "by", "from", "as", "is", "was",
            "get", "set", "new", "old", "tmp", "temp", "var", "fn", "func",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// Normalize text for searching (stem + stop word removal)
    pub fn normalize(&self, text: &str) -> Vec<String> {
        text.unicode_words()
            .map(|w| w.to_lowercase())
            .filter(|w| !self.stop_words.contains(w))
            .filter(|w| w.len() > 2)
            .map(|w| self.stemmer.stem(&w).to_string())
            .collect()
    }

    /// Normalize symbol name (handle camelCase/snake_case)
    pub fn normalize_symbol(&self, name: &str) -> Vec<String> {
        let mut tokens = Vec::new();

        for part in name.split('_') {
            tokens.extend(self.split_camel_case(part));
        }

        tokens.into_iter()
            .map(|t| t.to_lowercase())
            .filter(|t| t.len() > 1)
            .map(|t| self.stemmer.stem(&t).to_string())
            .collect()
    }

    fn split_camel_case(&self, s: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut last_was_upper = false;

        for ch in s.chars() {
            if ch.is_uppercase() {
                if !current.is_empty() && !last_was_upper {
                    result.push(current.clone());
                    current.clear();
                }
                current.push(ch);
                last_was_upper = true;
            } else {
                current.push(ch);
                last_was_upper = false;
            }
        }

        if !current.is_empty() {
            result.push(current);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_symbol_camel_case() {
        let normalizer = TextNormalizer::new();
        let result = normalizer.normalize_symbol("getUserAuthentication");
        assert!(result.contains(&"user".to_string()));
        assert!(result.contains(&"authent".to_string()));
    }

    #[test]
    fn test_normalize_symbol_snake_case() {
        let normalizer = TextNormalizer::new();
        let result = normalizer.normalize_symbol("user_authentication_handler");
        assert!(result.contains(&"user".to_string()));
        assert!(result.contains(&"authent".to_string()));
        assert!(result.contains(&"handler".to_string()));
    }

    #[test]
    fn test_indexing_stems_to_index() {
        let normalizer = TextNormalizer::new();
        let result = normalizer.normalize("indexing");
        assert_eq!(result, vec!["index".to_string()]);
    }
}
