use crate::core::models::BuildConfig;
use crate::utils::{Logger, UltraError, Result};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Configuration file format (ultra.config.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UltraConfig {
    /// Entry point file (e.g., "src/main.js")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,

    /// Output directory (default: "dist")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outdir: Option<String>,

    /// Enable/disable minification (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minify: Option<bool>,

    /// Enable/disable source maps (default: false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_maps: Option<bool>,

    /// Enable/disable tree shaking (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree_shaking: Option<bool>,

    /// Target ECMAScript version (default: "es2020")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// Enable/disable code splitting (default: false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_splitting: Option<bool>,

    /// Maximum chunk size in bytes (default: 250000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_chunk_size: Option<usize>,
}

impl Default for UltraConfig {
    fn default() -> Self {
        Self {
            entry: None,
            outdir: Some("dist".to_string()),
            minify: Some(true),
            source_maps: Some(false),
            tree_shaking: Some(true),
            target: Some("es2020".to_string()),
            code_splitting: Some(false),
            max_chunk_size: Some(250_000),
        }
    }
}

/// Config loader that supports config files with CLI override
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from file if it exists
    /// Searches for ultra.config.json in the project root
    pub fn load_from_file(root: &Path) -> Result<Option<UltraConfig>> {
        let config_path = root.join("ultra.config.json");

        if !config_path.exists() {
            Logger::debug("No ultra.config.json found, using defaults");
            return Ok(None);
        }

        Logger::debug(&format!("Loading config from {}", config_path.display()));

        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| UltraError::Io(e))?;

        let config: UltraConfig = serde_json::from_str(&content)
            .map_err(|e| UltraError::config(format!(
                "Failed to parse ultra.config.json: {}",
                e
            )))?;

        Logger::debug("✅ Config file loaded successfully");
        Ok(Some(config))
    }

    /// Merge file config with CLI arguments (CLI takes precedence)
    pub fn merge_with_cli(
        file_config: Option<UltraConfig>,
        root: PathBuf,
        outdir: Option<&str>,
        enable_tree_shaking: Option<bool>,
        enable_minification: Option<bool>,
        enable_source_maps: Option<bool>,
        enable_code_splitting: Option<bool>,
        max_chunk_size: Option<usize>,
    ) -> BuildConfig {
        let base = file_config.unwrap_or_default();

        // Determine output directory (CLI > config file > default)
        let outdir_str = outdir.unwrap_or_else(|| {
            base.outdir.as_deref().unwrap_or("dist")
        });

        // Resolve outdir relative to root if it's a relative path
        let resolved_outdir = if Path::new(outdir_str).is_absolute() {
            PathBuf::from(outdir_str)
        } else {
            root.join(outdir_str)
        };

        BuildConfig {
            root,
            outdir: resolved_outdir,
            enable_tree_shaking: enable_tree_shaking.unwrap_or_else(|| {
                base.tree_shaking.unwrap_or(true)
            }),
            enable_minification: enable_minification.unwrap_or_else(|| {
                base.minify.unwrap_or(true)
            }),
            enable_source_maps: enable_source_maps.unwrap_or_else(|| {
                base.source_maps.unwrap_or(false)
            }),
            enable_code_splitting: enable_code_splitting.unwrap_or_else(|| {
                base.code_splitting.unwrap_or(false)
            }),
            max_chunk_size: max_chunk_size.or(base.max_chunk_size).or(Some(250_000)),
        }
    }

    /// Generate example config file
    #[allow(dead_code)] // Future CLI command: ultra init
    pub fn generate_example() -> String {
        let example = UltraConfig::default();
        serde_json::to_string_pretty(&example).unwrap_or_else(|_| {
            r#"{
  "entry": "src/main.js",
  "outdir": "dist",
  "minify": true,
  "sourceMaps": true,
  "treeShaking": true,
  "target": "es2020",
  "codeSplitting": false,
  "maxChunkSize": 250000
}"#.to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_from_file_not_exists() {
        let temp_dir = tempfile::tempdir().unwrap();
        let result = ConfigLoader::load_from_file(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_load_from_file_valid() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"{{"outdir": "build", "minify": false}}"#).unwrap();

        let parent = temp_file.path().parent().unwrap();
        let file_name = temp_file.path().file_name().unwrap();

        // Rename to ultra.config.json
        let config_path = parent.join("ultra.config.json");
        std::fs::copy(temp_file.path(), &config_path).unwrap();

        let result = ConfigLoader::load_from_file(parent).unwrap();
        assert!(result.is_some());

        let config = result.unwrap();
        assert_eq!(config.outdir, Some("build".to_string()));
        assert_eq!(config.minify, Some(false));
    }

    #[test]
    fn test_merge_with_cli_override() {
        let file_config = UltraConfig {
            outdir: Some("build".to_string()),
            minify: Some(false),
            ..Default::default()
        };

        let merged = ConfigLoader::merge_with_cli(
            Some(file_config),
            PathBuf::from("."),
            Some("dist-override"), // CLI override
            None,
            Some(true), // CLI override
            None,
            None,
            None,
        );

        assert_eq!(merged.outdir, PathBuf::from("dist-override"));
        assert_eq!(merged.enable_minification, true); // CLI wins
    }

    #[test]
    fn test_generate_example() {
        let example = ConfigLoader::generate_example();
        assert!(example.contains("outdir"));
        assert!(example.contains("minify"));
    }
}
