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
}

/// JavaScript/TypeScript processing interface
#[async_trait]
pub trait JsProcessor: Send + Sync {
    async fn process_module(&self, module: &ModuleInfo) -> Result<String>;
    async fn bundle_modules(&self, modules: &[ModuleInfo]) -> Result<String>;
    async fn bundle_modules_with_tree_shaking(&self, modules: &[ModuleInfo], tree_shaking_stats: Option<&TreeShakingStats>) -> Result<String>;
    fn supports_module_type(&self, module_type: &ModuleType) -> bool;
}

/// CSS processing interface
#[async_trait]
pub trait CssProcessor: Send + Sync {
    async fn process_css(&self, content: &str, path: &Path) -> Result<String>;
    async fn bundle_css(&self, files: &[PathBuf]) -> Result<String>;
}

/// Tree shaking interface
#[async_trait]
pub trait TreeShaker: Send + Sync {
    async fn analyze_modules(&mut self, modules: &[ModuleInfo]) -> Result<()>;
    async fn shake(&mut self, entry_points: &[String]) -> Result<TreeShakingStats>;
}

/// Build service interface
#[async_trait]
pub trait BuildService: Send + Sync {
    async fn build(&self, config: &BuildConfig) -> Result<BuildResult>;
}
