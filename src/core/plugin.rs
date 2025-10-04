// Plugin system for Ultra Bundler
// Enables extensible architecture with custom transformations and hooks

use crate::core::models::{BuildConfig, BuildResult};
use crate::utils::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Context provided to plugins during execution
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// Project root directory
    pub root: PathBuf,
    /// Current build configuration
    pub config: BuildConfig,
}

impl PluginContext {
    pub fn new(root: PathBuf, config: BuildConfig) -> Self {
        Self { root, config }
    }
}

/// Main plugin trait that all plugins must implement
///
/// Plugins can hook into various stages of the build process:
/// - Build lifecycle (start/end)
/// - File transformation (transform)
/// - Module resolution (resolve)
pub trait Plugin: Send + Sync {
    /// Unique name for this plugin
    fn name(&self) -> &str;

    /// Called at the start of a build
    ///
    /// Use this to initialize resources, validate configuration, etc.
    fn on_build_start(&self, _context: &PluginContext) -> Result<()> {
        Ok(())
    }

    /// Called at the end of a build (success or failure)
    ///
    /// Use this to cleanup resources, log statistics, etc.
    fn on_build_end(&self, _context: &PluginContext, _result: &BuildResult) -> Result<()> {
        Ok(())
    }

    /// Transform file content
    ///
    /// Return Some(transformed_code) to replace the content,
    /// or None to leave it unchanged.
    ///
    /// # Arguments
    /// * `code` - Original file content
    /// * `file_path` - Path to the file being transformed
    /// * `context` - Plugin context with build information
    fn transform(
        &self,
        _code: &str,
        _file_path: &Path,
        _context: &PluginContext,
    ) -> Result<Option<String>> {
        Ok(None)
    }

    /// Resolve module imports
    ///
    /// Return Some(resolved_path) to override resolution,
    /// or None to use default resolution.
    ///
    /// # Arguments
    /// * `import` - Import specifier (e.g., "./utils", "lodash")
    /// * `importer` - Path to file doing the import
    /// * `context` - Plugin context with build information
    fn resolve(
        &self,
        _import: &str,
        _importer: &Path,
        _context: &PluginContext,
    ) -> Result<Option<PathBuf>> {
        Ok(None)
    }

    /// Called after modules are resolved but before processing
    ///
    /// Can be used to inject virtual modules or modify the module graph
    fn on_modules_resolved(
        &self,
        _modules: &[crate::core::models::ModuleInfo],
        _context: &PluginContext,
    ) -> Result<()> {
        Ok(())
    }
}

/// Manages plugin registration and execution
pub struct PluginManager {
    plugins: Vec<Arc<dyn Plugin>>,
}

impl PluginManager {
    /// Create a new empty plugin manager
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: Arc<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    /// Get number of registered plugins
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Execute on_build_start hook for all plugins
    pub fn on_build_start(&self, context: &PluginContext) -> Result<()> {
        for plugin in &self.plugins {
            plugin.on_build_start(context)?;
        }
        Ok(())
    }

    /// Execute on_build_end hook for all plugins
    pub fn on_build_end(&self, context: &PluginContext, result: &BuildResult) -> Result<()> {
        for plugin in &self.plugins {
            plugin.on_build_end(context, result)?;
        }
        Ok(())
    }

    /// Execute transform hooks for all plugins
    ///
    /// Plugins are executed in registration order.
    /// Each plugin receives the output of the previous plugin.
    pub fn transform(
        &self,
        mut code: String,
        file_path: &Path,
        context: &PluginContext,
    ) -> Result<String> {
        for plugin in &self.plugins {
            if let Some(transformed) = plugin.transform(&code, file_path, context)? {
                code = transformed;
            }
        }
        Ok(code)
    }

    /// Execute resolve hooks for all plugins
    ///
    /// Returns the first non-None result, or None if no plugin resolved it.
    pub fn resolve(
        &self,
        import: &str,
        importer: &Path,
        context: &PluginContext,
    ) -> Result<Option<PathBuf>> {
        for plugin in &self.plugins {
            if let Some(resolved) = plugin.resolve(import, importer, context)? {
                return Ok(Some(resolved));
            }
        }
        Ok(None)
    }

    /// Execute on_modules_resolved hook for all plugins
    pub fn on_modules_resolved(
        &self,
        modules: &[crate::core::models::ModuleInfo],
        context: &PluginContext,
    ) -> Result<()> {
        for plugin in &self.plugins {
            plugin.on_modules_resolved(modules, context)?;
        }
        Ok(())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        name: String,
    }

    impl TestPlugin {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            &self.name
        }

        fn transform(
            &self,
            code: &str,
            _file_path: &Path,
            _context: &PluginContext,
        ) -> Result<Option<String>> {
            // Uppercase transformation for testing
            Ok(Some(code.to_uppercase()))
        }
    }

    #[test]
    fn test_plugin_manager_registration() {
        let mut manager = PluginManager::new();
        assert_eq!(manager.plugin_count(), 0);

        manager.register(Arc::new(TestPlugin::new("test1")));
        assert_eq!(manager.plugin_count(), 1);

        manager.register(Arc::new(TestPlugin::new("test2")));
        assert_eq!(manager.plugin_count(), 2);
    }

    #[test]
    fn test_transform_chain() {
        let mut manager = PluginManager::new();
        manager.register(Arc::new(TestPlugin::new("test")));

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
                enable_css_modules: false,
            },
        };

        let result = manager
            .transform("hello world".to_string(), Path::new("test.js"), &context)
            .unwrap();

        assert_eq!(result, "HELLO WORLD");
    }
}
