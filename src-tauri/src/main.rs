// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod indexing;
mod models;

use commands::index_commands::*;
use indexing::tree_sitter_indexer::TreeSitterIndexer;
use std::sync::Mutex;

fn main() {
    // Initialize indexer state
    let indexer = TreeSitterIndexer::new().expect("Failed to initialize tree-sitter indexer");

    let indexer_state = IndexerState {
        indexer: Mutex::new(indexer),
        current_index: Mutex::new(None),
    };

    tauri::Builder::default()
        .manage(indexer_state)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            index_codebase,
            query_index,
            get_index_stats,
            get_file_symbols,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
