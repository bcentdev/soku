use std::path::{Path, PathBuf};
use std::collections::HashMap;
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

/// Node.js-style module resolution implementation
pub struct NodeModuleResolver {
    /// Cache of package.json files
    package_cache: HashMap<PathBuf, PackageJson>,
}

impl NodeModuleResolver {
    pub fn new() -> Self {
        Self {
            package_cache: HashMap::new(),
        }
    }

    /// Resolve a module import following Node.js resolution algorithm
    pub async fn resolve(
        &mut self,
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
    async fn resolve_relative(&mut self, import_path: &str, from_file: &Path) -> Option<PathBuf> {
        let current_dir = from_file.parent()?;
        let resolved = current_dir.join(import_path);
        self.resolve_file_or_directory(&resolved).await
    }

    /// Resolve a node_modules package
    async fn resolve_node_module(
        &mut self,
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
        &mut self,
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

        // Try different entry points in order of preference
        // 1. exports field (if present)
        if let Some(_exports) = &package_json.exports {
            // TODO: Implement exports field resolution (complex)
            // For now, fall through to other methods
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
                BrowserField::Object(_) => {
                    // TODO: Handle browser field replacements
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
    async fn resolve_file_or_directory(&mut self, path: &Path) -> Option<PathBuf> {
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


    /// Read and cache package.json
    async fn read_package_json(&mut self, path: &Path) -> Option<PackageJson> {
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
    pub fn is_node_module(import_path: &str) -> bool {
        !import_path.starts_with("./")
            && !import_path.starts_with("../")
            && !import_path.starts_with('/')
    }

    /// Get list of installed packages from node_modules
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