// Advanced HMR Hooks - Customizable Hot Module Replacement lifecycle
#![allow(dead_code)] // Public API - used via examples and external integrations

use crate::utils::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// Type alias for transform functions to reduce complexity
type TransformFn = Arc<dyn Fn(&str) -> Result<String> + Send + Sync>;

/// HMR update information passed to hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HmrHookContext {
    pub file_path: PathBuf,
    pub content: Option<String>,
    pub update_kind: HmrHookUpdateKind,
    pub timestamp: u64,
    pub client_count: usize,
}

/// Type of HMR update
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HmrHookUpdateKind {
    JavaScript,
    TypeScript,
    Css,
    Asset,
    Html,
    Other,
}

impl HmrHookContext {
    pub fn new(file_path: PathBuf, update_kind: HmrHookUpdateKind) -> Self {
        Self {
            file_path,
            content: None,
            update_kind,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            client_count: 0,
        }
    }

    pub fn with_content(mut self, content: String) -> Self {
        self.content = Some(content);
        self
    }

    pub fn with_client_count(mut self, count: usize) -> Self {
        self.client_count = count;
        self
    }
}

/// HMR hook trait for customizing HMR behavior
#[async_trait]
pub trait HmrHook: Send + Sync {
    /// Hook name for logging
    fn name(&self) -> &str;

    /// Called before sending an HMR update
    async fn before_update(&self, _context: &HmrHookContext) -> Result<()> {
        Ok(())
    }

    /// Called after successfully sending an HMR update
    async fn after_update(&self, _context: &HmrHookContext) -> Result<()> {
        Ok(())
    }

    /// Transform update content before sending
    async fn transform_content(&self, _context: &HmrHookContext, content: String) -> Result<String> {
        Ok(content)
    }

    /// Decide if full reload is needed (true = full reload, false = HMR update)
    async fn should_full_reload(&self, _context: &HmrHookContext) -> Result<bool> {
        Ok(false)
    }

    /// Called when an HMR client connects
    async fn on_client_connect(&self, _client_id: &str) -> Result<()> {
        Ok(())
    }

    /// Called when an HMR client disconnects
    async fn on_client_disconnect(&self, _client_id: &str) -> Result<()> {
        Ok(())
    }

    /// Called before full page reload
    async fn before_reload(&self, _context: &HmrHookContext) -> Result<()> {
        Ok(())
    }

    /// Called when HMR update fails
    async fn on_update_error(&self, _context: &HmrHookContext, _error: &str) -> Result<()> {
        Ok(())
    }
}

/// Manages and executes HMR hooks
pub struct HmrHookManager {
    hooks: Vec<Arc<dyn HmrHook>>,
}

impl HmrHookManager {
    #[allow(dead_code)] // Public API - used internally by HMR service
    pub fn new() -> Self {
        Self {
            hooks: Vec::new(),
        }
    }

    /// Register an HMR hook
    pub fn register(&mut self, hook: Arc<dyn HmrHook>) {
        self.hooks.push(hook);
    }

    /// Get number of registered hooks
    pub fn hook_count(&self) -> usize {
        self.hooks.len()
    }

    /// Execute before_update hooks
    pub async fn trigger_before_update(&self, context: &HmrHookContext) -> Result<()> {
        for hook in &self.hooks {
            hook.before_update(context).await?;
        }
        Ok(())
    }

    /// Execute after_update hooks
    pub async fn trigger_after_update(&self, context: &HmrHookContext) -> Result<()> {
        for hook in &self.hooks {
            hook.after_update(context).await?;
        }
        Ok(())
    }

    /// Transform content through all hooks
    pub async fn transform_content(&self, context: &HmrHookContext, mut content: String) -> Result<String> {
        for hook in &self.hooks {
            content = hook.transform_content(context, content).await?;
        }
        Ok(content)
    }

    /// Check if any hook requests full reload
    pub async fn should_full_reload(&self, context: &HmrHookContext) -> Result<bool> {
        for hook in &self.hooks {
            if hook.should_full_reload(context).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Trigger client connect hooks
    pub async fn trigger_client_connect(&self, client_id: &str) -> Result<()> {
        for hook in &self.hooks {
            hook.on_client_connect(client_id).await?;
        }
        Ok(())
    }

    /// Trigger client disconnect hooks
    pub async fn trigger_client_disconnect(&self, client_id: &str) -> Result<()> {
        for hook in &self.hooks {
            hook.on_client_disconnect(client_id).await?;
        }
        Ok(())
    }

    /// Trigger before reload hooks
    pub async fn trigger_before_reload(&self, context: &HmrHookContext) -> Result<()> {
        for hook in &self.hooks {
            hook.before_reload(context).await?;
        }
        Ok(())
    }

    /// Trigger error hooks
    pub async fn trigger_update_error(&self, context: &HmrHookContext, error: &str) -> Result<()> {
        for hook in &self.hooks {
            hook.on_update_error(context, error).await?;
        }
        Ok(())
    }
}

impl Default for HmrHookManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in HMR hooks
pub struct BuiltInHmrHooks;

impl BuiltInHmrHooks {
    /// Logging hook - logs all HMR events
    pub fn logger() -> LoggingHook {
        LoggingHook {
            verbose: false,
        }
    }

    /// Full reload hook for specific file patterns
    pub fn full_reload_on_pattern(pattern: String) -> FullReloadPatternHook {
        FullReloadPatternHook { pattern }
    }

    /// Notification hook - displays desktop notifications
    pub fn notification() -> NotificationHook {
        NotificationHook {
            enabled: true,
        }
    }

    /// Throttle hook - limits update frequency
    pub fn throttle(min_interval_ms: u64) -> ThrottleHook {
        ThrottleHook {
            min_interval_ms,
            last_update: std::sync::Arc::new(std::sync::Mutex::new(0)),
        }
    }

    /// Transform hook - applies custom transformations
    pub fn transform<F>(name: String, func: F) -> TransformHook
    where
        F: Fn(&str) -> Result<String> + Send + Sync + 'static,
    {
        TransformHook {
            name,
            transform_fn: Arc::new(func),
        }
    }
}

/// Logging hook implementation
pub struct LoggingHook {
    verbose: bool,
}

impl LoggingHook {
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

#[async_trait]
impl HmrHook for LoggingHook {
    fn name(&self) -> &str {
        "logging"
    }

    async fn before_update(&self, context: &HmrHookContext) -> Result<()> {
        if self.verbose {
            crate::utils::Logger::debug(&format!(
                "🔥 [HMR] Before update: {} ({:?})",
                context.file_path.display(),
                context.update_kind
            ));
        }
        Ok(())
    }

    async fn after_update(&self, context: &HmrHookContext) -> Result<()> {
        crate::utils::Logger::info(&format!(
            "✨ [HMR] Updated: {} → {} clients",
            context.file_path.display(),
            context.client_count
        ));
        Ok(())
    }

    async fn on_client_connect(&self, client_id: &str) -> Result<()> {
        crate::utils::Logger::info(&format!("🔌 [HMR] Client connected: {}", client_id));
        Ok(())
    }

    async fn on_client_disconnect(&self, client_id: &str) -> Result<()> {
        crate::utils::Logger::info(&format!("🔌 [HMR] Client disconnected: {}", client_id));
        Ok(())
    }
}

/// Full reload pattern hook
pub struct FullReloadPatternHook {
    pattern: String,
}

#[async_trait]
impl HmrHook for FullReloadPatternHook {
    fn name(&self) -> &str {
        "full-reload-pattern"
    }

    async fn should_full_reload(&self, context: &HmrHookContext) -> Result<bool> {
        let path_str = context.file_path.to_string_lossy();
        Ok(path_str.contains(&self.pattern))
    }
}

/// Notification hook
pub struct NotificationHook {
    enabled: bool,
}

#[async_trait]
impl HmrHook for NotificationHook {
    fn name(&self) -> &str {
        "notification"
    }

    async fn after_update(&self, context: &HmrHookContext) -> Result<()> {
        if self.enabled {
            crate::utils::Logger::info(&format!(
                "📢 [HMR] Desktop notification: {} updated",
                context.file_path.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
        Ok(())
    }
}

/// Throttle hook
pub struct ThrottleHook {
    min_interval_ms: u64,
    last_update: Arc<std::sync::Mutex<u64>>,
}

#[async_trait]
impl HmrHook for ThrottleHook {
    fn name(&self) -> &str {
        "throttle"
    }

    async fn before_update(&self, context: &HmrHookContext) -> Result<()> {
        let mut last = self.last_update.lock().unwrap();
        let now = context.timestamp;

        if now - *last < self.min_interval_ms {
            // Too soon, could potentially skip update (for now just log)
            crate::utils::Logger::debug(&format!(
                "⏱️ [HMR] Throttling update ({}ms since last)",
                now - *last
            ));
        }

        *last = now;
        Ok(())
    }
}

/// Transform hook
pub struct TransformHook {
    name: String,
    transform_fn: TransformFn,
}

#[async_trait]
impl HmrHook for TransformHook {
    fn name(&self) -> &str {
        &self.name
    }

    async fn transform_content(&self, _context: &HmrHookContext, content: String) -> Result<String> {
        (self.transform_fn)(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hmr_hook_manager_creation() {
        let manager = HmrHookManager::new();
        assert_eq!(manager.hook_count(), 0);
    }

    #[tokio::test]
    async fn test_register_hook() {
        let mut manager = HmrHookManager::new();
        let hook = Arc::new(BuiltInHmrHooks::logger());

        manager.register(hook);
        assert_eq!(manager.hook_count(), 1);
    }

    #[tokio::test]
    async fn test_logging_hook_before_update() {
        let hook = BuiltInHmrHooks::logger();
        let context = HmrHookContext::new(
            PathBuf::from("test.js"),
            HmrHookUpdateKind::JavaScript,
        );

        let result = hook.before_update(&context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_logging_hook_after_update() {
        let hook = BuiltInHmrHooks::logger();
        let context = HmrHookContext::new(
            PathBuf::from("test.js"),
            HmrHookUpdateKind::JavaScript,
        );

        let result = hook.after_update(&context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_full_reload_pattern_hook() {
        let hook = BuiltInHmrHooks::full_reload_on_pattern("config".to_string());

        // Should reload for config files
        let context = HmrHookContext::new(
            PathBuf::from("soku.config.json"),
            HmrHookUpdateKind::Other,
        );
        let should_reload = hook.should_full_reload(&context).await.unwrap();
        assert!(should_reload);

        // Should not reload for regular files
        let context = HmrHookContext::new(
            PathBuf::from("main.js"),
            HmrHookUpdateKind::JavaScript,
        );
        let should_reload = hook.should_full_reload(&context).await.unwrap();
        assert!(!should_reload);
    }

    #[tokio::test]
    async fn test_notification_hook() {
        let hook = BuiltInHmrHooks::notification();
        let context = HmrHookContext::new(
            PathBuf::from("test.js"),
            HmrHookUpdateKind::JavaScript,
        );

        let result = hook.after_update(&context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_throttle_hook() {
        let hook = BuiltInHmrHooks::throttle(100);

        let context1 = HmrHookContext::new(
            PathBuf::from("test.js"),
            HmrHookUpdateKind::JavaScript,
        );
        let result1 = hook.before_update(&context1).await;
        assert!(result1.is_ok());

        // Immediate second update (should throttle)
        let context2 = HmrHookContext::new(
            PathBuf::from("test.js"),
            HmrHookUpdateKind::JavaScript,
        );
        let result2 = hook.before_update(&context2).await;
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_transform_hook() {
        let hook = BuiltInHmrHooks::transform("uppercase".to_string(), |content| {
            Ok(content.to_uppercase())
        });

        let context = HmrHookContext::new(
            PathBuf::from("test.js"),
            HmrHookUpdateKind::JavaScript,
        );

        let result = hook.transform_content(&context, "hello".to_string()).await.unwrap();
        assert_eq!(result, "HELLO");
    }

    #[tokio::test]
    async fn test_hook_manager_trigger_before_update() {
        let mut manager = HmrHookManager::new();
        manager.register(Arc::new(BuiltInHmrHooks::logger()));

        let context = HmrHookContext::new(
            PathBuf::from("test.js"),
            HmrHookUpdateKind::JavaScript,
        );

        let result = manager.trigger_before_update(&context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hook_manager_transform_content() {
        let mut manager = HmrHookManager::new();
        manager.register(Arc::new(BuiltInHmrHooks::transform("add-header".to_string(), |content| {
            Ok(format!("// Header\n{}", content))
        })));

        let context = HmrHookContext::new(
            PathBuf::from("test.js"),
            HmrHookUpdateKind::JavaScript,
        );

        let result = manager.transform_content(&context, "code".to_string()).await.unwrap();
        assert!(result.starts_with("// Header"));
    }

    #[tokio::test]
    async fn test_hook_manager_should_full_reload() {
        let mut manager = HmrHookManager::new();
        manager.register(Arc::new(BuiltInHmrHooks::full_reload_on_pattern("config".to_string())));

        // Should reload for config files
        let context = HmrHookContext::new(
            PathBuf::from("soku.config.json"),
            HmrHookUpdateKind::Other,
        );
        let should_reload = manager.should_full_reload(&context).await.unwrap();
        assert!(should_reload);
    }

    #[tokio::test]
    async fn test_hook_context_with_content() {
        let context = HmrHookContext::new(
            PathBuf::from("test.js"),
            HmrHookUpdateKind::JavaScript,
        ).with_content("test content".to_string());

        assert_eq!(context.content, Some("test content".to_string()));
    }

    #[tokio::test]
    async fn test_hook_context_with_client_count() {
        let context = HmrHookContext::new(
            PathBuf::from("test.js"),
            HmrHookUpdateKind::JavaScript,
        ).with_client_count(5);

        assert_eq!(context.client_count, 5);
    }

    #[tokio::test]
    async fn test_multiple_hooks_in_chain() {
        let mut manager = HmrHookManager::new();
        manager.register(Arc::new(BuiltInHmrHooks::transform("first".to_string(), |content| {
            Ok(format!("[{}]", content))
        })));
        manager.register(Arc::new(BuiltInHmrHooks::transform("second".to_string(), |content| {
            Ok(content.to_uppercase())
        })));

        let context = HmrHookContext::new(
            PathBuf::from("test.js"),
            HmrHookUpdateKind::JavaScript,
        );

        let result = manager.transform_content(&context, "test".to_string()).await.unwrap();
        assert_eq!(result, "[TEST]");
    }

    #[tokio::test]
    async fn test_client_connect_disconnect_hooks() {
        let mut manager = HmrHookManager::new();
        manager.register(Arc::new(BuiltInHmrHooks::logger()));

        let result1 = manager.trigger_client_connect("client-123").await;
        assert!(result1.is_ok());

        let result2 = manager.trigger_client_disconnect("client-123").await;
        assert!(result2.is_ok());
    }
}
