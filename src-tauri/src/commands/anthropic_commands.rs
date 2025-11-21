use crate::anthropic::AnthropicClient;

#[tauri::command]
pub async fn analyze_intent(api_key: String, prompt: String) -> Result<String, String> {
    let client = AnthropicClient::new(api_key);
    client.analyze_intent(&prompt).await
}

#[tauri::command]
pub async fn extract_patterns(api_key: String, code_snippets: String) -> Result<String, String> {
    let client = AnthropicClient::new(api_key);
    client.extract_patterns(&code_snippets).await
}
