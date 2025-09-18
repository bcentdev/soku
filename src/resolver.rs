use crate::config::Config;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResolveRequest {
    pub specifier: String,
    pub importer: Option<PathBuf>,
    pub conditions: Vec<String>,
    pub kind: ResolveKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResolveKind {
    Import,
    DynamicImport,
    Require,
    Url,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResolveResult {
    pub path: PathBuf,
    pub external: bool,
    pub side_effects: bool,
    pub namespace: Option<String>,
}

pub struct Resolver {
    config: Config,
    package_cache: HashMap<PathBuf, PackageJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageJson {
    name: Option<String>,
    main: Option<String>,
    module: Option<String>,
    browser: Option<BrowserField>,
    exports: Option<ExportsField>,
    #[serde(rename = "type")]
    module_type: Option<String>,
    side_effects: Option<SideEffectsField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum BrowserField {
    String(String),
    Object(HashMap<String, Option<String>>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum ExportsField {
    String(String),
    Object(HashMap<String, ExportEntry>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum ExportEntry {
    String(String),
    Object(HashMap<String, String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum SideEffectsField {
    Boolean(bool),
    Array(Vec<String>),
}

impl Resolver {
    pub fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            package_cache: HashMap::new(),
        })
    }

    pub fn resolve(&mut self, request: &ResolveRequest) -> Result<ResolveResult> {
        // Handle external modules (node: protocol, bare imports)
        if request.specifier.starts_with("node:") {
            return Ok(ResolveResult {
                path: PathBuf::from(&request.specifier),
                external: true,
                side_effects: false,
                namespace: Some("node".to_string()),
            });
        }

        // Handle relative imports
        if request.specifier.starts_with("./") || request.specifier.starts_with("../") {
            return self.resolve_relative(request);
        }

        // Handle absolute imports
        if request.specifier.starts_with("/") {
            return self.resolve_absolute(request);
        }

        // Handle alias
        if let Some(alias_target) = self.config.resolve.alias.get(&request.specifier) {
            let aliased_request = ResolveRequest {
                specifier: alias_target.clone(),
                ..request.clone()
            };
            return self.resolve(&aliased_request);
        }

        // Handle bare imports (node_modules)
        self.resolve_bare_import(request)
    }

    fn resolve_relative(&self, request: &ResolveRequest) -> Result<ResolveResult> {
        let importer = request.importer.as_ref()
            .ok_or_else(|| anyhow!("Cannot resolve relative import without importer"))?;

        let base_dir = if importer.is_file() {
            importer.parent().unwrap()
        } else {
            importer
        };

        let candidate = base_dir.join(&request.specifier);
        self.resolve_file_or_directory(&candidate)
    }

    fn resolve_absolute(&self, request: &ResolveRequest) -> Result<ResolveResult> {
        let candidate = self.config.root.join(request.specifier.trim_start_matches('/'));
        self.resolve_file_or_directory(&candidate)
    }

    fn resolve_bare_import(&mut self, request: &ResolveRequest) -> Result<ResolveResult> {
        let start_dir = if let Some(importer) = &request.importer {
            if importer.is_file() {
                importer.parent().unwrap()
            } else {
                importer
            }
        } else {
            &self.config.root
        };

        // Walk up directories looking for node_modules
        let mut current = start_dir;
        loop {
            let node_modules = current.join("node_modules");
            if node_modules.exists() {
                let package_dir = node_modules.join(&request.specifier);
                if package_dir.exists() {
                    return self.resolve_package(&package_dir, request);
                }
            }

            match current.parent() {
                Some(parent) => current = parent,
                None => break,
            }
        }

        Err(anyhow!("Cannot resolve module: {}", request.specifier))
    }

    fn resolve_package(&mut self, package_dir: &Path, request: &ResolveRequest) -> Result<ResolveResult> {
        let package_json_path = package_dir.join("package.json");

        let package_json = if package_json_path.exists() {
            if !self.package_cache.contains_key(&package_json_path) {
                let content = std::fs::read_to_string(&package_json_path)?;
                let package_json: PackageJson = serde_json::from_str(&content)?;
                self.package_cache.insert(package_json_path.clone(), package_json.clone());
            }
            self.package_cache.get(&package_json_path).unwrap()
        } else {
            // No package.json, try index files
            return self.resolve_file_or_directory(&package_dir.join("index"));
        };

        // Handle exports field
        if let Some(exports) = &package_json.exports {
            if let Some(resolved) = self.resolve_exports(exports, ".", &request.conditions)? {
                let full_path = package_dir.join(resolved);
                return self.resolve_file_or_directory(&full_path);
            }
        }

        // Handle main fields
        for field in &self.config.resolve.main_fields {
            let entry = match field.as_str() {
                "module" => &package_json.module,
                "main" => &package_json.main,
                "browser" => {
                    if let Some(BrowserField::String(path)) = &package_json.browser {
                        Some(path)
                    } else {
                        None
                    }
                }
                _ => continue,
            };

            if let Some(entry_path) = entry {
                let full_path = package_dir.join(entry_path);
                if let Ok(result) = self.resolve_file_or_directory(&full_path) {
                    return Ok(result);
                }
            }
        }

        // Fallback to index
        self.resolve_file_or_directory(&package_dir.join("index"))
    }

    fn resolve_exports(&self, exports: &ExportsField, subpath: &str, conditions: &[String]) -> Result<Option<String>> {
        match exports {
            ExportsField::String(path) => Ok(Some(path.clone())),
            ExportsField::Object(map) => {
                // Try exact match first
                if let Some(entry) = map.get(subpath) {
                    return self.resolve_export_entry(entry, conditions);
                }

                // Try pattern matching
                for (pattern, entry) in map {
                    if pattern.ends_with("*") {
                        let prefix = &pattern[..pattern.len() - 1];
                        if subpath.starts_with(prefix) {
                            let replacement = &subpath[prefix.len()..];
                            if let Some(resolved) = self.resolve_export_entry(entry, conditions)? {
                                let final_path = resolved.replace("*", replacement);
                                return Ok(Some(final_path));
                            }
                        }
                    }
                }

                Ok(None)
            }
        }
    }

    fn resolve_export_entry(&self, entry: &ExportEntry, conditions: &[String]) -> Result<Option<String>> {
        match entry {
            ExportEntry::String(path) => Ok(Some(path.clone())),
            ExportEntry::Object(conditional) => {
                for condition in conditions {
                    if let Some(path) = conditional.get(condition) {
                        return Ok(Some(path.clone()));
                    }
                }
                Ok(conditional.get("default").cloned())
            }
        }
    }

    fn resolve_file_or_directory(&self, path: &Path) -> Result<ResolveResult> {
        // Try exact path first
        if path.is_file() {
            return Ok(ResolveResult {
                path: path.to_path_buf(),
                external: false,
                side_effects: true, // TODO: check package.json
                namespace: None,
            });
        }

        // Try with extensions
        for ext in &self.config.resolve.extensions {
            let with_ext = path.with_extension(&ext[1..]); // Remove the dot
            if with_ext.is_file() {
                return Ok(ResolveResult {
                    path: with_ext,
                    external: false,
                    side_effects: true,
                    namespace: None,
                });
            }
        }

        // Try as directory with index files
        if path.is_dir() {
            for ext in &self.config.resolve.extensions {
                let index_file = path.join("index").with_extension(&ext[1..]);
                if index_file.is_file() {
                    return Ok(ResolveResult {
                        path: index_file,
                        external: false,
                        side_effects: true,
                        namespace: None,
                    });
                }
            }
        }

        Err(anyhow!("Cannot resolve: {}", path.display()))
    }
}