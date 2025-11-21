use crate::models::code_index::SymbolKind;

pub struct RelevanceScorer;

impl RelevanceScorer {
    pub fn score_symbol_match(
        symbol_name: &str,
        query_term: &str,
        match_type: MatchType,
        total_symbols: usize,
        term_frequency: usize,
    ) -> f32 {
        let base_score = match match_type {
            MatchType::Exact => 1.0,
            MatchType::NormalizedExact => 0.9,
            MatchType::Prefix => 0.7,
            MatchType::Contains => 0.5,
            MatchType::Normalized => 0.6,
        };

        let length_ratio = query_term.len() as f32 / symbol_name.len() as f32;
        let length_bonus = length_ratio * 0.2;

        let idf = (total_symbols as f32 / term_frequency as f32).ln();
        let idf_bonus = (idf / 10.0).min(0.3);

        (base_score + length_bonus + idf_bonus).min(1.0)
    }

    pub fn score_symbol_kind(kind: &SymbolKind) -> f32 {
        match kind {
            SymbolKind::Function => 1.0,
            SymbolKind::Class | SymbolKind::Struct => 0.95,
            SymbolKind::Method => 0.9,
            SymbolKind::Enum | SymbolKind::Interface => 0.85,
            SymbolKind::Constant => 0.7,
            SymbolKind::Variable => 0.6,
            SymbolKind::Import | SymbolKind::Export => 0.4,
        }
    }

    pub fn calculate_final_score(
        symbol_score: f32,
        kind_score: f32,
        has_doc_comment: bool,
    ) -> f32 {
        let doc_bonus = if has_doc_comment { 0.05 } else { 0.0 };
        (symbol_score * 0.7 + kind_score * 0.3 + doc_bonus).min(1.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MatchType {
    Exact,
    NormalizedExact,
    Prefix,
    Contains,
    Normalized,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::code_index::SymbolKind;

    #[test]
    fn test_exact_match_highest() {
        let score = RelevanceScorer::score_symbol_match(
            "authenticate",
            "authenticate",
            MatchType::Exact,
            1000,
            10,
        );
        assert!(score > 0.9);
    }

    #[test]
    fn test_function_scores_higher_than_import() {
        let func_score = RelevanceScorer::score_symbol_kind(&SymbolKind::Function);
        let import_score = RelevanceScorer::score_symbol_kind(&SymbolKind::Import);
        assert!(func_score > import_score);
    }
}
