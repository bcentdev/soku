use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub root: PathBuf,
    pub outdir: PathBuf,
    pub enable_tree_shaking: bool,
    pub enable_minification: bool,
    pub enable_source_maps: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            outdir: PathBuf::from("dist"),
            enable_tree_shaking: true,
            enable_minification: true,
            enable_source_maps: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub path: PathBuf,
    pub content: String,
    pub module_type: ModuleType,
    pub dependencies: Vec<String>,
    pub exports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleType {
    JavaScript,
    TypeScript,
    Css,
    Html,
    Json,
    Unknown,
}

impl ModuleType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "js" | "jsx" => ModuleType::JavaScript,
            "ts" | "tsx" => ModuleType::TypeScript,
            "css" => ModuleType::Css,
            "html" | "htm" => ModuleType::Html,
            "json" => ModuleType::Json,
            _ => ModuleType::Unknown,
        }
    }
}

#[derive(Debug, Default)]
pub struct BuildResult {
    pub js_modules_processed: usize,
    pub css_files_processed: usize,
    pub tree_shaking_stats: Option<TreeShakingStats>,
    pub build_time: std::time::Duration,
    pub output_files: Vec<OutputFile>,
    pub success: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OutputFile {
    pub path: PathBuf,
    pub content: String,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct TreeShakingStats {
    pub total_modules: usize,
    pub total_exports: usize,
    pub used_exports: usize,
    pub removed_exports: usize,
    pub reduction_percentage: f64,
}

impl std::fmt::Display for TreeShakingStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tree shaking: {:.1}% code reduction, {} exports removed",
            self.reduction_percentage, self.removed_exports
        )
    }
}

#[derive(Debug, Default)]
pub struct ProjectStructure {
    pub js_modules: Vec<PathBuf>,
    pub css_files: Vec<PathBuf>,
    pub html_files: Vec<PathBuf>,
    pub other_files: Vec<PathBuf>,
}

impl ProjectStructure {
    pub fn total_files(&self) -> usize {
        self.js_modules.len() + self.css_files.len() + self.html_files.len() + self.other_files.len()
    }
}