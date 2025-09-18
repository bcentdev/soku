use crate::cache::{Cache, CacheEntry, CacheMetadata, ImportInfo, ModuleType};
use crate::resolver::{ResolveRequest, ResolveResult, Resolver, ResolveKind};
use crate::transform_simple::SimpleTransformer;
use crate::css::{LightningCssProcessor, CssOptions};
use crate::memory::{
    ModuleStorage, MemoryManager, MemoryThresholds, StringInterner,
    MemoryCategory, CleanupRequest
};
use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use rayon::prelude::*;

#[derive(Debug, Clone)]
pub struct ModuleNode {
    pub id: String,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub imports: Vec<ImportInfo>,
    pub exports: Vec<String>,
    pub metadata: CacheMetadata,
    pub invalidated: bool,
    pub last_processed: std::time::Instant,
}

pub struct ModuleGraph {
    nodes: Arc<RwLock<ModuleStorage<ModuleNode>>>,
    cache: Arc<RwLock<Cache>>,
    resolver: Arc<RwLock<Resolver>>,
    entry_points: Arc<RwLock<Vec<Arc<str>>>>,
    transformer: Arc<SimpleTransformer>,
    css_processor: Arc<LightningCssProcessor>,
    string_interner: Arc<RwLock<StringInterner>>,
    memory_manager: Arc<MemoryManager>,
}

impl ModuleGraph {
    pub fn new(cache: Cache, resolver: Resolver) -> Self {
        let transformer = SimpleTransformer::new();
        let css_processor = LightningCssProcessor::new(CssOptions::default());

        // Memory optimizations for large projects
        let module_storage = ModuleStorage::new(
            10_000, // Max 10k modules in memory
            512 * 1024 * 1024, // 512MB memory limit
        );

        let memory_thresholds = MemoryThresholds {
            warning_bytes: 400 * 1024 * 1024,   // 400 MB warning
            critical_bytes: 800 * 1024 * 1024,  // 800 MB critical
            cleanup_interval: std::time::Duration::from_secs(30),
        };

        let memory_manager = MemoryManager::new(memory_thresholds);

        Self {
            nodes: Arc::new(RwLock::new(module_storage)),
            cache: Arc::new(RwLock::new(cache)),
            resolver: Arc::new(RwLock::new(resolver)),
            entry_points: Arc::new(RwLock::new(Vec::new())),
            transformer: Arc::new(transformer),
            css_processor: Arc::new(css_processor),
            string_interner: Arc::new(RwLock::new(StringInterner::new())),
            memory_manager: Arc::new(memory_manager),
        }
    }

    pub fn add_entry_point(&self, path: &Path) -> Result<Arc<str>> {
        let canonical = path.canonicalize()?;
        let id_string = canonical.to_string_lossy().to_string();

        // Intern the string for memory efficiency
        let id = {
            let mut interner = self.string_interner.write().unwrap();
            interner.intern(&id_string)
        };

        {
            let mut entries = self.entry_points.write().unwrap();
            if !entries.iter().any(|e| **e == *id) {
                entries.push(id.clone());
            }
        }

        self.build_module_tree(&id)?;
        Ok(id)
    }

    pub fn invalidate_module(&self, path: &Path) -> Result<Vec<String>> {
        let canonical = path.canonicalize()?;
        let id = canonical.to_string_lossy().to_string();

        let affected = self.get_affected_modules(&id)?;

        {
            let mut nodes = self.nodes.write().unwrap();
            let mut cache = self.cache.write().unwrap();

            for affected_id in &affected {
                if let Some(node) = nodes.get_mut(affected_id) {
                    node.invalidated = true;
                }
                cache.invalidate(affected_id);
            }
        }

        // Rebuild affected modules
        self.rebuild_modules(&affected)?;

        Ok(affected)
    }

    fn get_affected_modules(&self, changed_id: &str) -> Result<Vec<String>> {
        let nodes = self.nodes.read().unwrap();
        let mut affected = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(changed_id.to_string());
        affected.insert(changed_id.to_string());

        while let Some(current_id) = queue.pop_front() {
            if let Some(node) = nodes.get(&current_id) {
                for dependent_id in &node.dependents {
                    if !affected.contains(dependent_id) {
                        affected.insert(dependent_id.clone());
                        queue.push_back(dependent_id.clone());
                    }
                }
            }
        }

        Ok(affected.into_iter().collect())
    }

    fn rebuild_modules(&self, module_ids: &[String]) -> Result<()> {
        // Rebuild modules in parallel, respecting dependency order
        let dependency_order = self.topological_sort(module_ids)?;

        dependency_order.par_iter().try_for_each(|id| {
            self.process_module(id)
        })?;

        Ok(())
    }

    fn topological_sort(&self, module_ids: &[String]) -> Result<Vec<String>> {
        let nodes = self.nodes.read().unwrap();
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();

        fn visit(
            id: &str,
            nodes: &HashMap<String, ModuleNode>,
            visited: &mut HashSet<String>,
            temp_visited: &mut HashSet<String>,
            result: &mut Vec<String>,
        ) -> Result<()> {
            if temp_visited.contains(id) {
                return Err(anyhow!("Circular dependency detected"));
            }
            if visited.contains(id) {
                return Ok(());
            }

            temp_visited.insert(id.to_string());

            if let Some(node) = nodes.get(id) {
                for dep_id in &node.dependencies {
                    visit(dep_id, nodes, visited, temp_visited, result)?;
                }
            }

            temp_visited.remove(id);
            visited.insert(id.to_string());
            result.push(id.to_string());

            Ok(())
        }

        for id in module_ids {
            if !visited.contains(id) {
                visit(id, &nodes, &mut visited, &mut temp_visited, &mut result)?;
            }
        }

        Ok(result)
    }

    fn build_module_tree(&self, entry_id: &Arc<str>) -> Result<()> {
        let mut queue = VecDeque::new();
        let mut processed = HashSet::new();

        queue.push_back(entry_id.clone());

        while let Some(current_id) = queue.pop_front() {
            if processed.contains(&*current_id) {
                continue;
            }

            self.process_module(&current_id)?;
            processed.insert(current_id.to_string());

            // Add dependencies to queue
            {
                let mut nodes = self.nodes.write().unwrap();
                if let Some(node) = nodes.get(&current_id) {
                    for dep_id in &node.dependencies {
                        if !processed.contains(dep_id) {
                            // Intern dependency IDs
                            let interned_dep = {
                                let mut interner = self.string_interner.write().unwrap();
                                interner.intern(dep_id)
                            };
                            queue.push_back(interned_dep);
                        }
                    }
                }
            }
        }

        // Update memory usage
        let module_memory = {
            let nodes = self.nodes.read().unwrap();
            nodes.memory_usage()
        };
        self.memory_manager.update_memory_usage(MemoryCategory::Modules, module_memory);

        Ok(())
    }

    fn process_module(&self, module_id: &Arc<str>) -> Result<()> {
        let path = PathBuf::from(module_id);

        // Check cache first
        let cache_key = {
            let cache = self.cache.read().unwrap();
            cache.compute_file_key(&path, &[], &HashMap::new())?
        };

        {
            let mut cache = self.cache.write().unwrap();
            if let Some(cached) = cache.get(&cache_key) {
                // Use cached result
                let node = ModuleNode {
                    id: module_id.to_string(),
                    path: path.clone(),
                    dependencies: cached.dependencies.iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect(),
                    dependents: Vec::new(),
                    imports: cached.metadata.imports.clone(),
                    exports: cached.metadata.exports.clone(),
                    metadata: cached.metadata.clone(),
                    invalidated: false,
                    last_processed: std::time::Instant::now(),
                };

                let mut nodes = self.nodes.write().unwrap();
                nodes.insert(module_id.to_string(), node);
                return Ok(());
            }
        }

        // Process module from scratch
        let content = std::fs::read(&path)?;
        let module_type = self.detect_module_type(&path);

        let (imports, exports) = self.parse_module(&content, &module_type, &path)?;

        // Resolve dependencies
        let mut dependencies = Vec::new();
        let mut dependency_paths = Vec::new();

        for import in &imports {
            let request = ResolveRequest {
                specifier: import.specifier.clone(),
                importer: Some(path.clone()),
                conditions: vec!["import".to_string(), "browser".to_string()],
                kind: match import.kind {
                    crate::cache::ImportKind::Dynamic => ResolveKind::DynamicImport,
                    _ => ResolveKind::Import,
                },
            };

            let mut resolver = self.resolver.write().unwrap();
            match resolver.resolve(&request) {
                Ok(result) if !result.external => {
                    let dep_id = result.path.to_string_lossy().to_string();
                    dependencies.push(dep_id);
                    dependency_paths.push(result.path);
                }
                _ => {
                    // External or failed resolution - skip
                }
            }
        }

        let metadata = CacheMetadata {
            module_type,
            exports,
            imports: imports.clone(),
            has_side_effects: true, // TODO: detect side effects
            is_entry: {
                let entries = self.entry_points.read().unwrap();
                entries.contains(&module_id.to_string())
            },
        };

        // Cache the result
        {
            let content_hash = {
                let cache = self.cache.read().unwrap();
                cache.compute_content_hash(&content)
            };

            let cache_entry = CacheEntry {
                hash: content_hash,
                dependencies: dependency_paths.clone(),
                ast: None, // TODO: serialize AST
                transformed_code: None,
                source_map: None,
                metadata: metadata.clone(),
            };

            let mut cache = self.cache.write().unwrap();
            cache.set(cache_key, cache_entry)?;
        }

        // Create module node
        let node = ModuleNode {
            id: module_id.to_string(),
            path,
            dependencies: dependencies.clone(),
            dependents: Vec::new(),
            imports,
            exports: metadata.exports.clone(),
            metadata,
            invalidated: false,
            last_processed: std::time::Instant::now(),
        };

        // Update graph
        {
            let mut nodes = self.nodes.write().unwrap();

            // Add/update current node
            nodes.insert(module_id.to_string(), node);

            // Update dependents
            for dep_id in &dependencies {
                if let Some(dep_node) = nodes.get_mut(dep_id) {
                    if !dep_node.dependents.contains(&module_id.to_string()) {
                        dep_node.dependents.push(module_id.to_string());
                    }
                }
            }
        }

        Ok(())
    }

    fn detect_module_type(&self, path: &Path) -> ModuleType {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("ts") => ModuleType::TypeScript,
            Some("tsx") => ModuleType::Tsx,
            Some("jsx") => ModuleType::Jsx,
            Some("css") => ModuleType::Css,
            Some("json") => ModuleType::Json,
            Some("js") | Some("mjs") => ModuleType::JavaScript,
            _ => ModuleType::Asset,
        }
    }

    fn parse_module(&self, content: &[u8], module_type: &ModuleType, file_path: &Path) -> Result<(Vec<ImportInfo>, Vec<String>)> {
        match module_type {
            ModuleType::JavaScript | ModuleType::TypeScript | ModuleType::Jsx | ModuleType::Tsx => {
                // Use simple transformer for parsing
                let source = String::from_utf8_lossy(content);
                let transform_result = self.transformer.transform(&source, file_path)?;

                Ok((transform_result.imports, transform_result.exports))
            }
            ModuleType::Css => {
                // Use Lightning CSS for ultra-fast CSS parsing
                let source = String::from_utf8_lossy(content);
                let result = self.css_processor.transform(&source, file_path)?;

                Ok((result.imports, vec!["default".to_string()]))
            }
            ModuleType::Json => {
                // JSON modules export default
                Ok((Vec::new(), vec!["default".to_string()]))
            }
            ModuleType::Asset => {
                // Assets export URL
                Ok((Vec::new(), vec!["default".to_string()]))
            }
        }
    }

    fn parse_css_imports(&self, content: &[u8]) -> Result<(Vec<ImportInfo>, Vec<String>)> {
        let css_content = String::from_utf8_lossy(content);
        let mut imports = Vec::new();

        // Simple regex-based CSS import detection
        // In production, we'd use Lightning CSS for proper parsing
        for line in css_content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("@import") {
                // Extract import URL - simple version
                if let Some(start) = trimmed.find('"') {
                    if let Some(end) = trimmed[start + 1..].find('"') {
                        let url = &trimmed[start + 1..start + 1 + end];
                        imports.push(ImportInfo {
                            specifier: url.to_string(),
                            kind: ImportKind::Css,
                            source_location: None,
                        });
                    }
                } else if let Some(start) = trimmed.find('\'') {
                    if let Some(end) = trimmed[start + 1..].find('\'') {
                        let url = &trimmed[start + 1..start + 1 + end];
                        imports.push(ImportInfo {
                            specifier: url.to_string(),
                            kind: ImportKind::Css,
                            source_location: None,
                        });
                    }
                }
            }
        }

        // CSS modules don't export named exports, just default
        Ok((imports, vec!["default".to_string()]))
    }

    pub fn get_module(&self, id: &str) -> Option<ModuleNode> {
        let nodes = self.nodes.read().unwrap();
        nodes.get(id).cloned()
    }

    pub fn get_all_modules(&self) -> Vec<ModuleNode> {
        let nodes = self.nodes.read().unwrap();
        nodes.values().cloned().collect()
    }

    pub fn get_entry_points(&self) -> Vec<String> {
        let entries = self.entry_points.read().unwrap();
        entries.clone()
    }
}