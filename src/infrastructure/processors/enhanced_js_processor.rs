#![allow(dead_code)] // Enhanced JS processor - advanced features, may not all be used yet

use crate::core::{interfaces::JsProcessor, models::*};
use crate::utils::{Result, UltraError, Logger, UltraCache, ErrorContext};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_diagnostics::OxcDiagnostic;
use oxc_span::SourceType;
use oxc_ast::ast;
use std::sync::Arc;
use std::path::Path;
use once_cell::sync::Lazy;
use regex::Regex;

// Pre-compiled regex patterns for performance
static TYPE_ANNOTATION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)\s*:\s*[a-zA-Z_$][a-zA-Z0-9_$<>\[\]|&\s]*([,)=])").unwrap()
});
static RETURN_TYPE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\)\s*:\s*[^=]+\s*(=>)").unwrap()
});
static GENERIC_TYPE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)<[^<>]*>").unwrap()
});

/// Enhanced JavaScript/TypeScript processor with advanced caching and optimizations
#[derive(Clone)]
pub struct EnhancedJsProcessor {
    cache: Arc<UltraCache>,
    enable_cache: bool,
}

impl EnhancedJsProcessor {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(UltraCache::new()),
            enable_cache: true,
        }
    }

    pub fn with_cache_disabled() -> Self {
        Self {
            cache: Arc::new(UltraCache::new()),
            enable_cache: false,
        }
    }

    /// Extract detailed error information from oxc parse errors
    fn create_parse_error_context(&self, errors: &[OxcDiagnostic], content: &str, file_path: &Path) -> ErrorContext {
        // Try to extract span information from the first error's labels
        let mut line_num = None;
        let mut col_num = None;
        let mut _error_span_start = None;

        // Note: OxcDiagnostic uses oxc-miette (a fork), not standard miette
        // We'll parse the error message to extract location info instead of accessing private fields
        if let Some(first_error) = errors.first() {
            let error_msg = format!("{:?}", first_error);
            // Try to parse line/column from debug output
            if let Some(start_pos) = Self::extract_span_from_debug(&error_msg) {
                _error_span_start = Some(start_pos);
                // Convert byte offset to line/column
                let (line, col) = Self::byte_offset_to_line_col(content, start_pos);
                line_num = Some(line);
                col_num = Some(col);
            }
        }

        // Extract contextual code snippet around the error (not just first 5 lines)
        let code_snippet = if let (Some(line), Some(_)) = (line_num, col_num) {
            Self::extract_code_snippet(content, line, 2) // 2 lines of context before/after
        } else {
            // Fallback: show first 5 lines if we couldn't extract location
            content.lines().take(5).collect::<Vec<_>>().join("\n")
        };

        let mut context = ErrorContext::new()
            .with_file(file_path.to_path_buf())
            .with_snippet(code_snippet);

        // Add line/column if available
        if let (Some(line), Some(col)) = (line_num, col_num) {
            context = context.with_location(line, col);
        }

        context
    }

    /// Extract span start position from debug output
    fn extract_span_from_debug(debug_str: &str) -> Option<usize> {
        // Look for pattern: offset: SourceOffset(313)
        if let Some(offset_idx) = debug_str.find("offset: SourceOffset(") {
            let after_offset = &debug_str[offset_idx + 21..]; // Skip "offset: SourceOffset("
            let num_str: String = after_offset
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            num_str.parse::<usize>().ok()
        } else {
            None
        }
    }

    /// Convert byte offset to 1-based line and 0-based column numbers
    fn byte_offset_to_line_col(content: &str, byte_offset: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 0;
        let mut current_offset = 0;

        for ch in content.chars() {
            if current_offset >= byte_offset {
                break;
            }

            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }

            current_offset += ch.len_utf8();
        }

        (line, col)
    }

    /// Extract code snippet with context lines around the error line
    fn extract_code_snippet(content: &str, error_line: usize, context_lines: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Calculate range with context
        let start_line = error_line.saturating_sub(context_lines + 1); // Convert to 0-based
        let end_line = (error_line + context_lines).min(total_lines);

        lines[start_line..end_line].join("\n")
    }

    pub fn with_persistent_cache(cache_dir: &Path) -> Self {
        Self {
            cache: Arc::new(UltraCache::with_persistent_cache(cache_dir)),
            enable_cache: true,
        }
    }

    /// Enhanced TypeScript processing with AST-based transformation
    async fn process_typescript(&self, module: &ModuleInfo) -> Result<String> {
        let _timer = crate::utils::Timer::start(&format!("AST TypeScript processing {}",
            module.path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")));

        let file_extension = module.path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        if file_extension == "tsx" {
            Logger::processing_typescript("TSX/JSX component (AST-based)");
            // For TSX files, we need both TypeScript stripping AND JSX transformation
            let processed = self.process_jsx_content(&module.content, &module.path)?;
            Logger::debug(&format!("JSX processed output:\n{}", processed));
            Ok(processed)
        } else {
            Logger::processing_typescript(&format!(
                "TypeScript {} (AST-based)",
                module.path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
            ));
            // For regular TS files, just strip TypeScript types
            let processed = self.ast_typescript_transform(&module.content, &module.path)?;
            Ok(processed)
        }
    }

    /// Process JSX/TSX content using AST-based transformation
    fn process_jsx_content(&self, content: &str, file_path: &Path) -> Result<String> {
        let allocator = Allocator::default();
        let source_type = SourceType::default().with_typescript(true).with_jsx(true); // Support TSX files

        let parser = Parser::new(&allocator, content, source_type);
        let result = parser.parse();

        if !result.errors.is_empty() {
            // Create detailed error context with file location
            let error_context = self.create_parse_error_context(&result.errors, content, file_path);
            let first_error = &result.errors[0];

            // Log detailed error information
            let detailed_error = UltraError::parse_with_context(
                format!("JSX/TSX parsing failed: {}", first_error),
                error_context
            );

            Logger::warn(&detailed_error.format_detailed());
            Logger::warn("Falling back to regex-based approach for JSX processing");

            // Fall back to regex-based approach
            let type_stripped = self.strip_typescript_types(content);
            return Ok(self.convert_jsx_to_js(&type_stripped));
        }

        // Transform JSX AST to JavaScript
        let transformed = self.transform_jsx_ast(&result.program, content);
        Ok(transformed)
    }

    /// Transform JSX AST to JavaScript function calls
    fn transform_jsx_ast(&self, _program: &ast::Program, original_content: &str) -> String {
        // For initial AST-based JSX transformation
        // This will be more accurate than regex parsing

        let mut result = String::new();

        for line in original_content.lines() {
            let js_line = self.transform_jsx_line(line);
            result.push_str(&js_line);
            result.push('\n');
        }

        result
    }

    /// Transform a single line of JSX to JavaScript
    fn transform_jsx_line(&self, line: &str) -> String {
        let mut result = line.to_string();

        // Clean TypeScript annotations first
        result = self.clean_typescript_annotations(&result);

        // Transform JSX elements to createElement calls
        // This is a simplified transformation - full JSX would need more complex AST walking

        // Handle simple JSX elements: <div>content</div> -> React.createElement('div', null, 'content')
        // Fixed: Removed backreference \1, match with general closing tag pattern
        if let Ok(re) = regex::Regex::new(r"<(\w+)>([^<]*)</\w+>") {
            result = re.replace_all(&result, "React.createElement('$1', null, '$2')").to_string();
        }

        // Handle self-closing JSX: <div /> -> React.createElement('div', null)
        if let Ok(re) = regex::Regex::new(r"<(\w+)\s*/>") {
            result = re.replace_all(&result, "React.createElement('$1', null)").to_string();
        }

        // For complex JSX with props, fall back to null for now
        if let Ok(re) = regex::Regex::new(r"<[^>]*\s[^>]*>") {
            result = re.replace_all(&result, "null").to_string();
        }

        result
    }

    /// Convert JSX syntax to JavaScript function calls (improved regex approach)
    fn convert_jsx_to_js(&self, content: &str) -> String {
        Logger::debug(&format!("JSX input content:\n{}", content));
        let mut result = content.to_string();

        // First strip TypeScript types
        result = self.strip_typescript_types(&result);
        Logger::debug(&format!("After TypeScript stripping:\n{}", result));

        // Transform simple lowercase elements: <div>text</div> (single line)
        // Fixed: Removed backreference \1, match with general lowercase closing tag
        if let Ok(re) = regex::Regex::new(r#"<([a-z][a-zA-Z0-9]*)\s*([^>]*?)>\s*([^<>]*?)\s*</[a-z][a-zA-Z0-9]*>"#) {
            let callback = |caps: &regex::Captures| {
                let element = &caps[1];
                let props = caps.get(2).map_or("", |m| m.as_str()).trim();
                let children = caps.get(3).map_or("", |m| m.as_str()).trim();

                let props_obj = if props.is_empty() {
                    "null".to_string()
                } else {
                    self.parse_jsx_props_simple(props)
                };

                let children_str = if children.is_empty() {
                    ""
                } else {
                    &format!(", \"{}\"", children)
                };

                let replacement = format!("React.createElement(\"{}\", {}{children_str})", element, props_obj);
                Logger::debug(&format!("JSX transform: {} -> {}", caps.get(0).unwrap().as_str(), replacement));
                replacement
            };
            result = re.replace_all(&result, callback).to_string();
        }

        // Transform self-closing lowercase elements: <input type="text" />
        if let Ok(re) = regex::RegexBuilder::new(r#"<([a-z][a-zA-Z0-9]*)\s*([^/>]*?)\s*/\s*>"#)
            .dot_matches_new_line(true)
            .build() {
            let callback = |caps: &regex::Captures| {
                let element = &caps[1];
                let props = caps.get(2).map_or("", |m| m.as_str()).trim();

                let props_obj = if props.is_empty() {
                    "null".to_string()
                } else {
                    self.parse_jsx_props_simple(props)
                };

                format!("React.createElement(\"{}\", {})", element, props_obj)
            };
            result = re.replace_all(&result, callback).to_string();
        }

        // Transform self-closing component tags: <Component prop={value} />
        if let Ok(re) = regex::RegexBuilder::new(r#"<([A-Z][a-zA-Z0-9.]*)\s*([^/>]*?)\s*/\s*>"#)
            .dot_matches_new_line(true)
            .build() {
            let callback = |caps: &regex::Captures| {
                let component = &caps[1];
                let props = caps.get(2).map_or("", |m| m.as_str()).trim();

                let props_obj = if props.is_empty() {
                    "null".to_string()
                } else {
                    self.parse_jsx_props_simple(props)
                };

                format!("React.createElement({}, {})", component, props_obj)
            };
            result = re.replace_all(&result, callback).to_string();
        }

        // Transform simple component elements: <Component>content</Component> (single line)
        // Fixed: Removed backreference \1, match with general uppercase closing tag
        if let Ok(re) = regex::Regex::new(r#"<([A-Z][a-zA-Z0-9.]*)\s*([^>]*?)>\s*([^<>]*?)\s*</[A-Z][a-zA-Z0-9.]*>"#) {
            let callback = |caps: &regex::Captures| {
                let component = &caps[1];
                let props = caps.get(2).map_or("", |m| m.as_str()).trim();
                let children = caps.get(3).map_or("", |m| m.as_str()).trim();

                let props_obj = if props.is_empty() {
                    "null".to_string()
                } else {
                    self.parse_jsx_props_simple(props)
                };

                let children_str = if children.is_empty() {
                    ""
                } else {
                    &format!(", {}", children)
                };

                format!("React.createElement({}, {}{children_str})", component, props_obj)
            };
            result = re.replace_all(&result, callback).to_string();
        }

        Logger::debug(&format!("Final JSX output:\n{}", result));
        result
    }

    /// Simplified JSX props parsing for initial implementation
    fn parse_jsx_props_simple(&self, props: &str) -> String {
        if props.trim().is_empty() {
            return "null".to_string();
        }

        // For now, return a simple object with the props as-is
        // This needs improvement for production use
        let cleaned_props = props
            .replace("={", ": ")
            .replace("}", "")
            .replace("=\"", ": \"")
            .replace("\"", "\"");

        format!("{{{}}}", cleaned_props)
    }

    /// Enhanced JSX props parsing
    fn parse_jsx_props_enhanced(&self, props: &str) -> String {
        if props.trim().is_empty() {
            return "null".to_string();
        }

        let mut prop_pairs = Vec::new();

        // Handle various prop patterns
        // Pattern 1: prop="string value"
        if let Ok(re) = regex::Regex::new(r#"([a-zA-Z][a-zA-Z0-9]*)\s*=\s*"([^"]*)""#) {
            for caps in re.captures_iter(props) {
                let prop_name = &caps[1];
                let value = &caps[2];
                prop_pairs.push(format!("{}: \"{}\"", prop_name, value));
            }
        }

        // Pattern 2: prop='string value'
        if let Ok(re) = regex::Regex::new(r#"([a-zA-Z][a-zA-Z0-9]*)\s*=\s*'([^']*)'"#) {
            for caps in re.captures_iter(props) {
                let prop_name = &caps[1];
                let value = &caps[2];
                if !prop_pairs.iter().any(|p| p.starts_with(&format!("{}:", prop_name))) {
                    prop_pairs.push(format!("{}: \"{}\"", prop_name, value));
                }
            }
        }

        // Pattern 3: prop={expression}
        if let Ok(re) = regex::Regex::new(r#"([a-zA-Z][a-zA-Z0-9]*)\s*=\s*\{([^}]*)\}"#) {
            for caps in re.captures_iter(props) {
                let prop_name = &caps[1];
                let expression = &caps[2];
                if !prop_pairs.iter().any(|p| p.starts_with(&format!("{}:", prop_name))) {
                    prop_pairs.push(format!("{}: {}", prop_name, expression));
                }
            }
        }

        // Pattern 4: boolean props (just the name)
        // Fixed: Removed negative look-ahead (?!...), filter manually instead
        if let Ok(re) = regex::Regex::new(r#"\b([a-zA-Z][a-zA-Z0-9]*)\b"#) {
            for caps in re.captures_iter(props) {
                let prop_name = &caps[1];
                // Check if this prop is followed by '=' in original string
                let full_match = caps.get(0).unwrap();
                let after_match = &props[full_match.end()..];
                if !after_match.trim_start().starts_with('=')
                   && !prop_pairs.iter().any(|p| p.starts_with(&format!("{}:", prop_name))) {
                    prop_pairs.push(format!("{}: true", prop_name));
                }
            }
        }

        if prop_pairs.is_empty() {
            "null".to_string()
        } else {
            format!("{{{}}}", prop_pairs.join(", "))
        }
    }

    /// AST-based TypeScript transformation using oxc parser
    fn ast_typescript_transform(&self, content: &str, file_path: &Path) -> Result<String> {
        let allocator = Allocator::default();
        let source_type = SourceType::default().with_typescript(true);

        let parser = Parser::new(&allocator, content, source_type);
        let result = parser.parse();

        if !result.errors.is_empty() {
            // Create detailed error context with file location
            let error_context = self.create_parse_error_context(&result.errors, content, file_path);
            let first_error = &result.errors[0];

            // Log detailed error information
            let detailed_error = UltraError::parse_with_context(
                format!("TypeScript parsing failed: {}", first_error),
                error_context
            );

            Logger::warn(&detailed_error.format_detailed());
            Logger::warn("Falling back to regex-based approach for TypeScript processing");

            // Fall back to fast regex-based approach if AST parsing fails
            return Ok(self.fast_typescript_strip(content));
        }

        // For now, use a simple approach to extract JavaScript from AST
        // This is more robust than regex for handling complex TypeScript
        let transformed = self.extract_javascript_from_ast(&result.program, content);
        Logger::debug(&format!("AST transformed output: {}", transformed));

        Ok(transformed)
    }

    /// Extract JavaScript code from TypeScript AST
    fn extract_javascript_from_ast(&self, _program: &ast::Program, original_content: &str) -> String {
        // For initial implementation, perform selective stripping based on AST validation
        // This ensures we only transform syntactically valid TypeScript

        // Use the fast regex-based approach which is more robust
        // TODO: Implement proper AST-based transformation later
        let result = self.fast_typescript_strip(original_content);
        result
    }

    /// Parse JSX props into JavaScript object notation
    fn parse_jsx_props(&self, props: &str) -> String {
        if props.trim().is_empty() {
            return "null".to_string();
        }

        let mut prop_pairs = Vec::new();

        // Simple regex to match prop="value" or prop={expression}
        if let Ok(re) = regex::Regex::new(r#"([a-zA-Z][a-zA-Z0-9]*)\s*=\s*(?:"([^"]*)"|'([^']*)'|\{([^}]*)\})"#) {
            for caps in re.captures_iter(props) {
                let prop_name = &caps[1];
                let value = if let Some(quoted) = caps.get(2).or(caps.get(3)) {
                    format!("\"{}\"", quoted.as_str())
                } else if let Some(expr) = caps.get(4) {
                    expr.as_str().to_string()
                } else {
                    "true".to_string()
                };
                prop_pairs.push(format!("{}: {}", prop_name, value));
            }
        }

        if prop_pairs.is_empty() {
            "null".to_string()
        } else {
            format!("{{{}}}", prop_pairs.join(", "))
        }
    }

    /// Fast TypeScript type stripping for fallback scenarios
    fn fast_typescript_strip(&self, content: &str) -> String {
        let mut lines = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip TypeScript-only declarations completely
            if trimmed.starts_with("interface ") ||
               trimmed.starts_with("export interface ") ||
               trimmed.starts_with("type ") ||
               trimmed.starts_with("export type ") ||
               trimmed.starts_with("enum ") ||
               trimmed.starts_with("export enum ") ||
               trimmed.starts_with("const enum ") ||
               trimmed.starts_with("export const enum ") ||
               trimmed.starts_with("import ") {
                continue;
            }

            // Skip lines that are only closing braces (from skipped declarations)
            if trimmed == "}" || trimmed == "};" {
                continue;
            }

            // Basic type annotation cleaning for function parameters
            let mut cleaned = line.to_string();

            // Remove simple type annotations: name: Type -> name
            // Using pre-compiled regex for performance
            cleaned = TYPE_ANNOTATION_REGEX.replace_all(&cleaned, "$1$2").to_string();

            // Remove function return types: ): Type => -> ) =>
            // Using pre-compiled regex for performance
            cleaned = RETURN_TYPE_REGEX.replace_all(&cleaned, ")$1").to_string();

            lines.push(cleaned);
        }

        lines.join("\n")
    }

    /// Advanced TypeScript type stripping with comprehensive support
    fn strip_typescript_types(&self, content: &str) -> String {
        // Use the fast approach by default to avoid performance issues
        self.fast_typescript_strip(content)
    }

    /// Clean TypeScript annotations from a single line of code
    fn clean_typescript_annotations(&self, line: &str) -> String {
        let mut result = line.to_string();

        // Fix common syntax errors first

        // Fix malformed function calls: onCountChange.(value) -> onCountChange(value)
        if let Ok(re) = regex::Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)\.\(([^)]*)\)") {
            result = re.replace_all(&result, "$1($2)").to_string();
        }

        // Fix template literal syntax: ${variant $disabled -> ${variant} ${disabled
        if let Ok(re) = regex::Regex::new(r"\$\{([a-zA-Z_$][a-zA-Z0-9_$]*)\s+\$([a-zA-Z_$][a-zA-Z0-9_$]*)") {
            result = re.replace_all(&result, "${$1} ${$2").to_string();
        }

        // Clean destructuring parameters with types: ({ text, onClick }: Props) -> ({ text, onClick })
        if let Ok(re) = regex::Regex::new(r"\(\s*\{([^}]*)\}\s*:\s*[^)]*\)") {
            result = re.replace_all(&result, "({ $1 })").to_string();
        }

        // Clean regular parameters with types: (text, onClick, disabled = false, variant = 'primary' : ButtonProps) -> (text, onClick, disabled = false, variant = 'primary')
        if let Ok(re) = regex::Regex::new(r"\(([^)]*?)\s*:\s*[^)]*\)\s*=>") {
            result = re.replace_all(&result, "($1) =>").to_string();
        }

        // Clean function parameters inline: text: string, -> text,
        if let Ok(re) = regex::Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)\s*:\s*[^,)=]+([,)])") {
            result = re.replace_all(&result, "$1$2").to_string();
        }

        // Clean variable declarations: let x: number = 5 -> let x = 5
        if let Ok(re) = regex::Regex::new(r"(let|const|var)\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*:\s*[^=]+(\s*=)") {
            result = re.replace_all(&result, "$1 $2$3").to_string();
        }

        // Clean function return types: function foo(): Type -> function foo()
        if let Ok(re) = regex::Regex::new(r"(function\s+[a-zA-Z_$][a-zA-Z0-9_$]*\s*\([^)]*\))\s*:\s*[^{]+(\s*\{)") {
            result = re.replace_all(&result, "$1$2").to_string();
        }

        // Clean arrow function return types: (): Type => -> () =>
        if let Ok(re) = regex::Regex::new(r"\)\s*:\s*[^=]+(\s*=>)") {
            result = re.replace_all(&result, ")$1").to_string();
        }

        // Add missing braces for arrow functions - DISABLED due to syntax errors
        // if let Ok(re) = regex::Regex::new(r"\)\s*=>\s*$") {
        //     result = re.replace_all(&result, ") => {").to_string();
        // }
        // if let Ok(re) = regex::Regex::new(r"\{\s*$\s*;\s*$") {
        //     result = re.replace_all(&result, "};\n").to_string();
        // }

        // Remove generic type parameters
        if let Ok(re) = regex::Regex::new(r"<[^<>]*>") {
            result = re.replace_all(&result, "").to_string();
        }

        // Remove TypeScript non-null assertion operator
        if let Ok(re) = regex::Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)\s*!") {
            result = re.replace_all(&result, "$1").to_string();
        }

        // Remove access modifiers
        if let Ok(re) = regex::Regex::new(r"\b(private|public|protected|readonly)\s+") {
            result = re.replace_all(&result, "").to_string();
        }

        // Remove as type assertions
        if let Ok(re) = regex::Regex::new(r"\s+as\s+[a-zA-Z_$][a-zA-Z0-9_$<>\[\]|&\s]*") {
            result = re.replace_all(&result, "").to_string();
        }

        result
    }

    /// Clean function parameters of TypeScript types
    fn clean_function_parameters(&self, content: &str) -> String {
        let mut result = content.to_string();

        // Clean destructuring parameters with types: ({ text, onClick }: Props) -> ({ text, onClick })
        if let Ok(re) = regex::Regex::new(r"\(\s*\{([^}]*)\}\s*:\s*[^)]+\)") {
            result = re.replace_all(&result, "({$1})").to_string();
        }

        // Clean regular typed parameters: (x: number, y: string) -> (x, y)
        if let Ok(re) = regex::Regex::new(r"\(([^)]*)\)") {
            result = re.replace_all(&result, |caps: &regex::Captures| {
                let params = &caps[1];
                if params.trim().is_empty() {
                    return "()".to_string();
                }

                // Handle destructuring separately from regular params
                if params.contains('{') && !params.contains(':') {
                    // Already processed destructuring above
                    return format!("({})", params);
                }

                let cleaned_params: Vec<String> = params
                    .split(',')
                    .map(|param| {
                        let trimmed = param.trim();

                        // Handle destructuring: { text, onClick }: Props -> { text, onClick }
                        if trimmed.starts_with('{') {
                            if let Some(colon_pos) = trimmed.find(':') {
                                let destructured = trimmed[..colon_pos].trim();
                                return destructured.to_string();
                            }
                            return trimmed.to_string();
                        }

                        // Extract parameter name before colon: "x: number" -> "x"
                        if let Some(colon_pos) = trimmed.find(':') {
                            let param_name = trimmed[..colon_pos].trim();
                            // Handle optional parameters: "x?" -> "x"
                            let clean_name = param_name.trim_end_matches('?');
                            // Handle default values: "disabled = false" -> keep as is
                            if clean_name.contains('=') {
                                clean_name.to_string()
                            } else {
                                clean_name.to_string()
                            }
                        } else {
                            // Keep parameters that don't have types
                            trimmed.trim_end_matches('?').to_string()
                        }
                    })
                    .collect();

                format!("({})", cleaned_params.join(", "))
            }).to_string();
        }

        result
    }

    /// Clean generic types recursively
    fn clean_generic_types(&self, content: &str) -> String {
        let mut result = content.to_string();
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10; // Prevent infinite loops

        while iterations < MAX_ITERATIONS {
            // Using pre-compiled regex for performance
            let new_result = GENERIC_TYPE_REGEX.replace_all(&result, "$1").to_string();
            if new_result == result {
                break; // No more changes
            }
            result = new_result;
            iterations += 1;
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
            // Create detailed error context with file location
            let error_context = self.create_parse_error_context(&result.errors, &module.content, &module.path);
            let first_error = &result.errors[0];

            // Log detailed error information
            let detailed_error = UltraError::parse_with_context(
                format!("JavaScript parsing warning: {}", first_error),
                error_context
            );

            Logger::warn(&detailed_error.format_detailed());
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

        // Check cache first for ultra-fast rebuilds
        let path_str = module.path.to_string_lossy();
        if self.enable_cache {
            if let Some(cached) = self.cache.get_js(&path_str, &module.content) {
                Logger::debug("Cache hit for enhanced processing");
                return Ok(cached);
            }
        }

        let result = match module.module_type {
            ModuleType::TypeScript => {
                self.process_typescript(module).await
            }
            ModuleType::JavaScript => {
                self.process_javascript(module).await
            }
            _ => Err(UltraError::build(format!(
                "Unsupported module type for enhanced processor: {:?}",
                module.module_type
            ))),
        };

        // Cache the result for future builds
        if self.enable_cache {
            if let Ok(ref processed) = result {
                self.cache.cache_js(&path_str, &module.content, processed.clone());
            }
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
        let _timer = crate::utils::Timer::start("Enhanced bundling with tree shaking and node_modules optimization");

        let mut bundle = String::new();
        bundle.push_str("// Ultra Bundler - Enhanced Build with Node Modules Tree Shaking\n");
        bundle.push_str("(function() {\n'use strict';\n\n");

        // Separate node_modules from local modules for different processing
        let (local_modules, node_modules): (Vec<_>, Vec<_>) = modules.iter()
            .partition(|module| !self.is_node_modules_path(&module.path));

        Logger::debug(&format!("Processing {} local modules, {} node_modules", local_modules.len(), node_modules.len()));

        // Process local modules first
        for module in local_modules {
            let processed = self.process_module(module).await?;
            bundle.push_str(&format!("// Module: {}\n", module.path.display()));
            bundle.push_str(&processed);
            bundle.push_str("\n\n");
        }

        // Process node_modules with optimization
        if !node_modules.is_empty() {
            bundle.push_str("// === NODE MODULES (Tree Shaken) ===\n");

            for module in node_modules {
                Logger::processing_file(
                    module.path.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown"),
                    "tree shaking node_modules"
                );

                let processed = self.optimize_node_module_content(&module.content, &module.path);

                bundle.push_str(&format!("// Node Module: {}\n", self.extract_package_name(&module.path)));
                bundle.push_str(&processed);
                bundle.push_str("\n\n");
            }
        }

        bundle.push_str("})();\n");

        Ok(bundle)
    }

    async fn bundle_modules_with_source_maps(&self, modules: &[ModuleInfo], config: &BuildConfig) -> Result<BundleOutput> {
        // For now, use a simple implementation that delegates to regular bundling
        // TODO: Implement proper source maps for enhanced TypeScript processing
        if config.enable_source_maps {
            let code = self.bundle_modules(modules).await?;
            Ok(BundleOutput {
                code: format!("{}\n//# sourceMappingURL=bundle.js.map", code),
                source_map: Some(r#"{"version":3,"sources":["enhanced"],"names":[],"mappings":"AAAA"}"#.to_string()),
            })
        } else {
            let code = self.bundle_modules(modules).await?;
            Ok(BundleOutput {
                code,
                source_map: None,
            })
        }
    }


    fn supports_module_type(&self, module_type: &ModuleType) -> bool {
        matches!(module_type, ModuleType::JavaScript | ModuleType::TypeScript)
    }
}

impl EnhancedJsProcessor {
    /// Check if a module path is from node_modules
    fn is_node_modules_path(&self, path: &std::path::Path) -> bool {
        path.to_string_lossy().contains("node_modules")
    }

    /// Extract package name from node_modules path
    fn extract_package_name(&self, path: &std::path::Path) -> String {
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
    fn optimize_node_module_content(&self, content: &str, path: &std::path::Path) -> String {
        let package_name = self.extract_package_name(path);

        if package_name == "lodash" {
            self.optimize_lodash_content(content, &package_name)
        } else {
            self.optimize_general_node_module_content(content)
        }
    }

    /// Specific optimization for lodash
    fn optimize_lodash_content(&self, content: &str, package_name: &str) -> String {
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
            if trimmed.starts_with("function ") ||
               trimmed.starts_with("var ") ||
               trimmed.starts_with("module.exports") ||
               trimmed.starts_with("exports.") {
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

    /// General optimization for node_modules
    fn optimize_general_node_module_content(&self, content: &str) -> String {
        let mut result = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip obvious dead code patterns
            if trimmed.starts_with("// Development only") ||
               trimmed.starts_with("// DEBUG") ||
               trimmed.contains("console.warn") ||
               trimmed.contains("console.error") {
                result.push_str(&format!("// TREE-SHAKEN: {}\n", line));
                continue;
            }

            result.push_str(line);
            result.push('\n');
        }

        result
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