#![allow(dead_code)] // Performance utilities - may not all be used yet

use dashmap::DashMap;
use std::sync::Arc;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use sled::{Db, Tree};
use string_interner::{StringInterner, DefaultSymbol, DefaultBackend};

/// Soku-fast caching system with persistent storage
pub struct SokuCache {
    // In-memory hot cache for ultra-speed
    js_cache: Arc<DashMap<String, String>>,
    css_cache: Arc<DashMap<String, String>>,
    parse_cache: Arc<DashMap<u64, String>>,

    // Persistent disk cache for cross-session performance
    persistent_cache: Option<Arc<PersistentCache>>,

    // String interning for memory optimization
    string_interner: Arc<parking_lot::Mutex<StringInterner<DefaultBackend>>>,
}

/// Persistent cache using sled for cross-session performance
pub struct PersistentCache {
    db: Db,
    js_tree: Tree,
    css_tree: Tree,
    metadata_tree: Tree,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheEntry {
    pub content_hash: u64,
    pub result: String,
    pub timestamp: u64,
    pub file_size: u64,
}

impl PersistentCache {
    pub fn new(cache_dir: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        std::fs::create_dir_all(cache_dir)?;
        let db_path = cache_dir.join("soku_cache.sled");

        let db = sled::open(db_path)?;
        let js_tree = db.open_tree("js_cache")?;
        let css_tree = db.open_tree("css_cache")?;
        let metadata_tree = db.open_tree("metadata")?;

        Ok(Self {
            db,
            js_tree,
            css_tree,
            metadata_tree,
        })
    }

    pub fn get_js(&self, key: &str) -> Option<CacheEntry> {
        self.js_tree.get(key).ok()
            .and_then(|opt| opt)
            .and_then(|bytes| bincode::deserialize(&bytes).ok())
    }

    pub fn set_js(&self, key: &str, entry: &CacheEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bytes = bincode::serialize(entry)?;
        self.js_tree.insert(key, bytes)?;
        self.js_tree.flush()?;
        Ok(())
    }

    pub fn get_css(&self, key: &str) -> Option<CacheEntry> {
        self.css_tree.get(key).ok()
            .and_then(|opt| opt)
            .and_then(|bytes| bincode::deserialize(&bytes).ok())
    }

    pub fn set_css(&self, key: &str, entry: &CacheEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bytes = bincode::serialize(entry)?;
        self.css_tree.insert(key, bytes)?;
        self.css_tree.flush()?;
        Ok(())
    }

    pub fn is_valid(&self, entry: &CacheEntry, current_hash: u64, current_size: u64) -> bool {
        entry.content_hash == current_hash && entry.file_size == current_size
    }
}

impl SokuCache {
    pub fn new() -> Self {
        Self {
            js_cache: Arc::new(DashMap::new()),
            css_cache: Arc::new(DashMap::new()),
            parse_cache: Arc::new(DashMap::new()),
            persistent_cache: None,
            string_interner: Arc::new(parking_lot::Mutex::new(StringInterner::default())),
        }
    }

    pub fn with_persistent_cache(cache_dir: &Path) -> Self {
        let persistent = PersistentCache::new(cache_dir).ok()
            .map(Arc::new);

        Self {
            js_cache: Arc::new(DashMap::new()),
            css_cache: Arc::new(DashMap::new()),
            parse_cache: Arc::new(DashMap::new()),
            persistent_cache: persistent,
            string_interner: Arc::new(parking_lot::Mutex::new(StringInterner::default())),
        }
    }

    /// Intern a string for memory efficiency
    pub fn intern_string(&self, s: &str) -> DefaultSymbol {
        let mut interner = self.string_interner.lock();
        interner.get_or_intern(s)
    }

    /// Cache JS processing result with persistent storage
    pub fn cache_js(&self, path: &str, content: &str, result: String) {
        let content_hash = self.hash_content(content);
        let key = format!("{}:{}", path, content_hash);

        // Hot cache for ultra-speed
        self.js_cache.insert(key.clone(), result.clone());

        // Persistent cache for cross-session performance
        if let Some(ref persistent) = self.persistent_cache {
            let entry = CacheEntry {
                content_hash,
                result,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                file_size: content.len() as u64,
            };
            let _ = persistent.set_js(&key, &entry);
        }
    }

    /// Get cached JS result with persistent fallback
    pub fn get_js(&self, path: &str, content: &str) -> Option<String> {
        let content_hash = self.hash_content(content);
        let key = format!("{}:{}", path, content_hash);

        // Check hot cache first
        if let Some(cached) = self.js_cache.get(&key) {
            return Some(cached.clone());
        }

        // Check persistent cache
        if let Some(ref persistent) = self.persistent_cache {
            if let Some(entry) = persistent.get_js(&key) {
                if persistent.is_valid(&entry, content_hash, content.len() as u64) {
                    // Promote to hot cache
                    self.js_cache.insert(key, entry.result.clone());
                    return Some(entry.result);
                }
            }
        }

        None
    }

    /// Cache CSS processing result with persistent storage
    pub fn cache_css(&self, path: &str, content: &str, result: String) {
        let content_hash = self.hash_content(content);
        let key = format!("{}:{}", path, content_hash);

        // Hot cache for ultra-speed
        self.css_cache.insert(key.clone(), result.clone());

        // Persistent cache for cross-session performance
        if let Some(ref persistent) = self.persistent_cache {
            let entry = CacheEntry {
                content_hash,
                result,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                file_size: content.len() as u64,
            };
            let _ = persistent.set_css(&key, &entry);
        }
    }

    /// Get cached CSS result with persistent fallback
    pub fn get_css(&self, path: &str, content: &str) -> Option<String> {
        let content_hash = self.hash_content(content);
        let key = format!("{}:{}", path, content_hash);

        // Check hot cache first
        if let Some(cached) = self.css_cache.get(&key) {
            return Some(cached.clone());
        }

        // Check persistent cache
        if let Some(ref persistent) = self.persistent_cache {
            if let Some(entry) = persistent.get_css(&key) {
                if persistent.is_valid(&entry, content_hash, content.len() as u64) {
                    // Promote to hot cache
                    self.css_cache.insert(key, entry.result.clone());
                    return Some(entry.result);
                }
            }
        }

        None
    }

    /// Cache parsed AST or processing result
    pub fn cache_parse(&self, content: &str, result: String) {
        let hash = self.hash_content(content);
        self.parse_cache.insert(hash, result);
    }

    /// Get cached parse result
    pub fn get_parse(&self, content: &str) -> Option<String> {
        let hash = self.hash_content(content);
        self.parse_cache.get(&hash).map(|v| v.clone())
    }

    /// Fast hash function for content
    fn hash_content(&self, content: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Clear all caches
    pub fn clear(&self) {
        self.js_cache.clear();
        self.css_cache.clear();
        self.parse_cache.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            js_entries: self.js_cache.len(),
            css_entries: self.css_cache.len(),
            parse_entries: self.parse_cache.len(),
        }
    }
}

impl Default for SokuCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub js_entries: usize,
    pub css_entries: usize,
    pub parse_entries: usize,
}

/// Parallel processing utilities
pub mod parallel {
    use rayon::prelude::*;
    use std::sync::Arc;
    use futures::future::join_all;
    use tokio::task;

    /// Process multiple items in parallel using Rayon
    pub fn process_parallel<T, R, F>(items: &[T], processor: F) -> Vec<R>
    where
        T: Sync,
        R: Send,
        F: Fn(&T) -> R + Sync + Send,
    {
        items.par_iter().map(processor).collect()
    }

    /// Process async items in parallel using tokio
    pub async fn process_async_parallel<T, R, F, Fut>(items: Vec<T>, processor: F) -> Vec<R>
    where
        T: Send + 'static,
        R: Send + 'static,
        F: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = R> + Send,
    {
        let processor = Arc::new(processor);
        let tasks: Vec<_> = items
            .into_iter()
            .map(|item| {
                let processor = processor.clone();
                task::spawn(async move { processor(item).await })
            })
            .collect();

        join_all(tasks)
            .await
            .into_iter()
            .map(|result| result.unwrap())
            .collect()
    }

    /// Chunk work for optimal CPU utilization
    pub fn optimal_chunk_size(total_items: usize) -> usize {
        let cpu_count = num_cpus::get();
        (total_items / cpu_count).clamp(1, 100)
    }
}

/// Memory optimization utilities
pub mod memory {
    use std::sync::Arc;

    /// String interning for memory efficiency
    pub struct StringInterner {
        strings: dashmap::DashMap<String, Arc<str>>,
    }

    impl StringInterner {
        pub fn new() -> Self {
            Self {
                strings: dashmap::DashMap::new(),
            }
        }

        /// Intern a string for memory efficiency
        pub fn intern(&self, s: String) -> Arc<str> {
            if let Some(interned) = self.strings.get(&s) {
                interned.clone()
            } else {
                let arc_str: Arc<str> = s.clone().into();
                self.strings.insert(s, arc_str.clone());
                arc_str
            }
        }

        /// Get statistics
        pub fn len(&self) -> usize {
            self.strings.len()
        }
    }

    impl Default for StringInterner {
        fn default() -> Self {
            Self::new()
        }
    }
}