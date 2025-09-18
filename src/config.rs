use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub root: PathBuf,
    pub cache_dir: PathBuf,
    pub entry_points: Vec<String>,
    pub resolve: ResolveConfig,
    pub build: BuildConfig,
    pub server: ServerConfig,
    pub define: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveConfig {
    pub alias: HashMap<String, String>,
    pub extensions: Vec<String>,
    pub main_fields: Vec<String>,
    pub conditions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub target: String,
    pub minify: bool,
    pub sourcemap: bool,
    pub chunk_size_warning_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub open: bool,
    pub cors: bool,
}

impl Config {
    pub fn load(root: &str) -> Result<Self> {
        let root = PathBuf::from(root).canonicalize()?;
        let config_path = root.join("ultra.config.json");

        let config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            serde_json::from_str(&content)?
        } else {
            Self::default_for_root(&root)
        };

        Ok(config)
    }

    fn default_for_root(root: &Path) -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| root.join(".cache"))
            .join("ultra-bundler");

        Self {
            root: root.to_path_buf(),
            cache_dir,
            entry_points: vec!["index.html".to_string()],
            resolve: ResolveConfig {
                alias: HashMap::new(),
                extensions: vec![
                    ".ts".to_string(),
                    ".tsx".to_string(),
                    ".js".to_string(),
                    ".jsx".to_string(),
                    ".mjs".to_string(),
                    ".json".to_string(),
                ],
                main_fields: vec![
                    "browser".to_string(),
                    "module".to_string(),
                    "jsnext:main".to_string(),
                    "main".to_string(),
                ],
                conditions: vec![
                    "import".to_string(),
                    "module".to_string(),
                    "browser".to_string(),
                    "default".to_string(),
                ],
            },
            build: BuildConfig {
                target: "es2020".to_string(),
                minify: true,
                sourcemap: true,
                chunk_size_warning_limit: 500 * 1024, // 500 KB
            },
            server: ServerConfig {
                host: "localhost".to_string(),
                open: false,
                cors: true,
            },
            define: HashMap::new(),
        }
    }
}