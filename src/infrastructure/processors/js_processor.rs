use crate::core::{interfaces::JsProcessor, models::*};
use crate::utils::{Result, UltraError, Logger};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

pub struct OxcJsProcessor;

#[async_trait::async_trait]
impl JsProcessor for OxcJsProcessor {
    async fn process_module(&self, module: &ModuleInfo) -> Result<String> {
        let _timer = crate::utils::Timer::start(&format!("Processing {}",
            module.path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")));

        match module.module_type {
            ModuleType::JavaScript | ModuleType::TypeScript => {
                self.process_js_module(module).await
            }
            _ => Err(UltraError::Build(format!(
                "Unsupported module type: {:?}",
                module.module_type
            ))),
        }
    }

    async fn bundle_modules(&self, modules: &[ModuleInfo]) -> Result<String> {
        let _timer = crate::utils::Timer::start("Bundling JavaScript modules");

        let mut bundle = String::new();
        bundle.push_str("// Ultra Bundler - Optimized Build Output\n");
        bundle.push_str("(function() {\n'use strict';\n\n");

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
        Self
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

        // Simple processing: remove import/export statements for bundling
        let processed = module.content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("import ") && !trimmed.starts_with("export ")
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(processed)
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