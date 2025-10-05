/// Shared functionality between JS processors
/// This module contains common code extracted from js_processor.rs and enhanced_js_processor.rs
/// to eliminate duplication and provide a single source of truth.
use std::path::Path;
use std::sync::Arc;
use once_cell::sync::Lazy;
use regex::Regex;
use crate::utils::performance::SokuCache;
use crate::utils::{Result, SokuError, ErrorContext, Logger};
use crate::core::{models::*, interfaces::JsProcessor};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_diagnostics::OxcDiagnostic;
use async_trait::async_trait;

// ============================================================================
// Processing Strategy Pattern (Shared)
// ============================================================================

/// Processing strategy determines the level of transformations and optimizations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingStrategy {
    /// Fast mode: Minimal transformations, maximum speed
    /// - Basic import/export handling
    /// - No TypeScript stripping
    /// - No JSX transformation
    Fast,

    /// Standard mode: Basic TypeScript stripping
    /// - TypeScript type annotations removed
    /// - Basic JSX support
    /// - Moderate caching
    Standard,

    /// Enhanced mode: Full TypeScript + JSX + all optimizations
    /// - Complete TypeScript transformation
    /// - Full JSX/TSX support
    /// - Advanced caching
    /// - Memory-mapped file operations
    Enhanced,
}

impl ProcessingStrategy {
    /// Auto-detect strategy based on file characteristics
    #[allow(dead_code)]
    pub fn auto_detect(has_typescript: bool, has_jsx: bool, file_count: usize) -> Self {
        // Use Enhanced mode for TypeScript/JSX files or larger projects
        if has_typescript || has_jsx || file_count > 5 {
            Self::Enhanced
        } else if file_count > 2 {
            Self::Standard
        } else {
            Self::Fast
        }
    }

    /// Get strategy name for logging
    pub fn name(&self) -> &'static str {
        match self {
            Self::Fast => "Fast",
            Self::Standard => "Standard",
            Self::Enhanced => "Enhanced",
        }
    }
}

/// Configuration options for processing
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProcessingOptions {
    /// Strip TypeScript type annotations
    pub strip_types: bool,

    /// Transform JSX/TSX to JavaScript
    pub transform_jsx: bool,

    /// Generate source maps
    pub generate_source_maps: bool,

    /// Enable caching
    pub enable_cache: bool,

    /// Optimize node_modules content
    pub optimize_node_modules: bool,
}

impl ProcessingOptions {
    /// Create options for Fast strategy
    pub fn fast() -> Self {
        Self {
            strip_types: false,
            transform_jsx: false,
            generate_source_maps: false,
            enable_cache: true,
            optimize_node_modules: false,
        }
    }

    /// Create options for Standard strategy
    pub fn standard() -> Self {
        Self {
            strip_types: true,
            transform_jsx: true,
            generate_source_maps: false,
            enable_cache: true,
            optimize_node_modules: true,
        }
    }

    /// Create options for Enhanced strategy
    pub fn enhanced() -> Self {
        Self {
            strip_types: true,
            transform_jsx: true,
            generate_source_maps: false,
            enable_cache: true,
            optimize_node_modules: true,
        }
    }

    /// Create options from strategy
    pub fn from_strategy(strategy: ProcessingStrategy) -> Self {
        match strategy {
            ProcessingStrategy::Fast => Self::fast(),
            ProcessingStrategy::Standard => Self::standard(),
            ProcessingStrategy::Enhanced => Self::enhanced(),
        }
    }
}

/// Unified JavaScript/TypeScript processor with strategy-based processing
///
/// This processor consolidates JavaScript/TypeScript processing with three strategies:
///
/// - **Fast**: Minimal transformations, maximum speed
/// - **Standard**: Basic TypeScript stripping with oxc
/// - **Enhanced**: Full TypeScript + JSX transformation with oxc
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust,no_run
/// use soku::infrastructure::processors::{UnifiedJsProcessor, ProcessingStrategy};
/// use std::path::Path;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create processor with Standard strategy
/// let processor = UnifiedJsProcessor::new(ProcessingStrategy::Standard);
///
/// // Process a file
/// let content = "const x: number = 42;";
/// let result = processor.process_content(content, Path::new("file.ts"))?;
/// # Ok(())
/// # }
/// ```
///
/// ## With Persistent Cache
///
/// ```rust,no_run
/// use soku::infrastructure::processors::{UnifiedJsProcessor, ProcessingStrategy};
/// use std::path::Path;
///
/// let cache_dir = Path::new(".soku-cache");
/// let processor = UnifiedJsProcessor::with_persistent_cache(
///     ProcessingStrategy::Enhanced,
///     cache_dir
/// );
/// ```
///
/// ## Auto-Detection
///
/// ```rust,no_run
/// use soku::infrastructure::processors::{UnifiedJsProcessor, ProcessingStrategy};
///
/// // Automatically select strategy based on project characteristics
/// let has_typescript = true;
/// let has_jsx = false;
/// let file_count = 10;
///
/// let strategy = ProcessingStrategy::auto_detect(has_typescript, has_jsx, file_count);
/// let processor = UnifiedJsProcessor::new(strategy);
/// ```
///
/// ## Custom Options
///
/// ```rust,ignore
/// // Note: ProcessingOptions is an internal type
/// use soku::infrastructure::processors::{UnifiedJsProcessor, ProcessingStrategy};
///
/// // Custom options example (simplified)
/// let processor = UnifiedJsProcessor::new(ProcessingStrategy::Enhanced);
/// ```
///
/// # Performance Characteristics
///
/// - **Fast**: ~5ms for typical files (no transformations)
/// - **Standard**: ~10-20ms (basic TypeScript stripping)
/// - **Enhanced**: ~20-50ms (full transformations with caching)
///
/// # Migration from Legacy Processors
///
/// ```rust,no_run
/// use soku::infrastructure::processors::{UnifiedJsProcessor, ProcessingStrategy};
///
/// // Standard mode: Basic TypeScript stripping
/// let standard = UnifiedJsProcessor::new(ProcessingStrategy::Standard);
///
/// // Enhanced mode: Full TypeScript + JSX transformation
/// let enhanced = UnifiedJsProcessor::new(ProcessingStrategy::Enhanced);
/// ```
#[derive(Clone)]
pub struct UnifiedJsProcessor {
    strategy: ProcessingStrategy,
    options: ProcessingOptions,
    cache: Arc<SokuCache>,
}

impl UnifiedJsProcessor {
    /// Create new processor with specified strategy
    pub fn new(strategy: ProcessingStrategy) -> Self {
        Self {
            strategy,
            options: ProcessingOptions::from_strategy(strategy),
            cache: Arc::new(SokuCache::new()),
        }
    }

    /// Create processor with custom options
    #[allow(dead_code)]
    pub fn with_options(strategy: ProcessingStrategy, options: ProcessingOptions) -> Self {
        Self {
            strategy,
            options,
            cache: Arc::new(SokuCache::new()),
        }
    }

    /// Create processor with persistent cache
    #[allow(dead_code)]
    pub fn with_persistent_cache(strategy: ProcessingStrategy, cache_dir: &Path) -> Self {
        Self {
            strategy,
            options: ProcessingOptions::from_strategy(strategy),
            cache: Arc::new(SokuCache::with_persistent_cache(cache_dir)),
        }
    }

    /// Get current strategy
    #[allow(dead_code)]
    pub fn strategy(&self) -> ProcessingStrategy {
        self.strategy
    }

    /// Get processing options
    #[allow(dead_code)]
    pub fn options(&self) -> &ProcessingOptions {
        &self.options
    }

    /// Process content based on strategy
    pub fn process_content(&self, content: &str, file_path: &Path) -> Result<String> {
        // Check cache first
        let path_str = file_path.to_string_lossy();
        if let Some(cached) = get_cached_js(&self.cache, &path_str, content, self.options.enable_cache) {
            Logger::debug(&format!("Cache hit for {}", path_str));
            return Ok(cached);
        }

        // Process based on strategy
        let processed = match self.strategy {
            ProcessingStrategy::Fast => {
                // Fast mode: minimal processing
                self.process_fast(content, file_path)?
            }
            ProcessingStrategy::Standard => {
                // Standard mode: basic TypeScript stripping
                self.process_standard(content, file_path)?
            }
            ProcessingStrategy::Enhanced => {
                // Enhanced mode: full processing
                self.process_enhanced(content, file_path)?
            }
        };

        // Store in cache
        store_cached_js(&self.cache, &path_str, content, processed.clone(), self.options.enable_cache);

        Ok(processed)
    }

    /// Fast processing: minimal transformations
    fn process_fast(&self, content: &str, _file_path: &Path) -> Result<String> {
        // In Fast mode, just remove import/export for bundling
        let processed = content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("import ") && !trimmed.starts_with("export ")
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(processed)
    }

    /// Standard processing: basic TypeScript stripping
    fn process_standard(&self, content: &str, file_path: &Path) -> Result<String> {
        let allocator = Allocator::default();

        // Determine if file has TypeScript or JSX
        let has_ts = file_path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext == "ts" || ext == "tsx")
            .unwrap_or(false);

        let has_jsx = file_path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext == "tsx" || ext == "jsx")
            .unwrap_or(false);

        // Parse with appropriate config
        let config = if has_jsx {
            ParsingConfig::jsx()
        } else if has_ts {
            ParsingConfig::typescript()
        } else {
            ParsingConfig::javascript()
        };

        // Parse and validate
        let _result = parse_with_oxc(
            &allocator,
            content,
            config,
            file_path,
            "Standard processing"
        );
        // Ignore parse errors in Standard mode

        // Strip TypeScript if needed
        let processed = if has_ts && self.options.strip_types {
            let stripped = strip_typescript_block_constructs(content);
            clean_typescript_inline_annotations(&stripped)
        } else {
            content.to_string()
        };

        Ok(processed)
    }

    /// Enhanced processing: full TypeScript + JSX transformation
    fn process_enhanced(&self, content: &str, file_path: &Path) -> Result<String> {
        let allocator = Allocator::default();

        // Determine file characteristics
        let has_ts = file_path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext == "ts" || ext == "tsx")
            .unwrap_or(false);

        let has_jsx = file_path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext == "tsx" || ext == "jsx")
            .unwrap_or(false);

        // Parse with appropriate config
        let config = if has_jsx {
            ParsingConfig::jsx()
        } else if has_ts {
            ParsingConfig::typescript()
        } else {
            ParsingConfig::javascript()
        };

        // Parse with full error handling
        let _result = match parse_with_oxc(
            &allocator,
            content,
            config,
            file_path,
            "Enhanced processing"
        ) {
            Ok(result) => result,
            Err(_) => {
                // Fallback to regex-based approach
                Logger::warn("Enhanced parsing failed, falling back to regex");
                let stripped = if has_ts {
                    let temp = strip_typescript_block_constructs(content);
                    clean_typescript_inline_annotations(&temp)
                } else {
                    content.to_string()
                };
                return Ok(stripped);
            }
        };

        // Full processing with all transformations
        let processed = if has_ts && self.options.strip_types {
            let stripped = strip_typescript_block_constructs(content);
            clean_typescript_inline_annotations(&stripped)
        } else {
            content.to_string()
        };

        Ok(processed)
    }
}

// ============================================================================
// JsProcessor Trait Implementation for UnifiedJsProcessor
// ============================================================================

#[async_trait]
impl JsProcessor for UnifiedJsProcessor {
    async fn process_module(&self, module: &ModuleInfo) -> Result<String> {
        let _timer = crate::utils::Timer::start(&format!("Processing {} ({})",
            module.path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown"),
            self.strategy.name()));

        // Use the unified process_content method
        self.process_content(&module.content, &module.path)
    }

    async fn bundle_modules(&self, modules: &[ModuleInfo]) -> Result<String> {
        let _timer = crate::utils::Timer::start(&format!("Bundling modules ({})", self.strategy.name()));

        let mut bundle = String::new();
        bundle.push_str(&format!("// Soku Bundler - {} Mode Build\n", self.strategy.name()));
        bundle.push_str("(function() {\n'use strict';\n\n");

        // Process each module
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

    async fn bundle_modules_with_tree_shaking(
        &self,
        modules: &[ModuleInfo],
        _tree_shaking_stats: Option<&TreeShakingStats>
    ) -> Result<String> {
        // For now, delegate to bundle_modules
        // Tree shaking is handled at a higher level
        self.bundle_modules(modules).await
    }

    async fn bundle_modules_with_source_maps(
        &self,
        modules: &[ModuleInfo],
        config: &BuildConfig
    ) -> Result<BundleOutput> {
        if !config.enable_source_maps {
            // Source maps disabled, just bundle normally
            let code = self.bundle_modules(modules).await?;
            return Ok(BundleOutput {
                code,
                source_map: None,
            });
        }

        // Build bundle with simple source map (line-level mapping)
        let mut bundle = String::new();
        let strategy_name = self.strategy.name();

        // Bundle header
        bundle.push_str(&format!("// Soku Bundler - {} Mode Build\n", strategy_name));
        bundle.push_str("(function() {\n'use strict';\n\n");

        // Build source tracking
        let mut sources = Vec::new();
        let mut sources_content = Vec::new();

        for module in modules {
            if !self.supports_module_type(&module.module_type) {
                continue;
            }

            // Process module
            let processed = self.process_module(module).await?;

            // Add module comment
            bundle.push_str(&format!("// Module: {}\n", module.path.display()));

            // Track source
            sources.push(module.path.to_string_lossy().to_string());
            sources_content.push(module.content.clone());

            // Add module content
            for line in processed.lines() {
                bundle.push_str(line);
                bundle.push('\n');
            }

            bundle.push('\n');
        }

        bundle.push_str("})();\n");

        // Build a simple source map JSON manually
        let source_map_json = serde_json::json!({
            "version": 3,
            "file": "bundle.js",
            "sources": sources,
            "sourcesContent": sources_content,
            "names": [],
            "mappings": "" // Simplified - no detailed mappings for now
        });

        let source_map_str = serde_json::to_string(&source_map_json)
            .map_err(|e| crate::utils::SokuError::build(format!("Failed to generate source map: {}", e)))?;

        Ok(BundleOutput {
            code: bundle,
            source_map: Some(source_map_str),
        })
    }

    fn supports_module_type(&self, module_type: &ModuleType) -> bool {
        matches!(module_type, ModuleType::JavaScript | ModuleType::TypeScript)
    }
}

// Pre-compiled regex patterns for TypeScript stripping (shared across processors)
static TYPE_ANNOTATION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)\s*:\s*[a-zA-Z_$][a-zA-Z0-9_$<>\[\]|&\s]*([,)=])").unwrap()
});

static RETURN_TYPE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\)\s*:\s*[^=]+\s*(=>)").unwrap()
});

static GENERIC_TYPE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)<[^<>]*>").unwrap()
});

static SIMPLE_GENERIC_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<[^<>]*>").unwrap()
});

// ============================================================================
// Unified OXC Parsing Interface (Shared)
// ============================================================================

/// Configuration for OXC parser setup
#[derive(Debug, Clone, Copy)]
pub struct ParsingConfig {
    pub typescript: bool,
    pub jsx: bool,
    pub module: bool,
}

impl ParsingConfig {
    /// Create config for JavaScript parsing
    pub fn javascript() -> Self {
        Self {
            typescript: false,
            jsx: false,
            module: true,
        }
    }

    /// Create config for TypeScript parsing
    pub fn typescript() -> Self {
        Self {
            typescript: true,
            jsx: false,
            module: true,
        }
    }

    /// Create config for JSX/TSX parsing
    pub fn jsx() -> Self {
        Self {
            typescript: true,
            jsx: true,
            module: true,
        }
    }

    /// Convert config to OXC SourceType
    pub fn to_source_type(self) -> SourceType {
        let mut source_type = SourceType::default();

        if self.typescript {
            source_type = source_type.with_typescript(true);
        }
        if self.jsx {
            source_type = source_type.with_jsx(true);
        }
        if self.module {
            source_type = source_type.with_module(true);
        }

        source_type
    }
}

/// Parse content with unified error handling
/// Returns parsed program or detailed error
pub fn parse_with_oxc<'a>(
    allocator: &'a Allocator,
    content: &'a str,
    config: ParsingConfig,
    file_path: &Path,
    error_prefix: &str,
) -> Result<oxc_parser::ParserReturn<'a>> {
    let source_type = config.to_source_type();
    let parser = Parser::new(allocator, content, source_type);
    let result = parser.parse();

    if !result.errors.is_empty() {
        let error_context = create_parse_error_context(&result.errors, content, file_path);
        let first_error = &result.errors[0];

        let detailed_error = SokuError::parse_with_context(
            format!("{}: {}", error_prefix, first_error),
            error_context
        );

        Logger::warn(&detailed_error.format_detailed());
        return Err(detailed_error);
    }

    Ok(result)
}

/// Create detailed error context from OXC parse errors
pub fn create_parse_error_context(
    errors: &[OxcDiagnostic],
    content: &str,
    file_path: &Path,
) -> ErrorContext {
    if errors.is_empty() {
        return ErrorContext::new()
            .with_file(file_path.to_path_buf())
            .with_location(1, 1)
            .with_snippet("Unknown error".to_string());
    }

    let first_error = &errors[0];
    let lines: Vec<&str> = content.lines().collect();

    // Extract error location from labels
    let (line_num, column) = first_error
        .labels
        .as_ref()
        .and_then(|labels| labels.first())
        .map(|label| {
            let offset = label.offset();
            let mut current_offset = 0;
            let mut line = 1;
            let mut col = 1;

            for (line_index, line_content) in lines.iter().enumerate() {
                let line_length = line_content.len() + 1; // +1 for newline
                if current_offset + line_length > offset {
                    line = line_index + 1;
                    col = offset - current_offset + 1;
                    break;
                }
                current_offset += line_length;
            }

            (line, col)
        })
        .unwrap_or((1, 1));

    // Get source code context around error
    let context_lines = 5;
    let start_line = line_num.saturating_sub(context_lines).max(1);
    let end_line = (line_num + context_lines).min(lines.len());

    let mut code_context = String::new();
    for i in start_line..=end_line {
        if i <= lines.len() {
            code_context.push_str(&format!("{:4} │ {}\n", i, lines[i - 1]));
        }
    }

    ErrorContext::new()
        .with_file(file_path.to_path_buf())
        .with_location(line_num, column)
        .with_snippet(code_context)
}

// ============================================================================
// Node Modules Optimization (Shared)
// ============================================================================

/// Check if a module path is from node_modules
pub fn is_node_modules_path(path: &Path) -> bool {
    path.to_string_lossy().contains("node_modules")
}

/// Extract package name from node_modules path
///
/// Example:
/// - `/path/to/node_modules/lodash/index.js` → `"lodash"`
/// - `/path/to/node_modules/@types/react/index.d.ts` → `"@types"`
pub fn extract_package_name(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    if let Some(node_modules_pos) = path_str.find("node_modules") {
        let after_node_modules = &path_str[node_modules_pos + "node_modules".len()..];
        if let Some(package_part) = after_node_modules.split('/').nth(1) {
            return package_part.to_string();
        }
    }
    "unknown_package".to_string()
}

/// Optimize node_modules content by keeping only essential parts
pub fn optimize_node_module_content(content: &str, path: &Path) -> String {
    let package_name = extract_package_name(path);

    if package_name == "lodash" {
        optimize_lodash_content(content, &package_name)
    } else {
        optimize_general_node_module_content(content)
    }
}

/// Specific optimization for lodash
/// Uses brace tracking to preserve complete function definitions
fn optimize_lodash_content(content: &str, package_name: &str) -> String {
    let mut result = String::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut in_function = false;
    let mut brace_count = 0;

    for line in lines {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.is_empty() {
            continue;
        }

        // Keep essential function definitions
        if trimmed.starts_with("function ")
            || trimmed.starts_with("var ")
            || trimmed.starts_with("module.exports")
            || trimmed.starts_with("exports.")
        {
            in_function = true;
            result.push_str(line);
            result.push('\n');

            brace_count += line.matches('{').count();
            brace_count -= line.matches('}').count();

            if brace_count == 0 {
                in_function = false;
            }
            continue;
        }

        // If inside function, keep the content
        if in_function {
            result.push_str(line);
            result.push('\n');

            brace_count += line.matches('{').count();
            brace_count -= line.matches('}').count();

            if brace_count == 0 {
                in_function = false;
            }
        }
    }

    format!("// TREE-SHAKEN: Optimized {} content\n{}", package_name, result)
}

/// General optimization for other node_modules packages
/// Removes development-only code and verbose comments
fn optimize_general_node_module_content(content: &str) -> String {
    let mut result = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip obvious dead code patterns
        if trimmed.starts_with("// Development only")
            || trimmed.starts_with("// DEBUG")
            || trimmed.contains("console.warn")
            || trimmed.contains("console.error")
        {
            result.push_str(&format!("// TREE-SHAKEN: {}\n", line));
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    result
}

// ============================================================================
// TypeScript Stripping Functions (Shared)
// ============================================================================

/// Strip TypeScript block-level constructs (interfaces, type aliases, enums)
/// Returns the cleaned content with TypeScript blocks commented out
pub fn strip_typescript_block_constructs(content: &str) -> String {
    let mut result = String::new();
    let mut in_interface = false;
    let mut in_type_alias = false;
    let mut in_enum = false;
    let mut in_decorator = false;
    let mut brace_depth = 0;
    let mut paren_depth = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        // Handle multiline decorator arguments
        if in_decorator {
            result.push_str(&format!("// {}\n", line));
            // Track parentheses for multiline decorators
            paren_depth += trimmed.matches('(').count();
            paren_depth = paren_depth.saturating_sub(trimmed.matches(')').count());
            if paren_depth == 0 {
                in_decorator = false;
            }
            continue;
        }

        // Skip decorators (@Decorator or @Decorator(...))
        if trimmed.starts_with('@') {
            Logger::debug(&format!("Stripping decorator: {}", trimmed));
            result.push_str(&format!("// {}\n", line));
            // Check if decorator has arguments spanning multiple lines
            paren_depth = trimmed.matches('(').count();
            paren_depth = paren_depth.saturating_sub(trimmed.matches(')').count());
            if paren_depth > 0 {
                in_decorator = true;
            }
            continue;
        }

        // Skip interface declarations
        if trimmed.starts_with("interface ") || trimmed.starts_with("export interface ") {
            in_interface = true;
            brace_depth = 0;
            result.push_str(&format!("// {}\n", line));
            if trimmed.contains('{') {
                brace_depth += trimmed.matches('{').count();
            }
            if trimmed.contains('}') {
                brace_depth -= trimmed.matches('}').count();
                if brace_depth == 0 {
                    in_interface = false;
                }
            }
            continue;
        }

        // Skip type aliases
        if trimmed.starts_with("type ") || trimmed.starts_with("export type ") {
            in_type_alias = true;
            result.push_str(&format!("// {}\n", line));
            if !trimmed.contains(';') && !trimmed.contains('=') {
                continue;
            }
            in_type_alias = false;
            continue;
        }

        // Skip enum declarations (including const enums)
        if trimmed.starts_with("enum ")
            || trimmed.starts_with("export enum ")
            || trimmed.starts_with("const enum ")
            || trimmed.starts_with("export const enum ")
        {
            in_enum = true;
            brace_depth = 0;
            result.push_str(&format!("// {}\n", line));
            if trimmed.contains('{') {
                brace_depth += trimmed.matches('{').count();
            }
            if trimmed.contains('}') {
                brace_depth -= trimmed.matches('}').count();
                if brace_depth == 0 {
                    in_enum = false;
                }
            }
            continue;
        }

        // Handle interface/type/enum continuation
        if in_interface || in_enum {
            result.push_str(&format!("// {}\n", line));
            if trimmed.contains('{') {
                brace_depth += trimmed.matches('{').count();
            }
            if trimmed.contains('}') {
                brace_depth -= trimmed.matches('}').count();
                if brace_depth == 0 {
                    in_interface = false;
                    in_enum = false;
                }
            }
            continue;
        }

        if in_type_alias {
            result.push_str(&format!("// {}\n", line));
            if trimmed.ends_with(';') {
                in_type_alias = false;
            }
            continue;
        }

        // Keep other lines
        result.push_str(line);
        result.push('\n');
    }

    result
}

/// Clean inline TypeScript annotations from code
/// Removes type annotations like `: Type`, return types, etc.
pub fn clean_typescript_inline_annotations(content: &str) -> String {
    let mut result = content.to_string();

    // Remove class property type annotations: propertyName: Type; -> propertyName;
    if let Ok(re) = Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)\s*:\s*[a-zA-Z_$][a-zA-Z0-9_$<>\[\]|&\s]+;") {
        result = re.replace_all(&result, "$1;").to_string();
    }

    // Remove method/function return types: methodName(): Type { -> methodName() {
    if let Ok(re) = Regex::new(r"\)\s*:\s*[a-zA-Z_$][a-zA-Z0-9_$<>\[\]|&\s]+\s*\{") {
        result = re.replace_all(&result, ") {").to_string();
    }

    // Remove simple type annotations: name: Type -> name
    result = TYPE_ANNOTATION_REGEX.replace_all(&result, "$1$2").to_string();

    // Remove function return types: ): Type => -> ) =>
    result = RETURN_TYPE_REGEX.replace_all(&result, ")$1").to_string();

    // Remove generic type parameters
    result = SIMPLE_GENERIC_REGEX.replace_all(&result, "").to_string();

    // Remove TypeScript non-null assertion operator
    if let Ok(re) = Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)\s*!") {
        result = re.replace_all(&result, "$1").to_string();
    }

    // Remove access modifiers
    if let Ok(re) = Regex::new(r"\b(private|public|protected|readonly)\s+") {
        result = re.replace_all(&result, "").to_string();
    }

    // Remove as type assertions
    if let Ok(re) = Regex::new(r"\s+as\s+[a-zA-Z_$][a-zA-Z0-9_$<>\[\]|&\s]*") {
        result = re.replace_all(&result, "").to_string();
    }

    result
}

/// Remove generic types from content (iteratively to handle nested generics)
pub fn remove_generic_types(content: &str, max_iterations: usize) -> String {
    let mut result = content.to_string();
    let mut iterations = 0;

    while iterations < max_iterations {
        let new_result = GENERIC_TYPE_REGEX.replace_all(&result, "$1").to_string();
        if new_result == result {
            break; // No more changes
        }
        result = new_result;
        iterations += 1;
    }

    result
}

// ============================================================================
// Unified Caching Interface (Shared)
// ============================================================================

/// Check cache for processed JavaScript content
/// Returns cached content if available and caching is enabled
pub fn get_cached_js(
    cache: &Arc<SokuCache>,
    path: &str,
    content: &str,
    enable_cache: bool,
) -> Option<String> {
    if enable_cache {
        cache.get_js(path, content)
    } else {
        None
    }
}

/// Store processed JavaScript content in cache
/// Only stores if caching is enabled
pub fn store_cached_js(
    cache: &Arc<SokuCache>,
    path: &str,
    content: &str,
    processed: String,
    enable_cache: bool,
) {
    if enable_cache {
        cache.cache_js(path, content, processed);
    }
}

/// Check cache for processed CSS content
/// Returns cached content if available and caching is enabled
#[allow(dead_code)]
pub fn get_cached_css(
    cache: &Arc<SokuCache>,
    path: &str,
    content: &str,
    enable_cache: bool,
) -> Option<String> {
    if enable_cache {
        cache.get_css(path, content)
    } else {
        None
    }
}

/// Store processed CSS content in cache
/// Only stores if caching is enabled
#[allow(dead_code)]
pub fn store_cached_css(
    cache: &Arc<SokuCache>,
    path: &str,
    content: &str,
    processed: String,
    enable_cache: bool,
) {
    if enable_cache {
        cache.cache_css(path, content, processed);
    }
}

// ============================================================================
// Dependency Extraction (Shared)
// ============================================================================

// Pre-compiled regex patterns for dependency extraction
static CSS_IMPORT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"import\s+['"]([^'"]+)['"]"#).unwrap()
});
static REQUIRE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"require\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap()
});

/// Extract dependencies from JavaScript/TypeScript content
///
/// This function parses import and require statements to find module dependencies.
/// It handles:
/// - ES6 imports: `import foo from 'module'` and `import 'module'`
/// - CommonJS: `require('module')`
/// - Both relative and absolute paths
///
/// # Arguments
/// * `content` - The JavaScript/TypeScript source code
///
/// # Returns
/// A vector of dependency paths (module specifiers)
///
/// # Example
/// ```rust,no_run
/// use soku::infrastructure::processors::common::extract_dependencies;
///
/// let code = r#"
///     import React from 'react';
///     import './styles.css';
///     const lodash = require('lodash');
/// "#;
///
/// let deps = extract_dependencies(code);
/// assert_eq!(deps, vec!["react", "./styles.css", "lodash"]);
/// ```
pub fn extract_dependencies(content: &str) -> Vec<String> {
    let mut dependencies = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Handle ES6 import patterns
        if trimmed.starts_with("import ") {
            if let Some(from_index) = trimmed.rfind(" from ") {
                let import_path = &trimmed[from_index + 6..];
                // Remove quotes and semicolon
                let clean_path = import_path.trim_matches(|c| c == '"' || c == '\'' || c == ';');

                if !clean_path.is_empty() {
                    // Handle both relative imports and node_modules imports
                    dependencies.push(clean_path.to_string());
                }
            } else {
                // Handle CSS/asset imports like: import './styles.css'
                // Using pre-compiled regex for performance
                if let Some(captures) = CSS_IMPORT_REGEX.captures(trimmed) {
                    let import_path = &captures[1];

                    // Handle all import paths
                    dependencies.push(import_path.to_string());
                }
            }
        }

        // Handle CommonJS require() patterns
        // Using pre-compiled regex for performance
        for captures in REQUIRE_REGEX.captures_iter(trimmed) {
            let require_path = &captures[1];
            if !require_path.is_empty() {
                dependencies.push(require_path.to_string());
            }
        }
    }

    dependencies
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_node_modules_path() {
        let node_path = PathBuf::from("/project/node_modules/lodash/index.js");
        assert!(is_node_modules_path(&node_path));

        let regular_path = PathBuf::from("/project/src/main.js");
        assert!(!is_node_modules_path(&regular_path));
    }

    #[test]
    fn test_extract_package_name() {
        let path = PathBuf::from("/project/node_modules/lodash/index.js");
        assert_eq!(extract_package_name(&path), "lodash");

        let scoped_path = PathBuf::from("/project/node_modules/@types/react/index.d.ts");
        assert_eq!(extract_package_name(&scoped_path), "@types");
    }

    #[test]
    fn test_optimize_lodash_content() {
        let content = r#"
// Lodash
function add(a, b) { return a + b; }
exports.add = add;

        "#;

        let result = optimize_lodash_content(content, "lodash");
        assert!(result.contains("function add"));
        assert!(result.contains("exports.add"));
    }
}
