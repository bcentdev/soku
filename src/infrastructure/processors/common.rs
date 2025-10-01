/// Shared functionality between JS processors
/// This module contains common code extracted from js_processor.rs and enhanced_js_processor.rs
/// to eliminate duplication and provide a single source of truth.

use std::path::Path;
use std::sync::Arc;
use once_cell::sync::Lazy;
use regex::Regex;
use crate::utils::performance::UltraCache;

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
    let mut brace_depth = 0;

    for line in content.lines() {
        let trimmed = line.trim();

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
    cache: &Arc<UltraCache>,
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
    cache: &Arc<UltraCache>,
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
pub fn get_cached_css(
    cache: &Arc<UltraCache>,
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
pub fn store_cached_css(
    cache: &Arc<UltraCache>,
    path: &str,
    content: &str,
    processed: String,
    enable_cache: bool,
) {
    if enable_cache {
        cache.cache_css(path, content, processed);
    }
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
