use std::path::PathBuf;
use serde::{Serialize, Deserialize};

/// Bundle output with optional source map
#[derive(Debug, Clone)]
pub struct BundleOutput {
    pub code: String,
    pub source_map: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub root: PathBuf,
    pub outdir: PathBuf,
    pub enable_tree_shaking: bool,
    #[allow(dead_code)] // Future feature
    pub enable_minification: bool,
    #[allow(dead_code)] // Future feature
    pub enable_source_maps: bool,
    #[allow(dead_code)] // Smart bundling feature
    pub enable_code_splitting: bool,
    #[allow(dead_code)] // Maximum chunk size in bytes
    pub max_chunk_size: Option<usize>,
    /// Enable CSS Modules for scoped CSS
    pub enable_css_modules: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            outdir: PathBuf::from("dist"),
            enable_tree_shaking: true,
            enable_minification: true,
            enable_source_maps: false,
            enable_code_splitting: false, // Disabled by default for now
            max_chunk_size: Some(250_000), // 250KB default
            enable_css_modules: false, // Disabled by default
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub path: PathBuf,
    pub content: String,
    pub module_type: ModuleType,
    #[allow(dead_code)] // Used for advanced dependency analysis
    pub dependencies: Vec<String>,
    #[allow(dead_code)] // Used for advanced export analysis
    pub exports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
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
    #[allow(dead_code)] // Used for detailed reporting
    pub js_modules_processed: usize,
    #[allow(dead_code)] // Used for detailed reporting
    pub css_files_processed: usize,
    #[allow(dead_code)] // Used for detailed reporting
    pub tree_shaking_stats: Option<TreeShakingStats>,
    #[allow(dead_code)] // Used for detailed reporting
    pub build_time: std::time::Duration,
    pub output_files: Vec<OutputFile>,
    pub success: bool,
    pub errors: Vec<String>,
    #[allow(dead_code)] // Used for detailed reporting
    pub warnings: Vec<String>,
    #[allow(dead_code)] // Used for bundle analysis
    pub modules: Vec<ModuleInfo>,
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

