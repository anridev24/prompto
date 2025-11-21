pub mod models;

use models::{Message, MessageRequest, MessageResponse};
use reqwest::Client;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicClient {
    client: Client,
    api_key: String,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub async fn create_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> Result<MessageResponse, String> {
        let request = MessageRequest {
            model: model.to_string(),
            max_tokens,
            messages,
            system,
            temperature,
            top_p: None,
        };

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("API request failed with status {}: {}", status, error_text));
        }

        response
            .json::<MessageResponse>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    pub async fn analyze_intent(&self, prompt: &str) -> Result<String, String> {
        let system_prompt = r#"You are an expert at analyzing user intent for code-related tasks. Extract structured information from prompts and return ONLY valid JSON with no markdown formatting.

Return a JSON object with:
- "action": The primary action (create/modify/fix/explain/refactor/other)
- "keywords": Array of key technical terms (3-8 terms)
- "scope": The scope level (file/function/class/module/project)
- "entities": Array of specific names mentioned (files, functions, classes, variables)"#;

        let messages = vec![
            Message {
                role: "user".to_string(),
                content: format!("Analyze this prompt and extract intent:\n\n{}", prompt),
            },
        ];

        let response = self
            .create_message("claude-sonnet-4-5-20250929", 1024, messages, Some(system_prompt.to_string()), Some(0.3))
            .await?;

        // Extract text from first content block
        if let Some(content_block) = response.content.first() {
            if let Some(text) = &content_block.text {
                return Ok(text.clone());
            }
        }

        Err("No content in response".to_string())
    }

    pub async fn extract_patterns(&self, code_snippets: &str) -> Result<String, String> {
        let system_prompt = r#"You are an expert code analyst. Analyze code to identify patterns, conventions, and architectural insights that would help a developer write consistent code.

Focus on:
- Naming conventions (variables, functions, classes)
- Code organization patterns
- Error handling approaches
- Common design patterns used
- Testing strategies
- Documentation style"#;

        let messages = vec![
            Message {
                role: "user".to_string(),
                content: format!(
                    "Analyze the following code and extract common patterns and conventions:\n\n{}",
                    code_snippets
                ),
            },
        ];

        let response = self
            .create_message("claude-sonnet-4-5-20250929", 2048, messages, Some(system_prompt.to_string()), Some(0.5))
            .await?;

        // Extract text from first content block
        if let Some(content_block) = response.content.first() {
            if let Some(text) = &content_block.text {
                return Ok(text.clone());
            }
        }

        Err("No content in response".to_string())
    }
}
