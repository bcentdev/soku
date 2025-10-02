// Stats Plugin: Logs detailed build statistics

use crate::core::plugin::{Plugin, PluginContext};
use crate::core::models::{BuildResult, ModuleInfo};
use crate::utils::{Result, Logger};
use std::path::Path;
use std::time::Instant;

/// Plugin that tracks and logs detailed build statistics
///
/// # Example
/// ```
/// use ultra::plugins::StatsPlugin;
/// use ultra::core::services::UltraBuildService;
/// use std::sync::Arc;
///
/// let plugin = Arc::new(StatsPlugin::new(true)); // verbose = true
///
/// let service = UltraBuildService::new(/* ... */)
///     .with_plugin(plugin);
/// ```
pub struct StatsPlugin {
    verbose: bool,
    start_time: Option<Instant>,
}

impl StatsPlugin {
    /// Create a new stats plugin
    ///
    /// # Arguments
    /// * `verbose` - If true, logs detailed statistics
    pub fn new(verbose: bool) -> Self {
        Self {
            verbose,
            start_time: None,
        }
    }
}

impl Plugin for StatsPlugin {
    fn name(&self) -> &str {
        "stats-plugin"
    }

    fn on_build_start(&self, context: &PluginContext) -> Result<()> {
        if self.verbose {
            Logger::info("📊 Stats Plugin: Build started");
            Logger::info(&format!("  Root: {}", context.root.display()));
            Logger::info(&format!("  Output: {}", context.config.outdir.display()));
            Logger::info(&format!("  Tree shaking: {}", context.config.enable_tree_shaking));
            Logger::info(&format!("  Minification: {}", context.config.enable_minification));
            Logger::info(&format!("  Source maps: {}", context.config.enable_source_maps));
        }
        Ok(())
    }

    fn on_build_end(&self, _context: &PluginContext, result: &BuildResult) -> Result<()> {
        if result.success {
            Logger::info("📊 Stats Plugin: Build Statistics");
            Logger::info(&format!("  ✅ Success: {}", result.success));
            Logger::info(&format!("  ⚡ Build time: {:?}", result.build_time));
            Logger::info(&format!("  📦 JS modules: {}", result.js_modules_processed));
            Logger::info(&format!("  🎨 CSS files: {}", result.css_files_processed));
            Logger::info(&format!("  📂 Output files: {}", result.output_files.len()));

            if let Some(tree_stats) = &result.tree_shaking_stats {
                Logger::info("  🌳 Tree Shaking:");
                Logger::info(&format!("     Total modules: {}", tree_stats.total_modules));
                Logger::info(&format!("     Removed exports: {}", tree_stats.removed_exports));
                Logger::info(&format!("     Reduction: {:.1}%", tree_stats.reduction_percentage));
            }

            if self.verbose {
                Logger::info("  📄 Output Files:");
                for (i, file) in result.output_files.iter().enumerate() {
                    Logger::info(&format!("     {}. {} ({} bytes)",
                        i + 1,
                        file.path.file_name().unwrap_or_default().to_string_lossy(),
                        file.size
                    ));
                }
            }
        } else {
            Logger::warn("❌ Stats Plugin: Build failed");
            if !result.errors.is_empty() {
                Logger::warn(&format!("  Errors: {}", result.errors.len()));
            }
        }

        Ok(())
    }

    fn on_modules_resolved(
        &self,
        modules: &[ModuleInfo],
        _context: &PluginContext,
    ) -> Result<()> {
        if self.verbose {
            Logger::debug(&format!("📊 Stats Plugin: {} modules resolved", modules.len()));

            // Count by type
            let js_count = modules.iter().filter(|m| matches!(m.module_type, crate::core::models::ModuleType::JavaScript | crate::core::models::ModuleType::TypeScript)).count();
            let css_count = modules.iter().filter(|m| matches!(m.module_type, crate::core::models::ModuleType::Css)).count();

            Logger::debug(&format!("  JS/TS modules: {}", js_count));
            Logger::debug(&format!("  CSS modules: {}", css_count));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::core::models::BuildConfig;

    #[test]
    fn test_stats_plugin_name() {
        let plugin = StatsPlugin::new(false);
        assert_eq!(plugin.name(), "stats-plugin");
    }

    #[test]
    fn test_stats_plugin_verbose() {
        let plugin = StatsPlugin::new(true);
        assert!(plugin.verbose);
    }
}
