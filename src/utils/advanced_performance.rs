use std::path::Path;
use std::fs::File;
use memmap2::{Mmap, MmapOptions};
use blake3::Hasher;
use dashmap::DashMap;
use crate::utils::{Result, SokuError};

/// Content hash for incremental compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub fn new(data: &[u8]) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(data);
        ContentHash(hasher.finalize().into())
    }

}

/// Memory-mapped file reader for zero-copy performance
#[allow(dead_code)]
pub struct MmapFileReader {
    _file: File,
    mmap: Mmap,
}

impl MmapFileReader {
    /// Create a new memory-mapped file reader
    pub fn new(path: &Path) -> Result<Self> {
        let file = File::open(path)
            .map_err(SokuError::Io)?;

        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)
                .map_err(|e| SokuError::build(format!("Memory mapping failed: {}", e)))?
        };

        Ok(Self {
            _file: file,
            mmap,
        })
    }

    /// Get the content as a string slice (zero-copy)
    pub fn as_str(&self) -> Result<&str> {
        std::str::from_utf8(&self.mmap)
            .map_err(|e| SokuError::build(format!("Invalid UTF-8 in file: {}", e)))
    }

    /// Get the raw bytes
    #[allow(dead_code)] // Future use for binary file processing
    pub fn as_bytes(&self) -> &[u8] {
        &self.mmap
    }

    /// Compute content hash efficiently
    pub fn compute_hash(&self) -> ContentHash {
        ContentHash::new(&self.mmap)
    }
}

// Arena allocator for fast bulk operations (thread-local for performance)

/// Incremental compilation cache with content-addressable storage
#[allow(dead_code)]
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
    #[allow(dead_code)] // Future use for dependency tracking
    pub fn add_dependency(&self, file: &str, dependency: &str) {
        self.dependency_graph
            .entry(file.to_string())
            .or_insert_with(Vec::new)
            .push(dependency.to_string());
    }

    /// Check if any dependencies have changed
    #[allow(dead_code)] // Future use for incremental compilation
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
    #[allow(dead_code)] // Future use for cache analytics
    pub fn stats(&self) -> IncrementalCacheStats {
        IncrementalCacheStats {
            content_entries: self.content_cache.len(),
            dependency_entries: self.dependency_graph.len(),
            file_hash_entries: self.file_hashes.len(),
        }
    }

    /// Clear all caches
    #[allow(dead_code)] // Future use for cache management
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
#[allow(dead_code)]
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

}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_mmap_file_reader() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello, Soku Bundler!").unwrap();

        let reader = MmapFileReader::new(temp_file.path()).unwrap();
        let content = reader.as_str().unwrap();

        assert!(content.contains("Hello, Soku Bundler!"));
    }

    #[test]
    fn test_content_hash() {
        let data1 = b"Hello, World!";
        let data2 = b"Hello, World!";
        let data3 = b"Hello, Soku!";

        let hash1 = ContentHash::new(data1);
        let hash2 = ContentHash::new(data2);
        let hash3 = ContentHash::new(data3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
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

}