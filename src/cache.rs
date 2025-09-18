use anyhow::Result;
use blake3::Hash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub hash: String,
    pub dependencies: Vec<PathBuf>,
    pub ast: Option<Vec<u8>>, // Serialized AST
    pub transformed_code: Option<String>,
    pub source_map: Option<String>,
    pub metadata: CacheMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub module_type: ModuleType,
    pub exports: Vec<String>,
    pub imports: Vec<ImportInfo>,
    pub has_side_effects: bool,
    pub is_entry: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleType {
    JavaScript,
    TypeScript,
    Jsx,
    Tsx,
    Css,
    Json,
    Asset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    pub specifier: String,
    pub kind: ImportKind,
    pub source_location: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportKind {
    Static,
    Dynamic,
    Css,
    Asset,
}

pub struct Cache {
    cache_dir: PathBuf,
    memory_cache: HashMap<String, CacheEntry>,
}

impl Cache {
    pub fn new(cache_dir: &Path) -> Result<Self> {
        fs::create_dir_all(cache_dir)?;

        Ok(Self {
            cache_dir: cache_dir.to_path_buf(),
            memory_cache: HashMap::new(),
        })
    }

    pub fn get(&mut self, key: &str) -> Option<&CacheEntry> {
        if self.memory_cache.contains_key(key) {
            return self.memory_cache.get(key);
        }

        // Try to load from disk
        if let Ok(entry) = self.load_from_disk(key) {
            self.memory_cache.insert(key.to_string(), entry);
            return self.memory_cache.get(key);
        }

        None
    }

    pub fn set(&mut self, key: String, entry: CacheEntry) -> Result<()> {
        self.save_to_disk(&key, &entry)?;
        self.memory_cache.insert(key, entry);
        Ok(())
    }

    pub fn invalidate(&mut self, key: &str) {
        self.memory_cache.remove(key);
        let _ = fs::remove_file(self.get_cache_path(key));
    }

    pub fn compute_content_hash(&self, content: &[u8]) -> String {
        blake3::hash(content).to_hex().to_string()
    }

    pub fn compute_file_key(&self, path: &Path, conditions: &[String], defines: &HashMap<String, String>) -> Result<String> {
        let metadata = fs::metadata(path)?;
        let mtime = metadata.modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let key_data = format!(
            "{}:{}:{}:{}",
            path.display(),
            mtime,
            conditions.join(","),
            defines.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(",")
        );

        Ok(blake3::hash(key_data.as_bytes()).to_hex().to_string())
    }

    fn load_from_disk(&self, key: &str) -> Result<CacheEntry> {
        let path = self.get_cache_path(key);
        let content = fs::read(&path)?;
        let entry: CacheEntry = bincode::deserialize(&content)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize cache entry: {}", e))?;
        Ok(entry)
    }

    fn save_to_disk(&self, key: &str, entry: &CacheEntry) -> Result<()> {
        let path = self.get_cache_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let serialized = bincode::serialize(entry)
            .map_err(|e| anyhow::anyhow!("Failed to serialize cache entry: {}", e))?;
        fs::write(&path, serialized)?;
        Ok(())
    }

    fn get_cache_path(&self, key: &str) -> PathBuf {
        let prefix = &key[..2.min(key.len())];
        self.cache_dir
            .join("modules")
            .join(prefix)
            .join(format!("{}.cache", key))
    }

    pub fn clear(&mut self) -> Result<()> {
        self.memory_cache.clear();
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
            fs::create_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }

    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            memory_entries: self.memory_cache.len(),
            disk_size: self.calculate_disk_size(),
        }
    }

    fn calculate_disk_size(&self) -> u64 {
        let mut size = 0;
        if let Ok(entries) = fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    size += metadata.len();
                }
            }
        }
        size
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub memory_entries: usize,
    pub disk_size: u64,
}