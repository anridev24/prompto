use crate::indexing::persistence::{CacheMetadata, PersistenceConfig};
use crate::indexing::tree_sitter_indexer::TreeSitterIndexer;
use crate::models::code_index::*;
use std::sync::Mutex;
use tauri::{AppHandle, State};

// Global state for the indexer
pub struct IndexerState {
    pub indexer: Mutex<TreeSitterIndexer>,
    pub current_index: Mutex<Option<CodebaseIndex>>,
    pub persistence: Mutex<Option<PersistenceConfig>>,
}

#[tauri::command]
pub async fn index_codebase(
    path: String,
    app_handle: AppHandle,
    state: State<'_, IndexerState>,
    force_reindex: Option<bool>,
) -> Result<IndexResult, String> {
    let start_time = std::time::Instant::now();
    let force_reindex = force_reindex.unwrap_or(false);

    // Initialize persistence config if not already done
    let mut persistence_lock = state
        .persistence
        .lock()
        .map_err(|e| format!("Failed to lock persistence: {}", e))?;

    if persistence_lock.is_none() {
        *persistence_lock = Some(PersistenceConfig::new(&app_handle)?);
    }

    let persistence = persistence_lock
        .as_ref()
        .ok_or_else(|| "Persistence not initialized".to_string())?;

    // Check if we have a valid cache
    let use_cache = !force_reindex && persistence.has_cached_index(&path);

    if use_cache {
        // Try to load from cache
        println!("Checking cache validity for: {}", path);

        let cache_metadata_path = persistence.get_cache_metadata_path(&path);
        if let Ok(cached_metadata) = CacheMetadata::load(&cache_metadata_path) {
            // Collect current timestamps
            let current_timestamps = TreeSitterIndexer::collect_file_timestamps(&path)?;

            // Check if cache is still valid
            if cached_metadata.is_valid(&current_timestamps) {
                println!("Cache is valid, loading from disk...");

                // Load main index
                let main_index_path = persistence.get_main_index_path(&path);
                let index = CodebaseIndex::load(&main_index_path)?;

                // Get indexer and set up Tantivy path
                let mut indexer = state
                    .indexer
                    .lock()
                    .map_err(|e| format!("Failed to lock indexer: {}", e))?;

                let tantivy_dir = persistence.get_tantivy_dir(&path);
                indexer.set_tantivy_path(tantivy_dir)?;

                // Load vector store
                let vector_index_path = persistence.get_vector_index_path(&path);
                let vector_metadata_path = persistence.get_vector_metadata_path(&path);
                indexer.load_vector_store(&vector_index_path, &vector_metadata_path)?;

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

                println!("Loaded from cache in {:?}", start_time.elapsed());
                return Ok(result);
            } else {
                println!("Cache is stale, re-indexing...");
            }
        }
    }

    drop(persistence_lock); // Release lock before indexing

    // Perform fresh indexing
    println!("Starting fresh indexing for: {}", path);

    // Get persistence config again (after dropping lock)
    let persistence_lock = state
        .persistence
        .lock()
        .map_err(|e| format!("Failed to lock persistence: {}", e))?;
    let persistence = persistence_lock
        .as_ref()
        .ok_or_else(|| "Persistence not initialized".to_string())?;

    // Create project directory
    let project_dir = persistence.get_project_dir(&path);
    std::fs::create_dir_all(&project_dir)
        .map_err(|e| format!("Failed to create project directory: {}", e))?;

    // Get indexer and set Tantivy path
    let mut indexer = state
        .indexer
        .lock()
        .map_err(|e| format!("Failed to lock indexer: {}", e))?;

    let tantivy_dir = persistence.get_tantivy_dir(&path);
    indexer.set_tantivy_path(tantivy_dir)?;

    // Perform indexing
    let index = indexer.index_codebase(&path)?;

    // Save everything to disk
    println!("Saving index to cache...");

    // Save main index
    let main_index_path = persistence.get_main_index_path(&path);
    index.save(&main_index_path)?;

    // Save vector store
    let vector_index_path = persistence.get_vector_index_path(&path);
    let vector_metadata_path = persistence.get_vector_metadata_path(&path);
    indexer.save_vector_store(&vector_index_path, &vector_metadata_path)?;

    // Collect and save cache metadata
    let file_timestamps = TreeSitterIndexer::collect_file_timestamps(&path)?;
    let cache_metadata = CacheMetadata::new(path.clone(), index.total_files, file_timestamps);
    let cache_metadata_path = persistence.get_cache_metadata_path(&path);
    cache_metadata.save(&cache_metadata_path)?;

    println!("Index saved to cache");

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

#[tauri::command]
pub async fn search_files(
    query: String,
    max_results: Option<usize>,
    state: State<'_, IndexerState>,
) -> Result<Vec<String>, String> {
    let indexer = state.indexer.lock()
        .map_err(|e| format!("Failed to lock indexer: {}", e))?;

    let index_lock = state.current_index.lock()
        .map_err(|e| format!("Failed to lock index: {}", e))?;

    let index = index_lock.as_ref()
        .ok_or_else(|| "No codebase indexed".to_string())?;

    Ok(indexer.query_file_paths(index, &query, max_results.unwrap_or(50)))
}

#[tauri::command]
pub async fn search_semantic(
    query: String,
    max_results: Option<usize>,
    state: State<'_, IndexerState>,
) -> Result<Vec<CodeChunk>, String> {
    let indexer = state.indexer.lock()
        .map_err(|e| format!("Failed to lock indexer: {}", e))?;

    indexer.search_semantic(&query, max_results.unwrap_or(20))
}
