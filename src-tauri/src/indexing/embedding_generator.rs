use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::Tokenizer;

use crate::models::code_index::CodeSymbol;

/// Generates semantic embeddings for code using BERT model
pub struct EmbeddingGenerator {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    embedding_dim: usize,
}

impl EmbeddingGenerator {
    /// Creates a new EmbeddingGenerator with all-MiniLM-L6-v2 model
    pub fn new() -> Result<Self, String> {
        println!("Initializing embedding generator...");

        // Use CPU device (GPU support can be added later)
        let device = Device::Cpu;

        // Download model from HuggingFace
        let api = Api::new().map_err(|e| format!("Failed to create HF API: {}", e))?;
        let repo = api.repo(Repo::new(
            "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            RepoType::Model,
        ));

        println!("Downloading model files from HuggingFace...");

        // Download required files
        let config_path = repo
            .get("config.json")
            .map_err(|e| format!("Failed to download config: {}", e))?;
        let tokenizer_path = repo
            .get("tokenizer.json")
            .map_err(|e| format!("Failed to download tokenizer: {}", e))?;
        let weights_path = repo
            .get("model.safetensors")
            .map_err(|e| format!("Failed to download weights: {}", e))?;

        println!("Loading model configuration...");

        // Load config
        let config_content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        let config: BertConfig = serde_json::from_str(&config_content)
            .map_err(|e| format!("Failed to parse config: {}", e))?;

        let embedding_dim = config.hidden_size;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| format!("Failed to load tokenizer: {}", e))?;

        println!("Loading model weights...");

        // Load model weights
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], candle_core::DType::F32, &device)
                .map_err(|e| format!("Failed to load weights: {}", e))?
        };

        let model = BertModel::load(vb, &config)
            .map_err(|e| format!("Failed to create model: {}", e))?;

        println!("Embedding generator ready (dim: {})", embedding_dim);

        Ok(Self {
            model,
            tokenizer,
            device,
            embedding_dim,
        })
    }

    /// Returns the dimensionality of embeddings produced by this generator
    pub fn embedding_dim(&self) -> usize {
        self.embedding_dim
    }

    /// Generate embedding for a single text
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let embeddings = self.embed_batch(&[text.to_string()])?;
        Ok(embeddings.into_iter().next().unwrap())
    }

    /// Generate embeddings for a batch of texts
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Tokenize all texts
        let encodings = self
            .tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| format!("Tokenization failed: {}", e))?;

        let mut input_ids_vec = Vec::new();
        let mut attention_mask_vec = Vec::new();

        for encoding in &encodings {
            input_ids_vec.push(encoding.get_ids().to_vec());
            attention_mask_vec.push(encoding.get_attention_mask().to_vec());
        }

        // Convert to tensors
        let input_ids = self.vec2d_to_tensor(&input_ids_vec)?;
        let attention_mask = self.vec2d_to_tensor(&attention_mask_vec)?;

        // Run model
        let output = self
            .model
            .forward(&input_ids, &attention_mask, None)
            .map_err(|e| format!("Model forward failed: {}", e))?;

        // Mean pooling
        let embeddings = self.mean_pooling(&output, &attention_mask)?;

        // Normalize embeddings
        let normalized = self.normalize_embedding(&embeddings)?;

        // Convert to Vec<Vec<f32>>
        self.tensor_to_vec2d(&normalized)
    }

    /// Convert 2D vector to tensor
    fn vec2d_to_tensor(&self, data: &[Vec<u32>]) -> Result<Tensor, String> {
        let batch_size = data.len();
        let seq_len = data[0].len();

        let flat: Vec<u32> = data.iter().flat_map(|v| v.clone()).collect();

        Tensor::from_vec(flat, (batch_size, seq_len), &self.device)
            .map_err(|e| format!("Failed to create tensor: {}", e))
    }

    /// Mean pooling over sequence dimension
    fn mean_pooling(&self, embeddings: &Tensor, attention_mask: &Tensor) -> Result<Tensor, String> {
        // embeddings: [batch_size, seq_len, hidden_dim]
        // attention_mask: [batch_size, seq_len]

        let attention_mask = attention_mask
            .unsqueeze(2)
            .map_err(|e| format!("Failed to unsqueeze: {}", e))?;

        let attention_mask_f32 = attention_mask
            .to_dtype(candle_core::DType::F32)
            .map_err(|e| format!("Failed to convert dtype: {}", e))?;

        // Multiply embeddings by attention mask
        let masked_embeddings = embeddings
            .broadcast_mul(&attention_mask_f32)
            .map_err(|e| format!("Failed to broadcast_mul: {}", e))?;

        // Sum over sequence dimension
        let sum_embeddings = masked_embeddings
            .sum(1)
            .map_err(|e| format!("Failed to sum: {}", e))?;

        // Sum attention mask to get counts
        let sum_mask = attention_mask_f32
            .sum(1)
            .map_err(|e| format!("Failed to sum mask: {}", e))?;

        // Divide to get mean
        sum_embeddings
            .broadcast_div(&sum_mask)
            .map_err(|e| format!("Failed to broadcast_div: {}", e))
    }

    /// Normalize embeddings to unit length
    fn normalize_embedding(&self, embeddings: &Tensor) -> Result<Tensor, String> {
        // embeddings: [batch_size, hidden_dim]

        let norm = embeddings
            .sqr()
            .map_err(|e| format!("Failed to square: {}", e))?
            .sum_keepdim(1)
            .map_err(|e| format!("Failed to sum: {}", e))?
            .sqrt()
            .map_err(|e| format!("Failed to sqrt: {}", e))?;

        embeddings
            .broadcast_div(&norm)
            .map_err(|e| format!("Failed to normalize: {}", e))
    }

    /// Convert tensor to 2D vector
    fn tensor_to_vec2d(&self, tensor: &Tensor) -> Result<Vec<Vec<f32>>, String> {
        let shape = tensor.dims();
        if shape.len() != 2 {
            return Err(format!("Expected 2D tensor, got {:?}", shape));
        }

        let data = tensor
            .to_vec2::<f32>()
            .map_err(|e| format!("Failed to convert to vec: {}", e))?;

        Ok(data)
    }
}

/// Convert a CodeSymbol to text for embedding
pub fn symbol_to_text(symbol: &CodeSymbol) -> String {
    let mut parts = Vec::new();

    // Add symbol name
    parts.push(symbol.name.clone());

    // Add kind
    parts.push(format!("{:?}", symbol.kind));

    // Add signature if available
    if let Some(ref sig) = symbol.signature {
        parts.push(sig.clone());
    }

    // Add doc comment if available
    if let Some(ref doc) = symbol.doc_comment {
        parts.push(doc.clone());
    }

    parts.join(" ")
}

/// Calculate cosine similarity between two embeddings
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![1.0, 0.0, 0.0];
        let d = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&c, &d) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalization() {
        let vec = vec![3.0, 4.0];
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_symbol_to_text() {
        use crate::models::code_index::{CodeSymbol, SymbolKind};

        let symbol = CodeSymbol {
            name: "authenticate_user".to_string(),
            kind: SymbolKind::Function,
            file_path: "auth.rs".to_string(),
            start_line: 10,
            end_line: 20,
            signature: Some("fn authenticate_user(username: &str, password: &str) -> bool".to_string()),
            doc_comment: Some("Authenticates a user with username and password".to_string()),
            parent: None,
        };

        let text = symbol_to_text(&symbol);
        assert!(text.contains("authenticate_user"));
        assert!(text.contains("Function"));
        assert!(text.contains("Authenticates"));
    }
}
