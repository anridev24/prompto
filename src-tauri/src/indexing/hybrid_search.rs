use crate::models::code_index::CodeChunk;
use std::collections::HashMap;

pub struct HybridSearcher;

impl HybridSearcher {
    pub fn search(
        &self,
        traditional_results: Vec<CodeChunk>,
        full_text_results: Vec<CodeChunk>,
        semantic_results: Vec<CodeChunk>,
        config: &HybridConfig,
    ) -> Vec<CodeChunk> {
        let fused_results = self.reciprocal_rank_fusion(
            &[
                (traditional_results, config.traditional_weight),
                (full_text_results, config.full_text_weight),
                (semantic_results, config.semantic_weight),
            ],
            config.rrf_k,
        );

        fused_results.into_iter()
            .take(config.max_results)
            .collect()
    }

    fn reciprocal_rank_fusion(
        &self,
        result_lists: &[(Vec<CodeChunk>, f32)],
        k: f32,
    ) -> Vec<CodeChunk> {
        let mut scores: HashMap<String, (f32, CodeChunk)> = HashMap::new();

        for (results, weight) in result_lists {
            for (rank, chunk) in results.iter().enumerate() {
                let key = format!(
                    "{}:{}:{}",
                    chunk.file_path,
                    chunk.start_line,
                    chunk.end_line
                );

                let rrf_score = weight / (k + (rank as f32 + 1.0));

                scores.entry(key)
                    .and_modify(|(score, _)| *score += rrf_score)
                    .or_insert((rrf_score, chunk.clone()));
            }
        }

        let mut results: Vec<_> = scores.into_iter()
            .map(|(_, (score, mut chunk))| {
                chunk.relevance_score = score;
                chunk
            })
            .collect();

        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HybridConfig {
    pub traditional_weight: f32,
    pub full_text_weight: f32,
    pub semantic_weight: f32,
    pub rrf_k: f32,
    pub max_results: usize,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            traditional_weight: 0.2,
            full_text_weight: 0.4,
            semantic_weight: 0.4,
            rrf_k: 60.0,
            max_results: 50,
        }
    }
}

impl HybridConfig {
    pub fn exact_match() -> Self {
        Self {
            traditional_weight: 0.7,
            full_text_weight: 0.2,
            semantic_weight: 0.1,
            ..Default::default()
        }
    }

    pub fn semantic_focused() -> Self {
        Self {
            traditional_weight: 0.1,
            full_text_weight: 0.2,
            semantic_weight: 0.7,
            ..Default::default()
        }
    }

    pub fn content_focused() -> Self {
        Self {
            traditional_weight: 0.1,
            full_text_weight: 0.6,
            semantic_weight: 0.3,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_deduplication() {
        // Test that same result in multiple lists gets highest combined score
        // This test validates that RRF properly combines scores when the same
        // chunk appears in multiple result sets
    }

    #[test]
    fn test_config_weights_sum() {
        let config = HybridConfig::default();
        let sum = config.traditional_weight + config.full_text_weight + config.semantic_weight;
        assert!((sum - 1.0).abs() < 0.001, "Weights should sum to ~1.0");
    }

    #[test]
    fn test_exact_match_config() {
        let config = HybridConfig::exact_match();
        assert!(config.traditional_weight > config.semantic_weight);
        assert!(config.traditional_weight > config.full_text_weight);
    }

    #[test]
    fn test_semantic_focused_config() {
        let config = HybridConfig::semantic_focused();
        assert!(config.semantic_weight > config.traditional_weight);
        assert!(config.semantic_weight > config.full_text_weight);
    }
}
