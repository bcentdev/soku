// Incremental build system for Ultra Bundler
// Tracks file changes and dependencies for smart rebuilds

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use serde::{Serialize, Deserialize};

/// File metadata for change detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub content_hash: u64,
    pub modified_time: SystemTime,
    pub size: u64,
}

impl FileMetadata {
    /// Create metadata from a file path
    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let metadata = std::fs::metadata(path)?;
        let content = std::fs::read_to_string(path)?;
        let content_hash = Self::hash_content(&content);

        Ok(Self {
            path: path.to_path_buf(),
            content_hash,
            modified_time: metadata.modified()?,
            size: metadata.len(),
        })
    }

    /// Hash file content using a fast hash function
    fn hash_content(content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Check if file has changed compared to this metadata
    pub fn has_changed(&self) -> bool {
        match Self::from_file(&self.path) {
            Ok(current) => {
                // Check hash first (most accurate)
                if current.content_hash != self.content_hash {
                    return true;
                }
                // Fallback to modification time
                if current.modified_time != self.modified_time {
                    return true;
                }
                // Check size as final fallback
                current.size != self.size
            }
            Err(_) => true, // File doesn't exist or can't be read = changed
        }
    }
}

/// Dependency graph for tracking file relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// Map from file path to its dependencies
    dependencies: HashMap<PathBuf, HashSet<PathBuf>>,
    /// Reverse map: dependents of each file
    dependents: HashMap<PathBuf, HashSet<PathBuf>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
        }
    }

    /// Add a dependency relationship: `from` depends on `to`
    pub fn add_dependency(&mut self, from: PathBuf, to: PathBuf) {
        // Add to forward map
        self.dependencies
            .entry(from.clone())
            .or_insert_with(HashSet::new)
            .insert(to.clone());

        // Add to reverse map
        self.dependents
            .entry(to)
            .or_insert_with(HashSet::new)
            .insert(from);
    }

    /// Get all dependencies of a file
    pub fn get_dependencies(&self, path: &Path) -> Option<&HashSet<PathBuf>> {
        self.dependencies.get(path)
    }

    /// Get all files that depend on this file
    pub fn get_dependents(&self, path: &Path) -> Option<&HashSet<PathBuf>> {
        self.dependents.get(path)
    }

    /// Get all files affected by a change (transitive closure)
    pub fn get_affected_files(&self, changed_file: &Path) -> HashSet<PathBuf> {
        let mut affected = HashSet::new();
        let mut to_process = vec![changed_file.to_path_buf()];

        while let Some(file) = to_process.pop() {
            if affected.contains(&file) {
                continue;
            }

            affected.insert(file.clone());

            // Add all dependents to process queue
            if let Some(dependents) = self.get_dependents(&file) {
                for dependent in dependents {
                    if !affected.contains(dependent) {
                        to_process.push(dependent.clone());
                    }
                }
            }
        }

        affected
    }

    /// Clear all dependencies
    pub fn clear(&mut self) {
        self.dependencies.clear();
        self.dependents.clear();
    }
}

/// Incremental build state manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalBuildState {
    /// File metadata from last build
    file_metadata: HashMap<PathBuf, FileMetadata>,
    /// Dependency graph
    graph: DependencyGraph,
    /// Last successful build timestamp
    last_build_time: SystemTime,
}

impl IncrementalBuildState {
    pub fn new() -> Self {
        Self {
            file_metadata: HashMap::new(),
            graph: DependencyGraph::new(),
            last_build_time: SystemTime::now(),
        }
    }

    /// Update file metadata
    pub fn update_file(&mut self, path: &Path) -> std::io::Result<()> {
        let metadata = FileMetadata::from_file(path)?;
        self.file_metadata.insert(path.to_path_buf(), metadata);
        Ok(())
    }

    /// Add a dependency relationship
    pub fn add_dependency(&mut self, from: PathBuf, to: PathBuf) {
        self.graph.add_dependency(from, to);
    }

    /// Get changed files since last build
    pub fn get_changed_files(&self) -> Vec<PathBuf> {
        self.file_metadata
            .iter()
            .filter(|(_, metadata)| metadata.has_changed())
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// Get all files that need to be rebuilt based on changes
    pub fn get_files_to_rebuild(&self) -> HashSet<PathBuf> {
        let changed_files = self.get_changed_files();
        let mut files_to_rebuild = HashSet::new();

        for changed_file in changed_files {
            // Get all affected files (transitive dependencies)
            let affected = self.graph.get_affected_files(&changed_file);
            files_to_rebuild.extend(affected);
        }

        files_to_rebuild
    }

    /// Mark build as complete
    pub fn mark_build_complete(&mut self) {
        self.last_build_time = SystemTime::now();
    }

    /// Check if any files have changed
    pub fn has_changes(&self) -> bool {
        !self.get_changed_files().is_empty()
    }

    /// Get dependency graph
    pub fn graph(&self) -> &DependencyGraph {
        &self.graph
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.file_metadata.clear();
        self.graph.clear();
    }
}

impl Default for IncrementalBuildState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();

        let main = PathBuf::from("main.js");
        let utils = PathBuf::from("utils.js");
        let helpers = PathBuf::from("helpers.js");

        // main.js depends on utils.js
        graph.add_dependency(main.clone(), utils.clone());
        // utils.js depends on helpers.js
        graph.add_dependency(utils.clone(), helpers.clone());

        // Check dependencies
        assert!(graph.get_dependencies(&main).unwrap().contains(&utils));
        assert!(graph.get_dependencies(&utils).unwrap().contains(&helpers));

        // Check dependents
        assert!(graph.get_dependents(&utils).unwrap().contains(&main));
        assert!(graph.get_dependents(&helpers).unwrap().contains(&utils));

        // Check affected files
        let affected = graph.get_affected_files(&helpers);
        assert_eq!(affected.len(), 3); // helpers, utils, main
        assert!(affected.contains(&helpers));
        assert!(affected.contains(&utils));
        assert!(affected.contains(&main));
    }

    #[test]
    fn test_incremental_build_state() {
        let mut state = IncrementalBuildState::new();

        let main = PathBuf::from("main.js");
        let utils = PathBuf::from("utils.js");

        state.add_dependency(main.clone(), utils.clone());

        // Check that dependency was added
        assert!(state.graph().get_dependencies(&main).is_some());
    }
}
