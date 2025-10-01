/// Shared functionality between JS processors
/// This module contains common code extracted from js_processor.rs and enhanced_js_processor.rs
/// to eliminate duplication and provide a single source of truth.

use std::path::Path;

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
