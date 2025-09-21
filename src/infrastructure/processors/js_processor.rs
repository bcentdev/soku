use crate::core::{interfaces::JsProcessor, models::*};
use crate::utils::{Result, UltraError, Logger, UltraCache};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::sync::Arc;

#[derive(Clone)]
pub struct OxcJsProcessor {
    cache: Arc<UltraCache>,
}

#[async_trait::async_trait]
impl JsProcessor for OxcJsProcessor {
    async fn process_module(&self, module: &ModuleInfo) -> Result<String> {
        let _timer = crate::utils::Timer::start(&format!("Processing {}",
            module.path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")));

        // Check cache first
        let path_str = module.path.to_string_lossy();
        if let Some(cached) = self.cache.get_js(&path_str, &module.content) {
            return Ok(cached);
        }

        let result = match module.module_type {
            ModuleType::JavaScript | ModuleType::TypeScript => {
                self.process_js_module(module).await
            }
            _ => Err(UltraError::Build(format!(
                "Unsupported module type: {:?}",
                module.module_type
            ))),
        };

        // Cache the result
        if let Ok(ref processed) = result {
            self.cache.cache_js(&path_str, &module.content, processed.clone());
        }

        result
    }

    async fn bundle_modules(&self, modules: &[ModuleInfo]) -> Result<String> {
        let _timer = crate::utils::Timer::start("Bundling JavaScript modules");

        let mut bundle = String::new();
        bundle.push_str("// Ultra Bundler - Optimized Build Output\n");
        bundle.push_str("(function() {\n'use strict';\n\n");

        // Process modules sequentially with caching for performance
        for module in modules {
            if self.supports_module_type(&module.module_type) {
                Logger::processing_file(
                    module.path.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown"),
                    "bundling"
                );

                let processed = self.process_module(module).await?;
                bundle.push_str(&format!(
                    "// Module: {}\n",
                    module.path.display()
                ));
                bundle.push_str(&processed);
                bundle.push_str("\n\n");
            }
        }

        bundle.push_str("})();\n");
        Ok(bundle)
    }

    fn supports_module_type(&self, module_type: &ModuleType) -> bool {
        matches!(module_type, ModuleType::JavaScript | ModuleType::TypeScript)
    }
}

impl OxcJsProcessor {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(UltraCache::new()),
        }
    }

    async fn process_js_module(&self, module: &ModuleInfo) -> Result<String> {
        // Parse with oxc for validation
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(&module.path)
            .unwrap_or_default();

        let parser = Parser::new(&allocator, &module.content, source_type);
        let result = parser.parse();

        if !result.errors.is_empty() {
            Logger::warn(&format!(
                "Parser warnings in {}: {} issues",
                module.path.display(),
                result.errors.len()
            ));
        }

        // Process the module content while preserving functionality
        let processed = self.transform_module_content(&module.content);

        Ok(processed)
    }

    fn transform_module_content(&self, content: &str) -> String {
        let mut processed_lines = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("import ") {
                // Transform import statements into comments for now
                // In a full implementation, we'd resolve and inline the imports
                processed_lines.push(format!("// {}", line));
            } else if trimmed.starts_with("export ") {
                // Handle exports - transform them to regular declarations
                if trimmed.starts_with("export const ") || trimmed.starts_with("export let ") || trimmed.starts_with("export var ") {
                    // Transform "export const foo = ..." to "const foo = ..."
                    processed_lines.push(line.replace("export ", ""));
                } else if trimmed.starts_with("export function ") {
                    // Transform "export function foo()" to "function foo()"
                    processed_lines.push(line.replace("export ", ""));
                } else if trimmed.starts_with("export {") || trimmed.starts_with("export *") {
                    // Transform export statements to comments
                    processed_lines.push(format!("// {}", line));
                } else {
                    // Keep other export patterns as comments
                    processed_lines.push(format!("// {}", line));
                }
            } else {
                // Keep regular code as-is
                processed_lines.push(line.to_string());
            }
        }

        processed_lines.join("\n")
    }

    pub fn extract_dependencies(&self, content: &str) -> Vec<String> {
        let mut dependencies = Vec::new();

        println!("🔍 DEBUG: Extracting dependencies from content ({} lines)", content.lines().count());

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Handle different import patterns
            if trimmed.starts_with("import ") {
                println!("  Line {}: Found import: {}", line_num + 1, trimmed);

                if let Some(from_index) = trimmed.rfind(" from ") {
                    let import_path = &trimmed[from_index + 6..];
                    // Remove quotes and semicolon
                    let clean_path = import_path.trim_matches(|c| c == '"' || c == '\'' || c == ';');

                    println!("    Raw import path: '{}', clean path: '{}'", import_path, clean_path);

                    if !clean_path.is_empty() {
                        // Only handle relative imports for now
                        if clean_path.starts_with("./") || clean_path.starts_with("../") {
                            println!("    ✅ Adding dependency: '{}'", clean_path);
                            dependencies.push(clean_path.to_string());
                        } else {
                            println!("    ❌ Skipping non-relative import: '{}'", clean_path);
                        }
                    } else {
                        println!("    ❌ Empty clean path");
                    }
                } else {
                    println!("    ❌ No ' from ' found in import");
                }
            }
        }

        println!("🔍 DEBUG: Found {} dependencies: {:?}", dependencies.len(), dependencies);
        dependencies
    }
}

impl Default for OxcJsProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_js_processing() {
        let processor = OxcJsProcessor::new();

        let module = ModuleInfo {
            path: PathBuf::from("test.js"),
            content: r#"
import { helper } from './helper.js';
export const test = () => console.log('test');
const result = helper();
console.log(result);
"#.to_string(),
            module_type: ModuleType::JavaScript,
            dependencies: vec![],
            exports: vec![],
        };

        let result = processor.process_module(&module).await.unwrap();

        // Should remove import/export but keep the rest
        assert!(!result.contains("import"));
        assert!(!result.contains("export"));
        assert!(result.contains("const result = helper();"));
        assert!(result.contains("console.log(result);"));
    }

    #[tokio::test]
    async fn test_bundle_modules() {
        let processor = OxcJsProcessor::new();

        let modules = vec![
            ModuleInfo {
                path: PathBuf::from("module1.js"),
                content: "console.log('module1');".to_string(),
                module_type: ModuleType::JavaScript,
                dependencies: vec![],
                exports: vec![],
            },
            ModuleInfo {
                path: PathBuf::from("module2.js"),
                content: "console.log('module2');".to_string(),
                module_type: ModuleType::JavaScript,
                dependencies: vec![],
                exports: vec![],
            },
        ];

        let result = processor.bundle_modules(&modules).await.unwrap();

        assert!(result.contains("Ultra Bundler"));
        assert!(result.contains("module1"));
        assert!(result.contains("module2"));
        assert!(result.starts_with("//"));
        assert!(result.ends_with("})();\n"));
    }
}