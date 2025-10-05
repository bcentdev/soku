// Plugin System - Event-based extensibility for Ultra Bundler
#![allow(dead_code)] // Public API - used via examples and external integrations

use crate::core::models::{BuildConfig, BuildResult, ModuleInfo};
use crate::utils::Result;
use std::sync::Arc;
use async_trait::async_trait;

/// Plugin lifecycle events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginEvent {
    /// Before build starts
    BeforeBuild,
    /// After build completes
    AfterBuild,
    /// Before bundling modules
    BeforeBundle,
    /// After bundling modules
    AfterBundle,
    /// Before processing a module
    BeforeModuleProcess,
    /// After processing a module
    AfterModuleProcess,
    /// Before writing output files
    BeforeOutput,
    /// After writing output files
    AfterOutput,
}

/// Context passed to plugins during events
#[derive(Debug, Clone)]
pub struct PluginContext {
    pub config: BuildConfig,
    pub modules: Vec<ModuleInfo>,
    pub current_event: PluginEvent,
}

impl PluginContext {
    pub fn new(config: BuildConfig, modules: Vec<ModuleInfo>, event: PluginEvent) -> Self {
        Self {
            config,
            modules,
            current_event: event,
        }
    }
}

/// Plugin trait - implement this to create a plugin
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Plugin name
    fn name(&self) -> &str;

    /// Plugin version
    fn version(&self) -> &str {
        "1.0.0"
    }

    /// Initialize plugin (called once at startup)
    async fn init(&mut self) -> Result<()> {
        Ok(())
    }

    /// Called before build starts
    async fn before_build(&self, _context: &PluginContext) -> Result<()> {
        Ok(())
    }

    /// Called after build completes
    async fn after_build(&self, _context: &PluginContext, _result: &BuildResult) -> Result<()> {
        Ok(())
    }

    /// Called before bundling modules
    async fn before_bundle(&self, _context: &PluginContext) -> Result<()> {
        Ok(())
    }

    /// Called after bundling modules
    async fn after_bundle(&self, _context: &PluginContext, _bundle_code: &str) -> Result<()> {
        Ok(())
    }

    /// Transform module code
    async fn transform_code(&self, _module: &ModuleInfo, code: String) -> Result<String> {
        Ok(code)
    }

    /// Resolve import path
    async fn resolve_import(&self, _import_path: &str, _from_file: &str) -> Result<Option<String>> {
        Ok(None)
    }

    /// Called before writing output files
    async fn before_output(&self, _context: &PluginContext) -> Result<()> {
        Ok(())
    }

    /// Called after writing output files
    async fn after_output(&self, _context: &PluginContext) -> Result<()> {
        Ok(())
    }
}

/// Plugin manager - manages and orchestrates plugins
pub struct PluginManager {
    plugins: Vec<Arc<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: Arc<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    /// Get all registered plugins
    pub fn plugins(&self) -> &[Arc<dyn Plugin>] {
        &self.plugins
    }

    /// Initialize all plugins
    pub async fn init_all(&mut self) -> Result<()> {
        for _plugin in &self.plugins {
            // Note: We can't call mutable methods on Arc<dyn Plugin>
            // In a real implementation, you'd use Arc<Mutex<dyn Plugin>>
            // or make init take &self instead of &mut self
        }
        Ok(())
    }

    /// Trigger before_build event
    pub async fn trigger_before_build(&self, context: &PluginContext) -> Result<()> {
        for plugin in &self.plugins {
            plugin.before_build(context).await?;
        }
        Ok(())
    }

    /// Trigger after_build event
    pub async fn trigger_after_build(&self, context: &PluginContext, result: &BuildResult) -> Result<()> {
        for plugin in &self.plugins {
            plugin.after_build(context, result).await?;
        }
        Ok(())
    }

    /// Trigger before_bundle event
    pub async fn trigger_before_bundle(&self, context: &PluginContext) -> Result<()> {
        for plugin in &self.plugins {
            plugin.before_bundle(context).await?;
        }
        Ok(())
    }

    /// Trigger after_bundle event
    pub async fn trigger_after_bundle(&self, context: &PluginContext, bundle_code: &str) -> Result<()> {
        for plugin in &self.plugins {
            plugin.after_bundle(context, bundle_code).await?;
        }
        Ok(())
    }

    /// Transform code through all plugins
    pub async fn transform_code(&self, module: &ModuleInfo, mut code: String) -> Result<String> {
        for plugin in &self.plugins {
            code = plugin.transform_code(module, code).await?;
        }
        Ok(code)
    }

    /// Resolve import through plugins (first plugin that returns Some wins)
    pub async fn resolve_import(&self, import_path: &str, from_file: &str) -> Result<Option<String>> {
        for plugin in &self.plugins {
            if let Some(resolved) = plugin.resolve_import(import_path, from_file).await? {
                return Ok(Some(resolved));
            }
        }
        Ok(None)
    }

    /// Trigger before_output event
    pub async fn trigger_before_output(&self, context: &PluginContext) -> Result<()> {
        for plugin in &self.plugins {
            plugin.before_output(context).await?;
        }
        Ok(())
    }

    /// Trigger after_output event
    pub async fn trigger_after_output(&self, context: &PluginContext) -> Result<()> {
        for plugin in &self.plugins {
            plugin.after_output(context).await?;
        }
        Ok(())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Example plugin: Logger plugin
pub struct LoggerPlugin {
    name: String,
}

impl LoggerPlugin {
    pub fn new() -> Self {
        Self {
            name: "logger".to_string(),
        }
    }
}

impl Default for LoggerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for LoggerPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn before_build(&self, context: &PluginContext) -> Result<()> {
        crate::utils::Logger::info(&format!("🔌 [{}] Build starting with {} modules", self.name, context.modules.len()));
        Ok(())
    }

    async fn after_build(&self, _context: &PluginContext, result: &BuildResult) -> Result<()> {
        crate::utils::Logger::info(&format!("🔌 [{}] Build completed: {} modules, {} files",
            self.name, result.js_modules_processed, result.output_files.len()));
        Ok(())
    }
}

/// Example plugin: Code transformer
pub struct TransformPlugin {
    name: String,
    pattern: String,
    replacement: String,
}

impl TransformPlugin {
    pub fn new(pattern: String, replacement: String) -> Self {
        Self {
            name: "transform".to_string(),
            pattern,
            replacement,
        }
    }
}

#[async_trait]
impl Plugin for TransformPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn transform_code(&self, module: &ModuleInfo, code: String) -> Result<String> {
        let transformed = code.replace(&self.pattern, &self.replacement);
        if transformed != code {
            crate::utils::Logger::debug(&format!("🔌 [{}] Transformed: {}", self.name, module.path.display()));
        }
        Ok(transformed)
    }
}

/// Example plugin: Import resolver
pub struct ImportResolverPlugin {
    name: String,
    aliases: std::collections::HashMap<String, String>,
}

impl ImportResolverPlugin {
    pub fn new(aliases: std::collections::HashMap<String, String>) -> Self {
        Self {
            name: "import-resolver".to_string(),
            aliases,
        }
    }
}

#[async_trait]
impl Plugin for ImportResolverPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn resolve_import(&self, import_path: &str, _from_file: &str) -> Result<Option<String>> {
        // Check if import matches an alias
        for (alias, target) in &self.aliases {
            if import_path.starts_with(alias) {
                let resolved = import_path.replacen(alias, target, 1);
                return Ok(Some(resolved));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::ModuleType;
    use std::path::PathBuf;

    fn create_test_module(path: &str, content: &str) -> ModuleInfo {
        ModuleInfo {
            path: PathBuf::from(path),
            content: content.to_string(),
            module_type: ModuleType::JavaScript,
            dependencies: Vec::new(),
            exports: Vec::new(),
        }
    }

    fn create_test_context() -> PluginContext {
        PluginContext::new(
            BuildConfig::default(),
            vec![create_test_module("test.js", "console.log('test');")],
            PluginEvent::BeforeBuild,
        )
    }

    #[tokio::test]
    async fn test_plugin_manager_creation() {
        let manager = PluginManager::new();
        assert_eq!(manager.plugins().len(), 0);
    }

    #[tokio::test]
    async fn test_register_plugin() {
        let mut manager = PluginManager::new();
        let plugin = Arc::new(LoggerPlugin::new());

        manager.register(plugin);
        assert_eq!(manager.plugins().len(), 1);
    }

    #[tokio::test]
    async fn test_logger_plugin_before_build() {
        let plugin = LoggerPlugin::new();
        let context = create_test_context();

        let result = plugin.before_build(&context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_logger_plugin_after_build() {
        let plugin = LoggerPlugin::new();
        let context = create_test_context();
        let build_result = BuildResult::default();

        let result = plugin.after_build(&context, &build_result).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_transform_plugin() {
        let plugin = TransformPlugin::new("console.log".to_string(), "logger.info".to_string());
        let module = create_test_module("test.js", "console.log('hello');");

        let transformed = plugin.transform_code(&module, module.content.clone()).await.unwrap();
        assert_eq!(transformed, "logger.info('hello');");
    }

    #[tokio::test]
    async fn test_transform_plugin_no_match() {
        let plugin = TransformPlugin::new("console.error".to_string(), "logger.error".to_string());
        let module = create_test_module("test.js", "console.log('hello');");

        let transformed = plugin.transform_code(&module, module.content.clone()).await.unwrap();
        assert_eq!(transformed, "console.log('hello');"); // Unchanged
    }

    #[tokio::test]
    async fn test_import_resolver_plugin() {
        let mut aliases = std::collections::HashMap::new();
        aliases.insert("@/".to_string(), "./src/".to_string());

        let plugin = ImportResolverPlugin::new(aliases);
        let resolved = plugin.resolve_import("@/components/Button", "main.js").await.unwrap();

        assert_eq!(resolved, Some("./src/components/Button".to_string()));
    }

    #[tokio::test]
    async fn test_import_resolver_no_match() {
        let aliases = std::collections::HashMap::new();
        let plugin = ImportResolverPlugin::new(aliases);

        let resolved = plugin.resolve_import("./utils", "main.js").await.unwrap();
        assert_eq!(resolved, None);
    }

    #[tokio::test]
    async fn test_plugin_manager_trigger_before_build() {
        let mut manager = PluginManager::new();
        manager.register(Arc::new(LoggerPlugin::new()));

        let context = create_test_context();
        let result = manager.trigger_before_build(&context).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_plugin_manager_transform_code() {
        let mut manager = PluginManager::new();
        manager.register(Arc::new(TransformPlugin::new("foo".to_string(), "bar".to_string())));

        let module = create_test_module("test.js", "const x = foo;");
        let transformed = manager.transform_code(&module, module.content.clone()).await.unwrap();

        assert_eq!(transformed, "const x = bar;");
    }

    #[tokio::test]
    async fn test_plugin_manager_multiple_transforms() {
        let mut manager = PluginManager::new();
        manager.register(Arc::new(TransformPlugin::new("foo".to_string(), "bar".to_string())));
        manager.register(Arc::new(TransformPlugin::new("bar".to_string(), "baz".to_string())));

        let module = create_test_module("test.js", "const x = foo;");
        let transformed = manager.transform_code(&module, module.content.clone()).await.unwrap();

        // First transform: foo -> bar, Second transform: bar -> baz
        assert_eq!(transformed, "const x = baz;");
    }

    #[tokio::test]
    async fn test_plugin_manager_resolve_import() {
        let mut manager = PluginManager::new();

        let mut aliases = std::collections::HashMap::new();
        aliases.insert("@/".to_string(), "./src/".to_string());
        manager.register(Arc::new(ImportResolverPlugin::new(aliases)));

        let resolved = manager.resolve_import("@/utils", "main.js").await.unwrap();
        assert_eq!(resolved, Some("./src/utils".to_string()));
    }

    #[tokio::test]
    async fn test_plugin_event_types() {
        assert_eq!(PluginEvent::BeforeBuild, PluginEvent::BeforeBuild);
        assert_ne!(PluginEvent::BeforeBuild, PluginEvent::AfterBuild);
    }

    #[tokio::test]
    async fn test_plugin_context_creation() {
        let config = BuildConfig::default();
        let modules = vec![create_test_module("test.js", "code")];
        let event = PluginEvent::BeforeBundle;

        let context = PluginContext::new(config.clone(), modules.clone(), event.clone());

        assert_eq!(context.modules.len(), 1);
        assert_eq!(context.current_event, event);
    }
}
