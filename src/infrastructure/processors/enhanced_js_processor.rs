use crate::core::{interfaces::JsProcessor, models::*};
use crate::utils::{Result, UltraError, Logger, UltraCache};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::sync::Arc;
use std::path::Path;

/// Enhanced JavaScript/TypeScript processor with advanced caching and optimizations
#[derive(Clone)]
pub struct EnhancedJsProcessor {
    cache: Arc<UltraCache>,
}

impl EnhancedJsProcessor {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(UltraCache::new()),
        }
    }

    pub fn with_persistent_cache(cache_dir: &Path) -> Self {
        Self {
            cache: Arc::new(UltraCache::with_persistent_cache(cache_dir)),
        }
    }

    /// Enhanced TypeScript processing with better type stripping
    async fn process_typescript(&self, module: &ModuleInfo) -> Result<String> {
        let _timer = crate::utils::Timer::start(&format!("Enhanced TypeScript processing {}",
            module.path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")));

        Logger::processing_typescript(
            module.path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
        );

        // Parse with oxc for validation and analysis
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(&module.path)
            .unwrap_or_default()
            .with_typescript(true);

        let parser = Parser::new(&allocator, &module.content, source_type);
        let result = parser.parse();

        if !result.errors.is_empty() {
            Logger::warn(&format!(
                "TypeScript parse warnings in {}: {} issues",
                module.path.display(),
                result.errors.len()
            ));
        }

        // Enhanced type stripping - more comprehensive than basic version
        let processed = self.strip_typescript_types(&module.content);

        Ok(processed)
    }

    /// Advanced TypeScript type stripping with comprehensive support
    fn strip_typescript_types(&self, content: &str) -> String {
        let mut processed_lines = Vec::new();
        let mut in_multiline_interface = false;
        let mut in_multiline_type = false;
        let mut in_multiline_enum = false;
        let mut brace_count = 0;

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                processed_lines.push(line.to_string());
                continue;
            }

            // Handle multiline constructs
            if in_multiline_interface || in_multiline_type || in_multiline_enum {
                brace_count += trimmed.chars().filter(|&c| c == '{').count() as i32;
                brace_count -= trimmed.chars().filter(|&c| c == '}').count() as i32;

                if brace_count <= 0 {
                    in_multiline_interface = false;
                    in_multiline_type = false;
                    in_multiline_enum = false;
                    brace_count = 0;
                }
                continue; // Skip this line as it's part of a type definition
            }

            // Remove import/export statements for bundling (handled by main processor)
            if trimmed.starts_with("import ") ||
               (trimmed.starts_with("export ") &&
                (trimmed.contains("type ") || trimmed.contains("interface ") ||
                 (trimmed.contains("enum ") && !trimmed.contains("const enum")))) {
                continue;
            }

            // Handle interface declarations
            if trimmed.starts_with("interface ") ||
               (trimmed.starts_with("export ") && trimmed.contains("interface ")) {
                if trimmed.contains('{') && !trimmed.ends_with('}') {
                    in_multiline_interface = true;
                    brace_count = trimmed.chars().filter(|&c| c == '{').count() as i32 -
                                 trimmed.chars().filter(|&c| c == '}').count() as i32;
                }
                continue;
            }

            // Handle type aliases
            if trimmed.starts_with("type ") ||
               (trimmed.starts_with("export ") && trimmed.contains("type ") && trimmed.contains(" = ")) {
                if trimmed.contains('{') && !trimmed.ends_with('}') {
                    in_multiline_type = true;
                    brace_count = trimmed.chars().filter(|&c| c == '{').count() as i32 -
                                 trimmed.chars().filter(|&c| c == '}').count() as i32;
                }
                continue;
            }

            // Handle enum declarations (but keep const enum)
            if (trimmed.starts_with("enum ") && !trimmed.starts_with("const enum")) ||
               (trimmed.starts_with("export ") && trimmed.contains("enum ") && !trimmed.contains("const enum")) {
                if trimmed.contains('{') && !trimmed.ends_with('}') {
                    in_multiline_enum = true;
                    brace_count = trimmed.chars().filter(|&c| c == '{').count() as i32 -
                                 trimmed.chars().filter(|&c| c == '}').count() as i32;
                }
                continue;
            }

            // Process the line for type annotations
            let mut processed = line.to_string();

            // Handle export statements that should be preserved but cleaned
            if trimmed.starts_with("export ") && !trimmed.contains("type ") && !trimmed.contains("interface ") {
                processed = self.clean_export_statement(&processed);
            } else {
                // Clean regular code lines
                processed = self.clean_typescript_annotations(&processed);
            }

            processed_lines.push(processed);
        }

        processed_lines.join("\n")
    }

    /// Clean TypeScript annotations from a single line of code
    fn clean_typescript_annotations(&self, line: &str) -> String {
        let mut result = line.to_string();

        // Clean function declarations with types: function foo(x: number, y: string): boolean -> function foo(x, y)
        if let Ok(re) = regex::Regex::new(r"function\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\(([^)]*)\)\s*:\s*[^{]+(\s*\{)") {
            // Clean parameters inside the function
            let params_cleaned = self.clean_function_parameters(&re.replace_all(&result, "function $1($2)$3").to_string());
            result = params_cleaned;
        }

        // Clean arrow functions with types: (x: number, y: string): boolean => -> (x, y) =>
        if let Ok(re) = regex::Regex::new(r"\(([^)]*)\)\s*:\s*[^=]+(\s*=>)") {
            let params_cleaned = self.clean_function_parameters(&format!("({}){}", "$1", "$2"));
            result = re.replace_all(&result, &params_cleaned).to_string();
        }

        // Clean variable declarations: let x: number = 5 -> let x = 5
        if let Ok(re) = regex::Regex::new(r"(let|const|var)\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*:\s*[^=]+(\s*=)") {
            result = re.replace_all(&result, "$1 $2$3").to_string();
        }

        // Clean function parameters: function foo(x: number, y: string) -> function foo(x, y)
        result = self.clean_function_parameters(&result);

        // Clean return types: ): number => -> ) =>
        if let Ok(re) = regex::Regex::new(r"\)\s*:\s*[^{=>;]+(\s*[{=>;])") {
            result = re.replace_all(&result, ")$1").to_string();
        }

        // Clean simple arrow function return types: (): number => -> () =>
        if let Ok(re) = regex::Regex::new(r"\(\s*\)\s*:\s*[^=]+(\s*=>)") {
            result = re.replace_all(&result, "()$1").to_string();
        }

        // Clean generic types: Array<string> -> Array, Promise<User[]> -> Promise
        result = self.clean_generic_types(&result);

        // Clean class property types: private name: string; -> private name;
        if let Ok(re) = regex::Regex::new(r"(private|public|protected|readonly)\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*:\s*[^;=]+;") {
            result = re.replace_all(&result, "$1 $2;").to_string();
        }

        // Clean as type assertions: value as string -> value
        if let Ok(re) = regex::Regex::new(r"\s+as\s+[a-zA-Z_$][a-zA-Z0-9_$<>\[\]|&\s]*") {
            result = re.replace_all(&result, "").to_string();
        }

        // Clean method return types: method(): Type -> method()
        if let Ok(re) = regex::Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\(\s*\)\s*:\s*[^{;]+(\s*[{;])") {
            result = re.replace_all(&result, "$1()$2").to_string();
        }

        result
    }

    /// Clean function parameters of TypeScript types
    fn clean_function_parameters(&self, content: &str) -> String {
        if let Ok(re) = regex::Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)\s*:\s*[^,\)]+") {
            re.replace_all(content, "$1").to_string()
        } else {
            content.to_string()
        }
    }

    /// Clean generic types recursively
    fn clean_generic_types(&self, content: &str) -> String {
        let mut result = content.to_string();
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10; // Prevent infinite loops

        while iterations < MAX_ITERATIONS {
            if let Ok(re) = regex::Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)<[^<>]*>") {
                let new_result = re.replace_all(&result, "$1").to_string();
                if new_result == result {
                    break; // No more changes
                }
                result = new_result;
                iterations += 1;
            } else {
                break;
            }
        }
        result
    }

    /// Clean export statements while preserving JavaScript functionality
    fn clean_export_statement(&self, line: &str) -> String {
        let mut result = line.to_string();

        // Handle export function with types: export function foo(x: number): string -> export function foo(x)
        if result.contains("export function") {
            result = self.clean_typescript_annotations(&result);
        }

        // Handle export const with types: export const x: number = 5 -> export const x = 5
        if result.contains("export const") || result.contains("export let") || result.contains("export var") {
            result = self.clean_typescript_annotations(&result);
        }

        // Handle export class with types
        if result.contains("export class") {
            result = self.clean_typescript_annotations(&result);
        }

        result
    }

    /// Enhanced JavaScript processing with optimizations
    async fn process_javascript(&self, module: &ModuleInfo) -> Result<String> {
        // Parse with oxc for validation
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(&module.path)
            .unwrap_or_default();

        let parser = Parser::new(&allocator, &module.content, source_type);
        let result = parser.parse();

        if !result.errors.is_empty() {
            Logger::warn(&format!(
                "JavaScript parse warnings in {}: {} issues",
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

#[async_trait::async_trait]
impl JsProcessor for EnhancedJsProcessor {
    async fn process_module(&self, module: &ModuleInfo) -> Result<String> {
        let _timer = crate::utils::Timer::start(&format!("Enhanced processing {}",
            module.path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")));

        // Check persistent cache first for ultra-performance
        let path_str = module.path.to_string_lossy();
        if let Some(cached) = self.cache.get_js(&path_str, &module.content) {
            Logger::debug("Cache hit for enhanced processing");
            return Ok(cached);
        }

        let result = match module.module_type {
            ModuleType::TypeScript => {
                self.process_typescript(module).await
            }
            ModuleType::JavaScript => {
                self.process_javascript(module).await
            }
            _ => Err(UltraError::Build(format!(
                "Unsupported module type for enhanced processor: {:?}",
                module.module_type
            ))),
        };

        // Cache the result for future ultra-speed
        if let Ok(ref processed) = result {
            self.cache.cache_js(&path_str, &module.content, processed.clone());
        }

        result
    }

    async fn bundle_modules(&self, modules: &[ModuleInfo]) -> Result<String> {
        let _timer = crate::utils::Timer::start("Enhanced bundling modules");

        let mut bundle = String::new();
        bundle.push_str("// Ultra Bundler - Enhanced TypeScript/JavaScript Build\n");
        bundle.push_str("(function() {\n'use strict';\n\n");

        // Process modules with enhanced performance
        for module in modules {
            if self.supports_module_type(&module.module_type) {
                Logger::processing_file(
                    module.path.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown"),
                    match module.module_type {
                        ModuleType::TypeScript => "Enhanced TS",
                        ModuleType::JavaScript => "Enhanced JS",
                        _ => "Enhanced"
                    }
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

    async fn bundle_modules_with_tree_shaking(&self, modules: &[ModuleInfo], _tree_shaking_stats: Option<&TreeShakingStats>) -> Result<String> {
        // For now, delegate to regular bundling
        // TODO: Implement tree shaking for enhanced processor
        self.bundle_modules(modules).await
    }

    fn supports_module_type(&self, module_type: &ModuleType) -> bool {
        matches!(module_type, ModuleType::JavaScript | ModuleType::TypeScript)
    }
}

impl Default for EnhancedJsProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_typescript_type_stripping() {
        let processor = EnhancedJsProcessor::new();

        let module = ModuleInfo {
            path: PathBuf::from("test.ts"),
            content: r#"
interface User {
    name: string;
    age: number;
}

type UserCallback = (user: User) => void;

const createUser = (name: string, age: number): User => {
    return { name, age };
};

function processUser(user: User): void {
    console.log(`User: ${user.name}, Age: ${user.age}`);
}

let count: number = 42;
const items: Array<string> = ['a', 'b'];
"#.to_string(),
            module_type: ModuleType::TypeScript,
            dependencies: vec![],
            exports: vec![],
        };

        let result = processor.process_module(&module).await.unwrap();

        // println!("Result: {}", result); // Debug output

        // Should strip TypeScript types
        assert!(!result.contains("interface User"));
        assert!(!result.contains("type UserCallback"));

        // Should keep JavaScript logic
        assert!(result.contains("createUser"));
        assert!(result.contains("processUser"));
        assert!(result.contains("console.log"));

        // Check that basic functionality works
        assert!(!result.contains("interface User"));
        assert!(!result.contains("type UserCallback"));
        assert!(result.contains("createUser"));
        assert!(result.contains("console.log"));

        // Check that some type stripping occurred
        assert!(result.contains("let count = 42")); // Should have ": number" removed
        assert!(result.contains("const items = ['a', 'b']")); // Should have "Array<string>" removed
    }

    #[tokio::test]
    async fn test_enhanced_caching() {
        let processor = EnhancedJsProcessor::new();

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

        // Second processing (should be cached and much faster)
        let start = std::time::Instant::now();
        let result2 = processor.process_module(&module).await.unwrap();
        let second_duration = start.elapsed();

        assert_eq!(result1, result2);
        // Cache should be significantly faster
        assert!(second_duration < first_duration / 2);
    }

    #[tokio::test]
    async fn test_tsx_processing() {
        let processor = EnhancedJsProcessor::new();

        let module = ModuleInfo {
            path: PathBuf::from("component.tsx"),
            content: r#"
interface Props {
    title: string;
    count: number;
}

const Counter = ({ title, count }: Props) => {
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
        assert!(!result.contains(": Props"));
        assert!(result.contains("Counter"));
        assert!(result.contains("<div>"));
    }
}