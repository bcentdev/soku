// Banner Plugin: Adds a comment banner to the top of output files

use crate::core::plugin::{Plugin, PluginContext};
use crate::core::models::BuildResult;
use crate::utils::{Result, Logger};
use std::path::Path;

/// Plugin that adds a custom banner comment to the top of bundled files
///
/// # Example
/// ```
/// use ultra::plugins::BannerPlugin;
/// use ultra::core::services::UltraBuildService;
/// use std::sync::Arc;
///
/// let banner = "/*! MyApp v1.0.0 | (c) 2025 */";
/// let plugin = Arc::new(BannerPlugin::new(banner));
///
/// let service = UltraBuildService::new(/* ... */)
///     .with_plugin(plugin);
/// ```
pub struct BannerPlugin {
    banner: String,
}

impl BannerPlugin {
    /// Create a new banner plugin with the specified banner text
    pub fn new(banner: impl Into<String>) -> Self {
        Self {
            banner: banner.into(),
        }
    }
}

impl Plugin for BannerPlugin {
    fn name(&self) -> &str {
        "banner-plugin"
    }

    fn on_build_start(&self, _context: &PluginContext) -> Result<()> {
        Logger::debug(&format!("Banner plugin initialized with banner: {}", self.banner));
        Ok(())
    }

    fn on_build_end(&self, _context: &PluginContext, result: &BuildResult) -> Result<()> {
        if result.success {
            Logger::info(&format!("✨ Banner plugin: Build completed successfully with {} output files", result.output_files.len()));
        }
        Ok(())
    }

    fn transform(
        &self,
        code: &str,
        file_path: &Path,
        _context: &PluginContext,
    ) -> Result<Option<String>> {
        // Only add banner to .js files
        if file_path.extension().and_then(|s| s.to_str()) == Some("js") {
            let transformed = format!("{}\n\n{}", self.banner, code);
            Ok(Some(transformed))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::core::models::BuildConfig;

    #[test]
    fn test_banner_plugin_transform() {
        let plugin = BannerPlugin::new("/* Banner */");
        let context = PluginContext {
            root: PathBuf::from("/tmp"),
            config: BuildConfig {
                root: PathBuf::from("/tmp"),
                outdir: PathBuf::from("/tmp/dist"),
                enable_tree_shaking: false,
                enable_minification: false,
                enable_source_maps: false,
                enable_code_splitting: false,
                max_chunk_size: None,
            },
        };

        let result = plugin
            .transform("console.log('test');", Path::new("test.js"), &context)
            .unwrap();

        assert!(result.is_some());
        assert!(result.unwrap().contains("/* Banner */"));
    }

    #[test]
    fn test_banner_plugin_no_transform_css() {
        let plugin = BannerPlugin::new("/* Banner */");
        let context = PluginContext {
            root: PathBuf::from("/tmp"),
            config: BuildConfig {
                root: PathBuf::from("/tmp"),
                outdir: PathBuf::from("/tmp/dist"),
                enable_tree_shaking: false,
                enable_minification: false,
                enable_source_maps: false,
                enable_code_splitting: false,
                max_chunk_size: None,
            },
        };

        let result = plugin
            .transform("body { color: red; }", Path::new("test.css"), &context)
            .unwrap();

        assert!(result.is_none());
    }
}
