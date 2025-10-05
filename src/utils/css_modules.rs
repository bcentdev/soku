// CSS Modules support for Soku Bundler
// Provides scoped CSS with unique class names to avoid global namespace pollution

use crate::utils::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// CSS Module transformation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssModuleResult {
    /// Transformed CSS with scoped class names
    pub css: String,
    /// Mapping of original class names to scoped names
    pub exports: HashMap<String, String>,
}

/// CSS Modules processor
pub struct CssModulesProcessor {
    /// Pattern for matching CSS class selectors
    class_pattern: Regex,
    /// Pattern for matching CSS ID selectors
    id_pattern: Regex,
}

impl CssModulesProcessor {
    /// Create a new CSS Modules processor
    pub fn new() -> Self {
        Self {
            // Match class selectors like .myClass, .my-class, .myClass:hover
            class_pattern: Regex::new(r"\.([a-zA-Z_][a-zA-Z0-9_-]*)").unwrap(),
            // Match ID selectors like #myId
            id_pattern: Regex::new(r"#([a-zA-Z_][a-zA-Z0-9_-]*)").unwrap(),
        }
    }

    /// Check if a file should be processed as a CSS module
    pub fn is_css_module(path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".module.css"))
            .unwrap_or(false)
    }

    /// Process CSS content and generate scoped class names
    pub fn process(&self, content: &str, path: &Path) -> Result<CssModuleResult> {
        let module_name = self.get_module_name(path);
        let hash = self.generate_hash(content);

        let mut exports = HashMap::new();
        let mut transformed_css = content.to_string();

        // Extract and transform class names
        let class_names = self.extract_class_names(content);
        for class_name in class_names {
            let scoped_name = format!(
                "{}_{}_{}",
                module_name,
                class_name,
                &hash[..6] // Use first 6 chars of hash
            );

            exports.insert(class_name.clone(), scoped_name.clone());

            // Replace all occurrences of the class name
            // Use word boundaries to avoid replacing partial matches
            let pattern = format!(r"\.{}\b", regex::escape(&class_name));
            let re = Regex::new(&pattern).unwrap();
            transformed_css = re
                .replace_all(&transformed_css, format!(".{}", scoped_name))
                .to_string();
        }

        // Extract and transform ID names
        let id_names = self.extract_id_names(content);
        for id_name in id_names {
            let scoped_name = format!("{}_{}_{}", module_name, id_name, &hash[..6]);

            exports.insert(id_name.clone(), scoped_name.clone());

            // Replace all occurrences of the ID name
            let pattern = format!(r"#{}\b", regex::escape(&id_name));
            let re = Regex::new(&pattern).unwrap();
            transformed_css = re
                .replace_all(&transformed_css, format!("#{}", scoped_name))
                .to_string();
        }

        Ok(CssModuleResult {
            css: transformed_css,
            exports,
        })
    }

    /// Extract all class names from CSS content
    fn extract_class_names(&self, content: &str) -> Vec<String> {
        let mut class_names = Vec::new();

        for cap in self.class_pattern.captures_iter(content) {
            if let Some(class_name) = cap.get(1) {
                let name = class_name.as_str().to_string();
                if !class_names.contains(&name) {
                    class_names.push(name);
                }
            }
        }

        class_names
    }

    /// Extract all ID names from CSS content
    fn extract_id_names(&self, content: &str) -> Vec<String> {
        let mut id_names = Vec::new();

        for cap in self.id_pattern.captures_iter(content) {
            if let Some(id_name) = cap.get(1) {
                let name = id_name.as_str().to_string();
                if !id_names.contains(&name) {
                    id_names.push(name);
                }
            }
        }

        id_names
    }

    /// Generate module name from file path
    fn get_module_name(&self, path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| {
                // Remove .module suffix if present
                s.trim_end_matches(".module").replace(['-', '.'], "_")
            })
            .unwrap_or_else(|| "Module".to_string())
    }

    /// Generate a hash for the CSS content
    fn generate_hash(&self, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

impl Default for CssModulesProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_css_module() {
        assert!(CssModulesProcessor::is_css_module(&PathBuf::from(
            "Button.module.css"
        )));
        assert!(CssModulesProcessor::is_css_module(&PathBuf::from(
            "components/Card.module.css"
        )));
        assert!(!CssModulesProcessor::is_css_module(&PathBuf::from(
            "styles.css"
        )));
        assert!(!CssModulesProcessor::is_css_module(&PathBuf::from(
            "global.css"
        )));
    }

    #[test]
    fn test_extract_class_names() {
        let processor = CssModulesProcessor::new();
        let css = r"
            .button { color: blue; }
            .button:hover { color: red; }
            .card { padding: 10px; }
            .card-title { font-size: 20px; }
        ";

        let classes = processor.extract_class_names(css);
        assert_eq!(classes.len(), 3); // button, card, card-title
        assert!(classes.contains(&"button".to_string()));
        assert!(classes.contains(&"card".to_string()));
        assert!(classes.contains(&"card-title".to_string()));
    }

    #[test]
    fn test_extract_id_names() {
        let processor = CssModulesProcessor::new();
        let css = r"
            #header { color: blue; }
            #main-content { padding: 10px; }
        ";

        let ids = processor.extract_id_names(css);
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"header".to_string()));
        assert!(ids.contains(&"main-content".to_string()));
    }

    #[test]
    fn test_process_simple_css() {
        let processor = CssModulesProcessor::new();
        let css = ".button { color: blue; }";
        let path = PathBuf::from("Button.module.css");

        let result = processor.process(css, &path).unwrap();

        assert!(result.css.contains("Button_button_"));
        assert_eq!(result.exports.len(), 1);
        assert!(result.exports.contains_key("button"));
    }

    #[test]
    fn test_process_with_pseudo_selectors() {
        let processor = CssModulesProcessor::new();
        let css = r"
            .button { color: blue; }
            .button:hover { color: red; }
            .button:active { color: green; }
        ";
        let path = PathBuf::from("Button.module.css");

        let result = processor.process(css, &path).unwrap();

        // All instances of .button should be replaced
        assert_eq!(result.css.matches("Button_button_").count(), 3);
        assert_eq!(result.exports.len(), 1);
    }
}
