
use crate::core::{interfaces::FileSystemService, models::*};
use crate::utils::{Result, UltraError, MmapFileReader, IncrementalCache, ContentHash};
use crate::utils::parallel_files;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use dashmap::DashMap;

/// Ultra-performance file system service with memory mapping and caching
#[allow(dead_code)]
pub struct UltraFileSystemService {
    incremental_cache: Arc<IncrementalCache>,
    file_metadata: Arc<DashMap<PathBuf, FileMetadata>>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct FileMetadata {
    content_hash: ContentHash,
    last_modified: std::time::SystemTime,
    size: u64,
}

impl UltraFileSystemService {
    pub fn new() -> Self {
        Self {
            incremental_cache: Arc::new(IncrementalCache::new()),
            file_metadata: Arc::new(DashMap::new()),
        }
    }

    /// Scan directory with parallel processing and memory mapping
    pub async fn scan_directory_ultra(&self, path: &Path) -> Result<ProjectStructure> {
        let _timer = crate::utils::Timer::start("Ultra directory scanning");

        // Collect all files first
        let mut all_files = Vec::new();
        self.collect_files_recursive(path, &mut all_files).await?;

        // Process files in parallel using memory mapping
        let results: Vec<_> = parallel_files::process_files_parallel(
            &all_files,
            |reader| self.classify_file_content(reader)
        );

        // Aggregate results
        let mut structure = ProjectStructure::default();
        for (file_path, result) in all_files.iter().zip(results.iter()) {
            match result {
                Ok(file_type) => {
                    match file_type {
                        FileType::JavaScript | FileType::TypeScript => {
                            structure.js_modules.push(file_path.clone());
                        }
                        FileType::Css => {
                            structure.css_files.push(file_path.clone());
                        }
                        FileType::Html => {
                            structure.html_files.push(file_path.clone());
                        }
                        FileType::Other => {
                            structure.other_files.push(file_path.clone());
                        }
                    }
                }
                Err(e) => {
                    crate::utils::Logger::warn(&format!("Failed to process file {}: {}", file_path.display(), e));
                }
            }
        }

        Ok(structure)
    }

    /// Read file with memory mapping and caching
    pub async fn read_file_ultra(&self, path: &Path) -> Result<String> {
        let path_str = path.to_string_lossy();

        // Try to get from incremental cache first
        let reader = MmapFileReader::new(path)?;
        let content_hash = reader.compute_hash();

        self.incremental_cache.get_or_compute(
            &path_str,
            content_hash,
            || {
                let content = reader.as_str()?.to_string();

                // Update metadata
                let metadata = FileMetadata {
                    content_hash,
                    last_modified: std::time::SystemTime::now(),
                    size: content.len() as u64,
                };
                self.file_metadata.insert(path.to_path_buf(), metadata);

                Ok(content)
            }
        )
    }

    /// Read multiple files in parallel with memory mapping
    pub async fn read_files_parallel(&self, paths: &[PathBuf]) -> Vec<Result<String>> {
        parallel_files::process_files_parallel(paths, |reader| {
            reader.as_str().map(|s| s.to_string())
        })
    }

    /// Check if file has changed since last read
    pub fn file_changed(&self, path: &Path) -> bool {
        if let Some(metadata) = self.file_metadata.get(path) {
            if let Ok(reader) = MmapFileReader::new(path) {
                let current_hash = reader.compute_hash();
                return current_hash != metadata.content_hash;
            }
        }
        true // Assume changed if no metadata
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> UltraFileSystemStats {
        let incremental_stats = self.incremental_cache.stats();

        UltraFileSystemStats {
            cached_files: incremental_stats.content_entries,
            memory_usage_bytes: 0, // Simplified for now
            metadata_entries: self.file_metadata.len(),
        }
    }

    /// Clear all caches
    pub fn clear_caches(&self) {
        self.incremental_cache.clear();
        self.file_metadata.clear();
    }

    async fn collect_files_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        let mut entries = tokio::fs::read_dir(dir).await
            .map_err(|e| UltraError::Io(e))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| UltraError::Io(e))? {

            let path = entry.path();
            if path.is_file() {
                files.push(path);
            } else if path.is_dir() && !self.should_skip_directory(&path) {
                Box::pin(self.collect_files_recursive(&path, files)).await?;
            }
        }

        Ok(())
    }

    fn should_skip_directory(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            matches!(name, "node_modules" | ".git" | "target" | "dist" | ".next" | "build" | ".ultra-cache")
        } else {
            false
        }
    }

    fn classify_file_content(&self, reader: &MmapFileReader) -> Result<FileType> {
        let content = reader.as_str()?;

        // Use SIMD-optimized string operations for classification
        if crate::utils::simd_strings::fast_string_contains(content, "interface ") ||
           crate::utils::simd_strings::fast_string_contains(content, "type ") ||
           crate::utils::simd_strings::fast_string_contains(content, ": string") ||
           crate::utils::simd_strings::fast_string_contains(content, ": number") {
            return Ok(FileType::TypeScript);
        }

        if crate::utils::simd_strings::fast_string_contains(content, "function ") ||
           crate::utils::simd_strings::fast_string_contains(content, "const ") ||
           crate::utils::simd_strings::fast_string_contains(content, "import ") {
            return Ok(FileType::JavaScript);
        }

        if crate::utils::simd_strings::fast_string_contains(content, "@media") ||
           crate::utils::simd_strings::fast_string_contains(content, ".class") ||
           crate::utils::simd_strings::fast_string_contains(content, "color:") {
            return Ok(FileType::Css);
        }

        if crate::utils::simd_strings::fast_string_contains(content, "<html") ||
           crate::utils::simd_strings::fast_string_contains(content, "<!DOCTYPE") {
            return Ok(FileType::Html);
        }

        Ok(FileType::Other)
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum FileType {
    JavaScript,
    TypeScript,
    Css,
    Html,
    Other,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct UltraFileSystemStats {
    pub cached_files: usize,
    pub memory_usage_bytes: usize,
    pub metadata_entries: usize,
}

impl Default for UltraFileSystemService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl FileSystemService for UltraFileSystemService {
    async fn scan_directory(&self, path: &Path) -> Result<ProjectStructure> {
        self.scan_directory_ultra(path).await
    }

    async fn read_file(&self, path: &Path) -> Result<String> {
        self.read_file_ultra(path).await
    }

    async fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            self.create_directory(parent).await?;
        }

        tokio::fs::write(path, content).await
            .map_err(|e| UltraError::Io(e))?;

        // Update metadata after writing
        if let Ok(reader) = MmapFileReader::new(path) {
            let content_hash = reader.compute_hash();
            let metadata = FileMetadata {
                content_hash,
                last_modified: std::time::SystemTime::now(),
                size: content.len() as u64,
            };
            self.file_metadata.insert(path.to_path_buf(), metadata);
        }

        Ok(())
    }

    async fn create_directory(&self, path: &Path) -> Result<()> {
        tokio::fs::create_dir_all(path).await
            .map_err(|e| UltraError::Io(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio;

    #[tokio::test]
    async fn test_ultra_file_system_operations() {
        let fs = UltraFileSystemService::new();
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.js");

        // Test write and read
        let content = "const hello = 'world';";
        fs.write_file(&test_file, content).await.unwrap();

        let read_content = fs.read_file(&test_file).await.unwrap();
        assert_eq!(content, read_content);

        // Test cache hit
        let read_content_2 = fs.read_file(&test_file).await.unwrap();
        assert_eq!(content, read_content_2);

        // Test file change detection
        assert!(!fs.file_changed(&test_file));

        let stats = fs.cache_stats();
        assert!(stats.cached_files > 0);
    }

    #[tokio::test]
    async fn test_ultra_directory_scan() {
        let fs = UltraFileSystemService::new();
        let temp_dir = tempdir().unwrap();

        // Create test files
        let js_file = temp_dir.path().join("test.js");
        let ts_file = temp_dir.path().join("test.ts");
        let css_file = temp_dir.path().join("test.css");

        fs.write_file(&js_file, "function test() {}").await.unwrap();
        fs.write_file(&ts_file, "interface User { name: string; }").await.unwrap();
        fs.write_file(&css_file, ".class { color: red; }").await.unwrap();

        let structure = fs.scan_directory(temp_dir.path()).await.unwrap();

        assert!(structure.js_modules.iter().any(|p| p.file_name().unwrap() == "test.js"));
        assert!(structure.js_modules.iter().any(|p| p.file_name().unwrap() == "test.ts"));
        assert!(structure.css_files.iter().any(|p| p.file_name().unwrap() == "test.css"));
    }

    #[tokio::test]
    async fn test_parallel_file_reading() {
        let fs = UltraFileSystemService::new();
        let temp_dir = tempdir().unwrap();

        // Create multiple test files
        let mut paths = Vec::new();
        for i in 0..10 {
            let file_path = temp_dir.path().join(format!("file_{}.js", i));
            fs.write_file(&file_path, &format!("const value{} = {};", i, i)).await.unwrap();
            paths.push(file_path);
        }

        let results = fs.read_files_parallel(&paths).await;

        assert_eq!(results.len(), 10);
        for (i, result) in results.iter().enumerate() {
            let content = result.as_ref().unwrap();
            assert!(content.contains(&format!("value{}", i)));
        }
    }
}