use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use crate::utils::{Result, UltraError};

/// Package.json structure for parsing npm packages
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageJson {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub main: Option<String>,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub browser: Option<BrowserField>,
    #[serde(default)]
    pub exports: Option<serde_json::Value>,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub dev_dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BrowserField {
    String(String),
    Object(HashMap<String, serde_json::Value>),
}

/// Export conditions for conditional exports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Part of package.json exports API
pub enum ExportCondition {
    Import,    // ESM import
    #[allow(dead_code)]
    Require,   // CommonJS require
    Browser,   // Browser environment
    Node,      // Node.js environment
    Default,   // Fallback
}

impl ExportCondition {
    fn as_str(&self) -> &str {
        match self {
            ExportCondition::Import => "import",
            ExportCondition::Require => "require",
            ExportCondition::Browser => "browser",
            ExportCondition::Node => "node",
            ExportCondition::Default => "default",
        }
    }

    /// Get priority order for resolution (higher priority first)
    fn priority_order() -> Vec<Self> {
        vec![
            ExportCondition::Import,   // Prefer ESM
            ExportCondition::Browser,  // Browser context
            ExportCondition::Node,     // Node.js context
            ExportCondition::Default,  // Fallback
        ]
    }
}

/// Node.js-style module resolution implementation
/// Now thread-safe for parallel resolution
pub struct NodeModuleResolver {
    /// Cache of package.json files (thread-safe)
    package_cache: Arc<DashMap<PathBuf, PackageJson>>,
}

impl NodeModuleResolver {
    pub fn new() -> Self {
        Self {
            package_cache: Arc::new(DashMap::new()),
        }
    }

    /// Clone the resolver for use in parallel contexts
    /// This is cheap as it only clones the Arc
    #[allow(dead_code)] // Part of public API for parallel resolution
    pub fn clone_ref(&self) -> Self {
        Self {
            package_cache: Arc::clone(&self.package_cache),
        }
    }

    /// Resolve a module import following Node.js resolution algorithm
    /// Now thread-safe - can be called from multiple threads
    pub async fn resolve(
        &self,
        import_path: &str,
        from_file: &Path,
        project_root: &Path,
    ) -> Option<PathBuf> {
        // Handle relative imports
        if import_path.starts_with("./") || import_path.starts_with("../") {
            return self.resolve_relative(import_path, from_file).await;
        }

        // Handle absolute imports starting with /
        if import_path.starts_with('/') {
            let resolved = project_root.join(&import_path[1..]);
            return self.resolve_file_or_directory(&resolved).await;
        }

        // Handle node_modules imports
        self.resolve_node_module(import_path, from_file, project_root).await
    }

    /// Resolve relative imports
    async fn resolve_relative(&self, import_path: &str, from_file: &Path) -> Option<PathBuf> {
        let current_dir = from_file.parent()?;
        let resolved = current_dir.join(import_path);
        self.resolve_file_or_directory(&resolved).await
    }

    /// Resolve a node_modules package
    async fn resolve_node_module(
        &self,
        package_name: &str,
        from_file: &Path,
        project_root: &Path,
    ) -> Option<PathBuf> {
        // Parse package name and subpath
        let (pkg_name, subpath) = self.parse_package_specifier(package_name);


        // Walk up directory tree looking for node_modules
        let mut current_dir = from_file.parent()?;

        loop {
            let node_modules = current_dir.join("node_modules");

            if node_modules.exists() && node_modules.is_dir() {
                let package_dir = node_modules.join(&pkg_name);

                if package_dir.exists() && package_dir.is_dir() {
                    // Found the package, resolve the entry point
                    if let Some(entry) = self.resolve_package_entry(&package_dir, subpath.clone()).await {
                        return Some(entry);
                    }
                }
            }

            // Move up one directory
            if current_dir == project_root || current_dir.parent().is_none() {
                break;
            }
            current_dir = current_dir.parent()?;
        }

        None
    }

    /// Parse package specifier into package name and subpath
    fn parse_package_specifier(&self, specifier: &str) -> (String, Option<String>) {
        // Handle scoped packages like @babel/core
        if specifier.starts_with('@') {
            if let Some(slash_pos) = specifier[1..].find('/') {
                let second_slash = specifier[slash_pos + 2..].find('/');
                if let Some(pos) = second_slash {
                    let pkg_name = specifier[..slash_pos + 2 + pos].to_string();
                    let subpath = specifier[slash_pos + 2 + pos + 1..].to_string();
                    return (pkg_name, Some(subpath));
                } else {
                    return (specifier.to_string(), None);
                }
            }
        } else {
            // Regular packages
            if let Some(slash_pos) = specifier.find('/') {
                let pkg_name = specifier[..slash_pos].to_string();
                let subpath = specifier[slash_pos + 1..].to_string();
                return (pkg_name, Some(subpath));
            }
        }

        (specifier.to_string(), None)
    }

    /// Resolve package entry point
    async fn resolve_package_entry(
        &self,
        package_dir: &Path,
        subpath: Option<String>,
    ) -> Option<PathBuf> {
        // If subpath is specified, try to resolve it directly
        if let Some(subpath) = subpath {
            let full_path = package_dir.join(&subpath);
            return self.resolve_file_or_directory(&full_path).await;
        }

        // Read package.json
        let package_json_path = package_dir.join("package.json");
        if !package_json_path.exists() {
            // No package.json, try index.js
            return self.resolve_file_or_directory(&package_dir.join("index")).await;
        }

        // Parse package.json
        let package_json = self.read_package_json(&package_json_path).await?;

        // Try different entry points in order of preference (Node.js 22/24 compatible)
        // 1. exports field (if present) - HIGHEST PRIORITY for modern packages
        if let Some(exports) = &package_json.exports {
            if let Some(resolved) = self.resolve_exports(exports, subpath.as_deref(), package_dir).await {
                return Some(resolved);
            }
        }

        // 2. module field (ES6 modules)
        if let Some(module) = &package_json.module {
            let entry = package_dir.join(module);
            if let Some(resolved) = self.resolve_file_or_directory(&entry).await {
                return Some(resolved);
            }
        }

        // 3. browser field (for browser builds)
        if let Some(browser) = &package_json.browser {
            match browser {
                BrowserField::String(path) => {
                    let entry = package_dir.join(path);
                    if let Some(resolved) = self.resolve_file_or_directory(&entry).await {
                        return Some(resolved);
                    }
                }
                BrowserField::Object(replacements) => {
                    // Handle browser field replacements
                    // Check if there's a main replacement
                    if let Some(main_value) = replacements.get(".") {
                        if let Some(path) = main_value.as_str() {
                            let entry = package_dir.join(path);
                            if let Some(resolved) = self.resolve_file_or_directory(&entry).await {
                                return Some(resolved);
                            }
                        }
                    }
                }
            }
        }

        // 4. main field (CommonJS default)
        if let Some(main) = &package_json.main {
            let entry = package_dir.join(main);
            if let Some(resolved) = self.resolve_file_or_directory(&entry).await {
                return Some(resolved);
            }
        }

        // 5. Default to index.js
        self.resolve_file_or_directory(&package_dir.join("index")).await
    }

    /// Try to resolve as file or directory
    async fn resolve_file_or_directory(&self, path: &Path) -> Option<PathBuf> {
        // Try as file first
        if let Some(file) = self.resolve_as_file(path).await {
            return Some(file);
        }

        // Try as directory with package.json
        if !path.exists() || !path.is_dir() {
            return None;
        }

        // Check for package.json in directory
        let package_json = path.join("package.json");
        if package_json.exists() {
            if let Some(pkg) = self.read_package_json(&package_json).await {
                // Use main field if present
                if let Some(main) = &pkg.main {
                    let entry = path.join(main);
                    // Just check as file, avoid recursion
                    if let Some(resolved) = self.resolve_as_file(&entry).await {
                        return Some(resolved);
                    }
                }
            }
        }

        // Try index files
        for index in &["index.js", "index.jsx", "index.ts", "index.tsx", "index.json"] {
            let index_file = path.join(index);
            if index_file.exists() && index_file.is_file() {
                return Some(index_file);
            }
        }

        None
    }

    /// Try to resolve as a file with various extensions
    async fn resolve_as_file(&self, path: &Path) -> Option<PathBuf> {
        // Check if file exists as-is
        if path.exists() && path.is_file() {
            return Some(path.to_path_buf());
        }

        // Try with different extensions
        for ext in &[".js", ".jsx", ".ts", ".tsx", ".json", ".mjs", ".cjs"] {
            let with_ext = path.with_extension(&ext[1..]);
            if with_ext.exists() && with_ext.is_file() {
                return Some(with_ext);
            }
        }

        None
    }

    /// Resolve exports field for a given subpath
    /// Implements Node.js exports field resolution algorithm
    async fn resolve_exports(
        &self,
        exports: &serde_json::Value,
        subpath: Option<&str>,
        package_dir: &Path,
    ) -> Option<PathBuf> {
        use serde_json::Value;

        let target_subpath = subpath.unwrap_or(".");

        match exports {
            // String export: "exports": "./index.js"
            Value::String(path) if target_subpath == "." => {
                let resolved = package_dir.join(path);
                return self.resolve_file_or_directory(&resolved).await;
            }

            // Object export
            Value::Object(map) => {
                // Check if it's a conditional export or subpath export
                let has_conditions = map.keys().any(|k| {
                    matches!(k.as_str(), "import" | "require" | "browser" | "node" | "default")
                });

                if has_conditions && target_subpath == "." {
                    // Conditional exports: { "import": "./esm/index.js", "require": "./cjs/index.js" }
                    return self.resolve_conditional_exports(map, package_dir).await;
                }

                // Subpath exports: { "./utils": "./src/utils.js", "./package.json": "./package.json" }
                if let Some(export_value) = map.get(target_subpath) {
                    return self.resolve_export_value(export_value, package_dir).await;
                }

                // Try pattern matching: { "./*": "./dist/*.js" }
                for (pattern, export_value) in map {
                    if let Some(resolved) = self.match_subpath_pattern(pattern, target_subpath, export_value, package_dir).await {
                        return Some(resolved);
                    }
                }
            }

            _ => {}
        }

        None
    }

    /// Resolve conditional exports based on conditions
    async fn resolve_conditional_exports(
        &self,
        conditions: &serde_json::Map<String, serde_json::Value>,
        package_dir: &Path,
    ) -> Option<PathBuf> {
        // Try conditions in priority order
        for condition in ExportCondition::priority_order() {
            if let Some(export_value) = conditions.get(condition.as_str()) {
                if let Some(resolved) = self.resolve_export_value(export_value, package_dir).await {
                    return Some(resolved);
                }
            }
        }

        None
    }

    /// Resolve an export value (string or nested object)
    fn resolve_export_value<'a>(
        &'a self,
        value: &'a serde_json::Value,
        package_dir: &'a Path,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<PathBuf>> + Send + 'a>> {
        use serde_json::Value;

        Box::pin(async move {
            match value {
                Value::String(path) => {
                    let resolved = package_dir.join(path);
                    self.resolve_file_or_directory(&resolved).await
                }
                Value::Object(map) => {
                    // Nested conditional exports
                    self.resolve_conditional_exports(map, package_dir).await
                }
                _ => None,
            }
        })
    }

    /// Match and resolve subpath patterns like "./*": "./dist/*.js"
    async fn match_subpath_pattern(
        &self,
        pattern: &str,
        subpath: &str,
        export_value: &serde_json::Value,
        package_dir: &Path,
    ) -> Option<PathBuf> {
        // Only handle patterns with single *
        if !pattern.contains('*') {
            return None;
        }

        // Split pattern by *
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() != 2 {
            return None; // Only support single * patterns
        }

        let (prefix, suffix) = (parts[0], parts[1]);

        // Check if subpath matches pattern
        if subpath.starts_with(prefix) && subpath.ends_with(suffix) {
            let matched = &subpath[prefix.len()..subpath.len() - suffix.len()];

            // Replace * in export value with matched part
            if let serde_json::Value::String(export_pattern) = export_value {
                let resolved_path = export_pattern.replace('*', matched);
                let full_path = package_dir.join(&resolved_path);
                return self.resolve_file_or_directory(&full_path).await;
            }
        }

        None
    }

    /// Read and cache package.json
    async fn read_package_json(&self, path: &Path) -> Option<PackageJson> {
        // Check cache first
        if let Some(cached) = self.package_cache.get(path) {
            return Some(cached.clone());
        }

        // Read and parse package.json
        let content = tokio::fs::read_to_string(path).await.ok()?;
        let package: PackageJson = serde_json::from_str(&content).ok()?;

        // Cache it
        self.package_cache.insert(path.to_path_buf(), package.clone());

        Some(package)
    }

    /// Check if a path is a node_modules import
    #[allow(dead_code)]
    pub fn is_node_module(import_path: &str) -> bool {
        !import_path.starts_with("./")
            && !import_path.starts_with("../")
            && !import_path.starts_with('/')
    }

    /// Get list of installed packages from node_modules
    #[allow(dead_code)]
    pub async fn get_installed_packages(project_root: &Path) -> Result<Vec<String>> {
        let node_modules = project_root.join("node_modules");

        if !node_modules.exists() {
            return Ok(Vec::new());
        }

        let mut packages = Vec::new();
        let mut entries = tokio::fs::read_dir(&node_modules).await
            .map_err(|e| UltraError::Io(e))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| UltraError::Io(e))? {

            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Handle scoped packages
                    if name.starts_with('@') {
                        // Read subdirectories for scoped packages
                        let mut scoped_entries = tokio::fs::read_dir(&path).await
                            .map_err(|e| UltraError::Io(e))?;

                        while let Some(scoped_entry) = scoped_entries.next_entry().await
                            .map_err(|e| UltraError::Io(e))? {

                            if scoped_entry.path().is_dir() {
                                if let Some(pkg_name) = scoped_entry.path().file_name()
                                    .and_then(|n| n.to_str()) {
                                    packages.push(format!("{}/{}", name, pkg_name));
                                }
                            }
                        }
                    } else if !name.starts_with('.') {
                        packages.push(name.to_string());
                    }
                }
            }
        }

        Ok(packages)
    }
}

impl Default for NodeModuleResolver {
    fn default() -> Self {
        Self::new()
    }
}