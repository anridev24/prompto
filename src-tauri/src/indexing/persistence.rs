use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// Configuration for where to store index files
pub struct PersistenceConfig {
    pub cache_dir: PathBuf,
}

impl PersistenceConfig {
    /// Create persistence config using Tauri's app data directory
    pub fn new(app_handle: &AppHandle) -> Result<Self, String> {
        let cache_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?
            .join("indexes");

        // Create cache directory if it doesn't exist
        fs::create_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to create cache directory: {}", e))?;

        Ok(Self { cache_dir })
    }

    /// Get the directory for a specific project's index
    pub fn get_project_dir(&self, project_path: &str) -> PathBuf {
        let hash = Self::hash_path(project_path);
        self.cache_dir.join(hash)
    }

    /// Create a simple hash of the project path for directory naming
    fn hash_path(path: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get path for the main index file
    pub fn get_main_index_path(&self, project_path: &str) -> PathBuf {
        self.get_project_dir(project_path).join("index.bin")
    }

    /// Get path for the vector index file
    pub fn get_vector_index_path(&self, project_path: &str) -> PathBuf {
        self.get_project_dir(project_path).join("vectors.usearch")
    }

    /// Get path for the vector metadata file
    pub fn get_vector_metadata_path(&self, project_path: &str) -> PathBuf {
        self.get_project_dir(project_path)
            .join("vectors_metadata.bin")
    }

    /// Get path for the Tantivy index directory
    pub fn get_tantivy_dir(&self, project_path: &str) -> PathBuf {
        self.get_project_dir(project_path).join("tantivy")
    }

    /// Get path for the cache metadata file
    pub fn get_cache_metadata_path(&self, project_path: &str) -> PathBuf {
        self.get_project_dir(project_path).join("metadata.json")
    }

    /// Check if a cached index exists for a project
    pub fn has_cached_index(&self, project_path: &str) -> bool {
        let main_index = self.get_main_index_path(project_path);
        let metadata = self.get_cache_metadata_path(project_path);
        main_index.exists() && metadata.exists()
    }

    /// Delete cached index for a project
    pub fn clear_project_cache(&self, project_path: &str) -> Result<(), String> {
        let project_dir = self.get_project_dir(project_path);
        if project_dir.exists() {
            fs::remove_dir_all(&project_dir)
                .map_err(|e| format!("Failed to remove cache directory: {}", e))?;
        }
        Ok(())
    }

    /// Get all cached project paths
    pub fn get_cached_projects(&self) -> Result<Vec<CacheInfo>, String> {
        let mut projects = Vec::new();

        if !self.cache_dir.exists() {
            return Ok(projects);
        }

        let entries = fs::read_dir(&self.cache_dir)
            .map_err(|e| format!("Failed to read cache directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.is_dir() {
                let metadata_path = path.join("metadata.json");
                if metadata_path.exists() {
                    if let Ok(metadata) = CacheMetadata::load(&metadata_path) {
                        let size = Self::calculate_dir_size(&path).unwrap_or(0);
                        projects.push(CacheInfo {
                            project_path: metadata.project_path,
                            cached_at: metadata.cached_at,
                            file_count: metadata.file_count,
                            size_bytes: size,
                        });
                    }
                }
            }
        }

        Ok(projects)
    }

    /// Calculate total size of a directory
    fn calculate_dir_size(path: &Path) -> Result<u64, std::io::Error> {
        let mut total = 0;
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    total += entry.metadata()?.len();
                } else if path.is_dir() {
                    total += Self::calculate_dir_size(&path)?;
                }
            }
        }
        Ok(total)
    }
}

/// Metadata about a cached index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub project_path: String,
    pub cached_at: u64,
    pub file_count: usize,
    pub file_timestamps: HashMap<String, u64>,
}

impl CacheMetadata {
    pub fn new(project_path: String, file_count: usize, file_timestamps: HashMap<String, u64>) -> Self {
        Self {
            project_path,
            cached_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            file_count,
            file_timestamps,
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

        fs::write(path, json).map_err(|e| format!("Failed to write metadata: {}", e))?;

        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let json = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read metadata: {}", e))?;

        serde_json::from_str(&json).map_err(|e| format!("Failed to parse metadata: {}", e))
    }

    /// Check if the cache is still valid by comparing file timestamps
    pub fn is_valid(&self, current_timestamps: &HashMap<String, u64>) -> bool {
        // Check if file count matches
        if self.file_timestamps.len() != current_timestamps.len() {
            return false;
        }

        // Check if any file has been modified
        for (path, &cached_time) in &self.file_timestamps {
            match current_timestamps.get(path) {
                Some(&current_time) if current_time == cached_time => continue,
                _ => return false, // File was modified or removed
            }
        }

        // Check for new files
        for path in current_timestamps.keys() {
            if !self.file_timestamps.contains_key(path) {
                return false;
            }
        }

        true
    }
}

/// Information about a cached project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInfo {
    pub project_path: String,
    pub cached_at: u64,
    pub file_count: usize,
    pub size_bytes: u64,
}
