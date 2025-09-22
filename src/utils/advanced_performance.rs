#![allow(dead_code)] // Advanced performance utilities - may not all be used yet

use std::path::Path;
use std::fs::File;
use std::sync::Arc;
use memmap2::{Mmap, MmapOptions};
use blake3::Hasher;
use bumpalo::Bump;
use parking_lot::Mutex;
use dashmap::DashMap;
use crate::utils::{Result, UltraError};

/// Content hash for incremental compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub fn new(data: &[u8]) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(data);
        ContentHash(hasher.finalize().into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Memory-mapped file reader for zero-copy performance
pub struct MmapFileReader {
    _file: File,
    mmap: Mmap,
}

impl MmapFileReader {
    /// Create a new memory-mapped file reader
    pub fn new(path: &Path) -> Result<Self> {
        let file = File::open(path)
            .map_err(|e| UltraError::Io(e))?;

        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)
                .map_err(|e| UltraError::Build(format!("Memory mapping failed: {}", e)))?
        };

        Ok(Self {
            _file: file,
            mmap,
        })
    }

    /// Get the content as a string slice (zero-copy)
    pub fn as_str(&self) -> Result<&str> {
        std::str::from_utf8(&self.mmap)
            .map_err(|e| UltraError::Build(format!("Invalid UTF-8 in file: {}", e)))
    }

    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.mmap
    }

    /// Compute content hash efficiently
    pub fn compute_hash(&self) -> ContentHash {
        ContentHash::new(&self.mmap)
    }
}

// Arena allocator for fast bulk operations (thread-local for performance)
thread_local! {
    static ARENA: Bump = Bump::new();
}

pub struct UltraArena {
    // Use simple counters for stats instead of direct arena access
    allocations: Arc<parking_lot::Mutex<ArenaStats>>,
}

impl UltraArena {
    pub fn new() -> Self {
        Self {
            allocations: Arc::new(parking_lot::Mutex::new(ArenaStats {
                ast_bytes: 0,
                string_bytes: 0,
                temp_bytes: 0,
            })),
        }
    }

    /// Allocate a string efficiently (returns owned string for simplicity)
    pub fn alloc_str(&self, s: &str) -> String {
        let mut stats = self.allocations.lock();
        stats.string_bytes += s.len();
        s.to_string()
    }

    /// Allocate a slice efficiently (returns owned vec for simplicity)
    pub fn alloc_slice<T: Clone>(&self, slice: &[T]) -> Vec<T> {
        let mut stats = self.allocations.lock();
        stats.temp_bytes += std::mem::size_of::<T>() * slice.len();
        slice.to_vec()
    }

    /// Reset arena counters
    pub fn reset(&self) {
        let mut stats = self.allocations.lock();
        *stats = ArenaStats {
            ast_bytes: 0,
            string_bytes: 0,
            temp_bytes: 0,
        };
    }

    /// Get memory usage statistics
    pub fn memory_usage(&self) -> ArenaStats {
        self.allocations.lock().clone()
    }
}

impl Default for UltraArena {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ArenaStats {
    pub ast_bytes: usize,
    pub string_bytes: usize,
    pub temp_bytes: usize,
}

impl ArenaStats {
    pub fn total_bytes(&self) -> usize {
        self.ast_bytes + self.string_bytes + self.temp_bytes
    }
}

/// Incremental compilation cache with content-addressable storage
pub struct IncrementalCache {
    content_cache: DashMap<ContentHash, String>,
    dependency_graph: DashMap<String, Vec<String>>,
    file_hashes: DashMap<String, ContentHash>,
}

impl IncrementalCache {
    pub fn new() -> Self {
        Self {
            content_cache: DashMap::new(),
            dependency_graph: DashMap::new(),
            file_hashes: DashMap::new(),
        }
    }

    /// Get cached result or compute if not available
    pub fn get_or_compute<F>(&self, path: &str, content_hash: ContentHash, compute_fn: F) -> Result<String>
    where
        F: FnOnce() -> Result<String>,
    {
        // Check if content has changed
        if let Some(cached_hash) = self.file_hashes.get(path) {
            if *cached_hash == content_hash {
                if let Some(cached_result) = self.content_cache.get(&content_hash) {
                    return Ok(cached_result.clone());
                }
            }
        }

        // Compute new result
        let result = compute_fn()?;

        // Cache the result
        self.content_cache.insert(content_hash, result.clone());
        self.file_hashes.insert(path.to_string(), content_hash);

        Ok(result)
    }

    /// Add dependency relationship
    pub fn add_dependency(&self, file: &str, dependency: &str) {
        self.dependency_graph
            .entry(file.to_string())
            .or_insert_with(Vec::new)
            .push(dependency.to_string());
    }

    /// Check if any dependencies have changed
    pub fn dependencies_changed(&self, file: &str) -> bool {
        if let Some(deps) = self.dependency_graph.get(file) {
            deps.iter().any(|dep| {
                // Check if dependency file has changed
                self.file_hashes.get(dep).is_none()
            })
        } else {
            false
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> IncrementalCacheStats {
        IncrementalCacheStats {
            content_entries: self.content_cache.len(),
            dependency_entries: self.dependency_graph.len(),
            file_hash_entries: self.file_hashes.len(),
        }
    }

    /// Clear all caches
    pub fn clear(&self) {
        self.content_cache.clear();
        self.dependency_graph.clear();
        self.file_hashes.clear();
    }
}

impl Default for IncrementalCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct IncrementalCacheStats {
    pub content_entries: usize,
    pub dependency_entries: usize,
    pub file_hash_entries: usize,
}

/// Parallel file processing utilities
pub mod parallel_files {
    use super::*;
    use rayon::prelude::*;
    use std::path::PathBuf;

    /// Process multiple files in parallel with memory mapping
    pub fn process_files_parallel<F, R>(paths: &[PathBuf], processor: F) -> Vec<Result<R>>
    where
        F: Fn(&MmapFileReader) -> Result<R> + Send + Sync,
        R: Send,
    {
        paths
            .par_iter()
            .map(|path| {
                let reader = MmapFileReader::new(path)?;
                processor(&reader)
            })
            .collect()
    }

    /// Batch process files with optimal chunk sizes
    pub fn batch_process_files<F, R>(paths: &[PathBuf], processor: F) -> Vec<Result<R>>
    where
        F: Fn(&[PathBuf]) -> Vec<Result<R>> + Send + Sync,
        R: Send,
    {
        let chunk_size = optimal_chunk_size(paths.len());

        paths
            .par_chunks(chunk_size)
            .flat_map(processor)
            .collect()
    }

    /// Calculate optimal chunk size based on available CPU cores
    fn optimal_chunk_size(total_items: usize) -> usize {
        let cpu_count = num_cpus::get();
        (total_items / (cpu_count * 2)).max(1).min(100)
    }
}

/// Optimized string processing (simplified SIMD for compatibility)
pub mod simd_strings {
    /// Fast string comparison with basic optimizations
    pub fn fast_string_contains(haystack: &str, needle: &str) -> bool {
        if needle.len() > haystack.len() {
            return false;
        }

        // Use standard library with small optimizations
        if needle.is_empty() {
            return true;
        }

        // For very short needles, use simple byte comparison
        if needle.len() == 1 {
            let needle_byte = needle.as_bytes()[0];
            return haystack.as_bytes().iter().any(|&b| b == needle_byte);
        }

        // Use standard contains for general case (still very fast)
        haystack.contains(needle)
    }

    /// Fast whitespace trimming
    pub fn fast_trim(s: &str) -> &str {
        // Use standard library trim (already highly optimized)
        s.trim()
    }

    /// Fast prefix check
    pub fn fast_starts_with(haystack: &str, needle: &str) -> bool {
        if needle.len() > haystack.len() {
            return false;
        }

        // Optimized prefix check
        &haystack.as_bytes()[..needle.len()] == needle.as_bytes()
    }

    /// Fast suffix check
    pub fn fast_ends_with(haystack: &str, needle: &str) -> bool {
        if needle.len() > haystack.len() {
            return false;
        }

        let start = haystack.len() - needle.len();
        &haystack.as_bytes()[start..] == needle.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_mmap_file_reader() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello, Ultra Bundler!").unwrap();

        let reader = MmapFileReader::new(temp_file.path()).unwrap();
        let content = reader.as_str().unwrap();

        assert!(content.contains("Hello, Ultra Bundler!"));
    }

    #[test]
    fn test_content_hash() {
        let data1 = b"Hello, World!";
        let data2 = b"Hello, World!";
        let data3 = b"Hello, Ultra!";

        let hash1 = ContentHash::new(data1);
        let hash2 = ContentHash::new(data2);
        let hash3 = ContentHash::new(data3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_ultra_arena() {
        let arena = UltraArena::new();

        let s1 = arena.alloc_str("Hello");
        let s2 = arena.alloc_str("World");

        assert_eq!(s1, "Hello");
        assert_eq!(s2, "World");

        let stats = arena.memory_usage();
        assert!(stats.total_bytes() > 0);

        arena.reset();
        let stats_after_reset = arena.memory_usage();
        assert_eq!(stats_after_reset.total_bytes(), 0);
    }

    #[test]
    fn test_incremental_cache() {
        let cache = IncrementalCache::new();
        let hash1 = ContentHash::new(b"content1");
        let hash2 = ContentHash::new(b"content2");

        // First computation
        let result1 = cache.get_or_compute("file1", hash1, || Ok("result1".to_string())).unwrap();
        assert_eq!(result1, "result1");

        // Second computation with same hash (should be cached)
        let result2 = cache.get_or_compute("file1", hash1, || Ok("different_result".to_string())).unwrap();
        assert_eq!(result2, "result1"); // Should return cached result

        // Third computation with different hash
        let result3 = cache.get_or_compute("file1", hash2, || Ok("result2".to_string())).unwrap();
        assert_eq!(result3, "result2");

        let stats = cache.stats();
        assert!(stats.content_entries >= 2);
    }

    #[test]
    fn test_simd_string_contains() {
        use simd_strings::*;

        assert!(fast_string_contains("Hello, World!", "World"));
        assert!(fast_string_contains("TypeScript is awesome", "Script"));
        assert!(!fast_string_contains("JavaScript", "Python"));
        assert!(fast_string_contains("", ""));
        assert!(!fast_string_contains("short", "longer_than_haystack"));
    }

    #[test]
    fn test_fast_trim() {
        use simd_strings::*;

        assert_eq!(fast_trim("  Hello  "), "Hello");
        assert_eq!(fast_trim("\t\nWorld\r\n"), "World");
        assert_eq!(fast_trim("NoWhitespace"), "NoWhitespace");
        assert_eq!(fast_trim("   "), "");
        assert_eq!(fast_trim(""), "");
    }

    #[test]
    fn test_fast_prefix_suffix() {
        use simd_strings::*;

        assert!(fast_starts_with("hello world", "hello"));
        assert!(!fast_starts_with("hello world", "world"));
        assert!(fast_ends_with("hello world", "world"));
        assert!(!fast_ends_with("hello world", "hello"));
    }
}