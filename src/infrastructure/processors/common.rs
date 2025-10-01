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
fn optimize_lodash_content(content: &str, package_name: &str) -> String {
    let mut result = Vec::new();
    result.push(format!("// Tree-shaken lodash from {}", package_name));

    for line in content.lines() {
        let trimmed = line.trim();

        // Keep function definitions and exports
        if trimmed.starts_with("function ")
            || trimmed.starts_with("exports.")
            || trimmed.starts_with("module.exports")
            || trimmed.starts_with("var ")
            || trimmed.contains("lodash")
        {
            result.push(line.to_string());
        }
        // Skip comments and empty lines
        else if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.is_empty() {
            continue;
        }
    }

    result.join("\n")
}

/// General optimization for other node_modules packages
fn optimize_general_node_module_content(content: &str) -> String {
    let mut result = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip overly verbose comments
        if trimmed.starts_with("/**") || trimmed.starts_with(" * @") {
            continue;
        }
        // Skip empty lines in node_modules (they add up!)
        if trimmed.is_empty() {
            continue;
        }

        result.push(line.to_string());
    }

    result.join("\n")
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
