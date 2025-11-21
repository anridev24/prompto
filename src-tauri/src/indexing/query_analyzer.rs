use crate::indexing::hybrid_search::HybridConfig;

pub struct QueryAnalyzer;

#[derive(Debug, PartialEq)]
pub enum QueryType {
    ExactSymbol,
    FilePath,
    SemanticIntent,
    CodeContent,
    Mixed,
}

impl QueryAnalyzer {
    pub fn analyze_query(query: &str) -> QueryType {
        let lower = query.to_lowercase();
        let words: Vec<&str> = query.split_whitespace().collect();

        // File path patterns
        if query.contains('/') || query.contains('\\')
            || query.ends_with(".rs") || query.ends_with(".ts") || query.ends_with(".js")
            || query.ends_with(".py") || query.ends_with(".java") || query.ends_with(".go")
        {
            return QueryType::FilePath;
        }

        // Semantic patterns
        if lower.starts_with("how") || lower.starts_with("what")
            || lower.starts_with("why") || lower.contains("how to")
            || lower.starts_with("where") || lower.starts_with("when")
        {
            return QueryType::SemanticIntent;
        }

        // Code patterns
        if query.contains("fn ") || query.contains("async ")
            || query.contains("class ") || query.contains("impl ")
            || query.contains("struct ") || query.contains("trait ")
            || query.contains("interface ") || query.contains("function ")
        {
            return QueryType::CodeContent;
        }

        // Single word likely symbol
        if words.len() == 1 {
            return QueryType::ExactSymbol;
        }

        QueryType::Mixed
    }

    pub fn get_config_for_query(query_type: &QueryType) -> HybridConfig {
        match query_type {
            QueryType::ExactSymbol => HybridConfig::exact_match(),
            QueryType::FilePath => HybridConfig {
                traditional_weight: 0.8,
                full_text_weight: 0.2,
                semantic_weight: 0.0,
                ..Default::default()
            },
            QueryType::SemanticIntent => HybridConfig::semantic_focused(),
            QueryType::CodeContent => HybridConfig::content_focused(),
            QueryType::Mixed => HybridConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_type_detection() {
        assert_eq!(
            QueryAnalyzer::analyze_query("AuthenticationService"),
            QueryType::ExactSymbol
        );

        assert_eq!(
            QueryAnalyzer::analyze_query("how to authenticate"),
            QueryType::SemanticIntent
        );

        assert_eq!(
            QueryAnalyzer::analyze_query("what does indexing do"),
            QueryType::SemanticIntent
        );

        assert_eq!(
            QueryAnalyzer::analyze_query("src/indexing/mod.rs"),
            QueryType::FilePath
        );

        assert_eq!(
            QueryAnalyzer::analyze_query("fn index_codebase"),
            QueryType::CodeContent
        );

        assert_eq!(
            QueryAnalyzer::analyze_query("search results ranking"),
            QueryType::Mixed
        );
    }

    #[test]
    fn test_semantic_patterns() {
        let semantic_queries = vec![
            "how does authentication work",
            "what is the indexing process",
            "why use hybrid search",
            "where is the config stored",
        ];

        for query in semantic_queries {
            assert_eq!(
                QueryAnalyzer::analyze_query(query),
                QueryType::SemanticIntent,
                "Failed for query: {}",
                query
            );
        }
    }

    #[test]
    fn test_file_path_patterns() {
        let file_queries = vec![
            "indexer.rs",
            "src/main.rs",
            "components\\Header.tsx",
            "config.json",
        ];

        for query in file_queries {
            assert_eq!(
                QueryAnalyzer::analyze_query(query),
                QueryType::FilePath,
                "Failed for query: {}",
                query
            );
        }
    }

    #[test]
    fn test_config_selection() {
        let config = QueryAnalyzer::get_config_for_query(&QueryType::ExactSymbol);
        assert!(config.traditional_weight > 0.5);

        let config = QueryAnalyzer::get_config_for_query(&QueryType::SemanticIntent);
        assert!(config.semantic_weight > 0.5);

        let config = QueryAnalyzer::get_config_for_query(&QueryType::FilePath);
        assert!(config.traditional_weight > 0.5);
        assert_eq!(config.semantic_weight, 0.0);
    }
}
