use crate::core::models::*;
use crate::utils::Result;
use std::path::{Path, PathBuf};
use async_trait::async_trait;

/// File system operations interface
#[async_trait]
pub trait FileSystemService: Send + Sync {
    async fn scan_directory(&self, path: &Path) -> Result<ProjectStructure>;
    async fn read_file(&self, path: &Path) -> Result<String>;
    async fn write_file(&self, path: &Path, content: &str) -> Result<()>;
    async fn create_directory(&self, path: &Path) -> Result<()>;
    fn file_exists(&self, path: &Path) -> bool;
}

/// JavaScript/TypeScript processing interface
#[async_trait]
pub trait JsProcessor: Send + Sync {
    async fn process_module(&self, module: &ModuleInfo) -> Result<String>;
    async fn bundle_modules(&self, modules: &[ModuleInfo]) -> Result<String>;
    fn supports_module_type(&self, module_type: &ModuleType) -> bool;
}

/// CSS processing interface
#[async_trait]
pub trait CssProcessor: Send + Sync {
    async fn process_css(&self, content: &str, path: &Path) -> Result<String>;
    async fn bundle_css(&self, files: &[PathBuf]) -> Result<String>;
    fn supports_minification(&self) -> bool;
}

/// Tree shaking interface
#[async_trait]
pub trait TreeShaker: Send + Sync {
    async fn analyze_modules(&mut self, modules: &[ModuleInfo]) -> Result<()>;
    async fn shake(&mut self, entry_points: &[String]) -> Result<TreeShakingStats>;
    async fn optimize_module(&self, module: &ModuleInfo) -> Result<String>;
}

/// Build service interface
#[async_trait]
pub trait BuildService: Send + Sync {
    async fn build(&self, config: &BuildConfig) -> Result<BuildResult>;
}

/// Cache interface
#[async_trait]
pub trait CacheService: Send + Sync {
    async fn get(&self, key: &str) -> Option<String>;
    async fn set(&self, key: &str, value: &str) -> Result<()>;
    async fn invalidate(&self, key: &str) -> Result<()>;
    async fn clear(&self) -> Result<()>;
}