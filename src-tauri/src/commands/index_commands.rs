use crate::indexing::tree_sitter_indexer::TreeSitterIndexer;
use crate::models::code_index::*;
use std::sync::Mutex;
use tauri::State;

// Global state for the indexer
pub struct IndexerState {
    pub indexer: Mutex<TreeSitterIndexer>,
    pub current_index: Mutex<Option<CodebaseIndex>>,
}

#[tauri::command]
pub async fn index_codebase(
    path: String,
    state: State<'_, IndexerState>,
) -> Result<IndexResult, String> {
    let start_time = std::time::Instant::now();

    // Get indexer from state
    let mut indexer = state
        .indexer
        .lock()
        .map_err(|e| format!("Failed to lock indexer: {}", e))?;

    // Perform indexing
    let index = indexer.index_codebase(&path)?;

    // Calculate result
    let total_symbols: usize = index.files.values().map(|f| f.symbols.len()).sum();

    let result = IndexResult {
        success: true,
        total_files: index.total_files,
        total_symbols,
        languages: index.language_stats.keys().cloned().collect(),
        duration_ms: start_time.elapsed().as_millis() as u64,
        errors: Vec::new(),
    };

    // Store index in state
    *state
        .current_index
        .lock()
        .map_err(|e| format!("Failed to lock index: {}", e))? = Some(index);

    Ok(result)
}

#[tauri::command]
pub async fn query_index(
    query: IndexQuery,
    state: State<'_, IndexerState>,
) -> Result<Vec<CodeChunk>, String> {
    let indexer = state
        .indexer
        .lock()
        .map_err(|e| format!("Failed to lock indexer: {}", e))?;

    let index_lock = state
        .current_index
        .lock()
        .map_err(|e| format!("Failed to lock index: {}", e))?;

    let index = index_lock
        .as_ref()
        .ok_or_else(|| "No codebase indexed".to_string())?;

    Ok(indexer.query_index(index, &query))
}

#[tauri::command]
pub async fn get_index_stats(state: State<'_, IndexerState>) -> Result<serde_json::Value, String> {
    let index_lock = state
        .current_index
        .lock()
        .map_err(|e| format!("Failed to lock index: {}", e))?;

    let index = index_lock
        .as_ref()
        .ok_or_else(|| "No codebase indexed".to_string())?;

    Ok(serde_json::json!({
        "total_files": index.total_files,
        "languages": index.language_stats,
        "root_path": index.root_path,
        "indexed_at": index.indexed_at,
    }))
}

#[tauri::command]
pub async fn get_file_symbols(
    file_path: String,
    state: State<'_, IndexerState>,
) -> Result<Vec<CodeSymbol>, String> {
    let index_lock = state
        .current_index
        .lock()
        .map_err(|e| format!("Failed to lock index: {}", e))?;

    let index = index_lock
        .as_ref()
        .ok_or_else(|| "No codebase indexed".to_string())?;

    index
        .files
        .get(&file_path)
        .map(|f| f.symbols.clone())
        .ok_or_else(|| format!("File not found: {}", file_path))
}
