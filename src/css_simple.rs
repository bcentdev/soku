// Simplified CSS processor for demo compilation
use crate::cache::{ImportInfo, ImportKind};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssTransformResult {
    pub code: String,
    pub source_map: Option<String>,
    pub imports: Vec<ImportInfo>,
    pub exports: HashMap<String, String>,
    pub dependencies: Vec<std::path::PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CssOptions {
    pub minify: bool,
    pub modules: bool,
    pub autoprefixer: bool,
    pub nesting: bool,
    pub custom_properties: bool,
}

impl Default for CssOptions {
    fn default() -> Self {
        Self {
            minify: false,
            modules: false,
            autoprefixer: true,
            nesting: true,
            custom_properties: true,
        }
    }
}

pub struct LightningCssProcessor {
    options: CssOptions,
}

impl LightningCssProcessor {
    pub fn new(options: CssOptions) -> Self {
        Self { options }
    }

    pub fn transform(&self, source: &str, file_path: &Path) -> Result<CssTransformResult> {
        // Simplified CSS processing for demo
        let imports = self.extract_imports(source);
        let exports = if self.options.modules {
            self.extract_css_modules_exports(source, file_path)
        } else {
            HashMap::new()
        };

        let processed_code = if self.options.minify {
            self.minify_css(source)
        } else {
            source.to_string()
        };

        Ok(CssTransformResult {
            code: processed_code,
            source_map: None,
            imports,
            exports,
            dependencies: Vec::new(),
        })
    }

    fn extract_imports(&self, source: &str) -> Vec<ImportInfo> {
        let mut imports = Vec::new();

        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("@import") {
                if let Some(url) = self.extract_import_url(trimmed) {
                    imports.push(ImportInfo {
                        specifier: url,
                        kind: ImportKind::Css,
                        source_location: None,
                    });
                }
            }
        }

        imports
    }

    fn extract_import_url(&self, import_line: &str) -> Option<String> {
        // Extract URL from @import statement
        if let Some(start) = import_line.find('"') {
            if let Some(end) = import_line[start + 1..].find('"') {
                return Some(import_line[start + 1..start + 1 + end].to_string());
            }
        }
        if let Some(start) = import_line.find('\'') {
            if let Some(end) = import_line[start + 1..].find('\'') {
                return Some(import_line[start + 1..start + 1 + end].to_string());
            }
        }
        None
    }

    fn extract_css_modules_exports(&self, source: &str, file_path: &Path) -> HashMap<String, String> {
        let mut exports = HashMap::new();
        let base_name = file_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module");

        // Simple CSS class extraction
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('.') {
                if let Some(space_pos) = trimmed.find([' ', '{', ':']) {
                    let class_name = &trimmed[1..space_pos];
                    if !class_name.is_empty() {
                        let hashed_name = format!("{}_{}_123abc", base_name, class_name);
                        exports.insert(class_name.to_string(), hashed_name);
                    }
                }
            }
        }

        exports
    }

    fn minify_css(&self, source: &str) -> String {
        // Very basic CSS minification
        source
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with("/*"))
            .collect::<Vec<_>>()
            .join("")
            .replace(" {", "{")
            .replace("{ ", "{")
            .replace(" }", "}")
            .replace("; ", ";")
            .replace(": ", ":")
    }

    pub fn minify(&self, source: &str, _file_path: &Path) -> Result<String> {
        Ok(self.minify_css(source))
    }
}