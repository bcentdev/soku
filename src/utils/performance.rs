use dashmap::DashMap;
use std::sync::Arc;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Ultra-fast caching system for build artifacts
pub struct UltraCache {
    js_cache: Arc<DashMap<String, String>>,
    css_cache: Arc<DashMap<String, String>>,
    parse_cache: Arc<DashMap<u64, String>>,
}

impl UltraCache {
    pub fn new() -> Self {
        Self {
            js_cache: Arc::new(DashMap::new()),
            css_cache: Arc::new(DashMap::new()),
            parse_cache: Arc::new(DashMap::new()),
        }
    }

    /// Cache JS processing result
    pub fn cache_js(&self, path: &str, content: &str, result: String) {
        let key = format!("{}:{}", path, self.hash_content(content));
        self.js_cache.insert(key, result);
    }

    /// Get cached JS result
    pub fn get_js(&self, path: &str, content: &str) -> Option<String> {
        let key = format!("{}:{}", path, self.hash_content(content));
        self.js_cache.get(&key).map(|v| v.clone())
    }

    /// Cache CSS processing result
    pub fn cache_css(&self, path: &str, content: &str, result: String) {
        let key = format!("{}:{}", path, self.hash_content(content));
        self.css_cache.insert(key, result);
    }

    /// Get cached CSS result
    pub fn get_css(&self, path: &str, content: &str) -> Option<String> {
        let key = format!("{}:{}", path, self.hash_content(content));
        self.css_cache.get(&key).map(|v| v.clone())
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

impl Default for UltraCache {
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
        (total_items / cpu_count).max(1).min(100)
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