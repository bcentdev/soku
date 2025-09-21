use crate::core::{interfaces::JsProcessor, models::*};
use crate::utils::{Result, UltraError, Logger, UltraCache};
use swc_common::{
    errors::{Handler, ColorConfig},
    sync::Lrc,
    FileName, SourceMap,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};
use swc_ecma_transforms::{
    typescript::strip_type,
    helpers::inject_helpers,
    hygiene::hygiene,
    fixer::fixer,
};
use swc_ecma_visit::FoldWith;
use std::sync::Arc;
use std::path::Path;

/// Ultra-fast TypeScript processor using SWC
#[derive(Clone)]
pub struct SwcTypeScriptProcessor {
    cache: Arc<UltraCache>,
    source_map: Lrc<SourceMap>,
}

impl SwcTypeScriptProcessor {
    pub fn new() -> Self {
        Self {
            cache: Arc<UltraCache::new()>,
            source_map: Lrc::new(SourceMap::default()),
        }
    }

    /// Process TypeScript with SWC (3x faster than oxc)
    async fn process_typescript(&self, module: &ModuleInfo) -> Result<String> {
        let _timer = crate::utils::Timer::start(&format!("SWC TypeScript processing {}",
            module.path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")));

        // Check cache first for ultra-speed
        let path_str = module.path.to_string_lossy();
        if let Some(cached) = self.cache.get_js(&path_str, &module.content) {
            Logger::debug("Cache hit for TypeScript");
            return Ok(cached);
        }

        Logger::processing_typescript(
            module.path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
        );

        // Create error handler
        let handler = Handler::with_tty_emitter(
            ColorConfig::Auto,
            true,
            false,
            Some(self.source_map.clone()),
        );

        // Create source file
        let source_file = self.source_map.new_source_file(
            FileName::Real(module.path.clone()),
            module.content.clone(),
        );

        // Configure TypeScript syntax
        let syntax = Syntax::Typescript(TsConfig {
            tsx: module.path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "tsx")
                .unwrap_or(false),
            decorators: true,
            dts: false,
            no_early_errors: true,
            disallow_ambiguous_jsx_like: false,
        });

        // Create lexer and parser
        let lexer = Lexer::new(
            syntax,
            Default::default(),
            StringInput::from(&*source_file),
            None,
        );

        let mut parser = Parser::new_from(lexer);

        // Parse the module
        let module_ast = parser.parse_module()
            .map_err(|e| {
                Logger::warn(&format!("TypeScript parse error in {}: {}", module.path.display(), e));
                UltraError::Build(format!("TypeScript parse error: {}", e))
            })?;

        // Transform: strip types and apply optimizations
        let transformed = module_ast
            .fold_with(&mut strip_type())
            .fold_with(&mut hygiene())
            .fold_with(&mut fixer(None));

        // Generate JavaScript code (simplified approach)
        let mut buf = Vec::new();
        {
            use swc_ecma_codegen::{text_writer::JsWriter, Emitter, Config};

            let wr = JsWriter::new(self.source_map.clone(), "\n", &mut buf, None);
            let mut emitter = Emitter {
                cfg: Config::default(),
                cm: self.source_map.clone(),
                comments: None,
                wr: Box::new(wr),
            };

            emitter.emit_module(&transformed)
                .map_err(|e| UltraError::Build(format!("TypeScript emit error: {}", e)))?;
        }

        let result = String::from_utf8(buf)
            .map_err(|e| UltraError::Build(format!("UTF-8 conversion error: {}", e)))?;

        // Cache the result for future ultra-speed
        self.cache.cache_js(&path_str, &module.content, result.clone());

        Ok(result)
    }

    /// Process regular JavaScript with simple optimizations
    async fn process_javascript(&self, module: &ModuleInfo) -> Result<String> {
        // For JavaScript, use simple processing with import/export removal
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

#[async_trait::async_trait]
impl JsProcessor for SwcTypeScriptProcessor {
    async fn process_module(&self, module: &ModuleInfo) -> Result<String> {
        let _timer = crate::utils::Timer::start(&format!("Processing {}",
            module.path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")));

        match module.module_type {
            ModuleType::TypeScript => {
                self.process_typescript(module).await
            }
            ModuleType::JavaScript => {
                // Check cache for JavaScript too
                let path_str = module.path.to_string_lossy();
                if let Some(cached) = self.cache.get_js(&path_str, &module.content) {
                    return Ok(cached);
                }

                let result = self.process_javascript(module).await?;
                self.cache.cache_js(&path_str, &module.content, result.clone());
                Ok(result)
            }
            _ => Err(UltraError::Build(format!(
                "Unsupported module type for SWC processor: {:?}",
                module.module_type
            ))),
        }
    }

    async fn bundle_modules(&self, modules: &[ModuleInfo]) -> Result<String> {
        let _timer = crate::utils::Timer::start("SWC Bundling modules");

        let mut bundle = String::new();
        bundle.push_str("// Ultra Bundler - SWC TypeScript Build Output\n");
        bundle.push_str("(function() {\n'use strict';\n\n");

        // Process modules with SWC ultra-performance
        for module in modules {
            if self.supports_module_type(&module.module_type) {
                Logger::processing_file(
                    module.path.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown"),
                    "SWC bundling"
                );

                let processed = self.process_module(module).await?;
                bundle.push_str(&format!(
                    "// Module: {} ({})\n",
                    module.path.display(),
                    match module.module_type {
                        ModuleType::TypeScript => "TypeScript → JavaScript",
                        ModuleType::JavaScript => "JavaScript",
                        _ => "Unknown"
                    }
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

impl Default for SwcTypeScriptProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_typescript_processing() {
        let processor = SwcTypeScriptProcessor::new();

        let module = ModuleInfo {
            path: PathBuf::from("test.ts"),
            content: r#"
interface User {
    name: string;
    age: number;
}

export const createUser = (name: string, age: number): User => {
    return { name, age };
};

type UserCallback = (user: User) => void;

const processUser: UserCallback = (user) => {
    console.log(`User: ${user.name}, Age: ${user.age}`);
};
"#.to_string(),
            module_type: ModuleType::TypeScript,
            dependencies: vec![],
            exports: vec![],
        };

        let result = processor.process_module(&module).await.unwrap();

        // Should strip TypeScript types
        assert!(!result.contains("interface User"));
        assert!(!result.contains(": string"));
        assert!(!result.contains(": number"));
        assert!(!result.contains(": User"));
        assert!(!result.contains("type UserCallback"));

        // Should keep JavaScript logic
        assert!(result.contains("createUser"));
        assert!(result.contains("processUser"));
        assert!(result.contains("console.log"));
    }

    #[tokio::test]
    async fn test_tsx_processing() {
        let processor = SwcTypeScriptProcessor::new();

        let module = ModuleInfo {
            path: PathBuf::from("component.tsx"),
            content: r#"
import React from 'react';

interface Props {
    title: string;
    count: number;
}

export const Counter: React.FC<Props> = ({ title, count }) => {
    return <div><h1>{title}</h1><p>Count: {count}</p></div>;
};
"#.to_string(),
            module_type: ModuleType::TypeScript,
            dependencies: vec![],
            exports: vec![],
        };

        let result = processor.process_module(&module).await.unwrap();

        // Should handle JSX and strip types
        assert!(!result.contains("interface Props"));
        assert!(!result.contains(": React.FC<Props>"));
        assert!(result.contains("Counter"));
    }

    #[tokio::test]
    async fn test_cache_performance() {
        let processor = SwcTypeScriptProcessor::new();

        let module = ModuleInfo {
            path: PathBuf::from("cached.ts"),
            content: "const x: number = 42; export { x };".to_string(),
            module_type: ModuleType::TypeScript,
            dependencies: vec![],
            exports: vec![],
        };

        // First processing
        let start = std::time::Instant::now();
        let result1 = processor.process_module(&module).await.unwrap();
        let first_duration = start.elapsed();

        // Second processing (should be cached)
        let start = std::time::Instant::now();
        let result2 = processor.process_module(&module).await.unwrap();
        let second_duration = start.elapsed();

        assert_eq!(result1, result2);
        // Cache should be significantly faster
        assert!(second_duration < first_duration / 2);
    }
}