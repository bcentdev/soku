// Custom Transformers - User-defined code transformations
#![allow(dead_code)] // Public API - used via examples and external integrations

use crate::core::models::ModuleInfo;
use crate::utils::{Result, UltraError, Plugin};
use async_trait::async_trait;
use regex::Regex;
use std::sync::Arc;

/// Transformer type - defines how the transformation is applied
#[derive(Clone)]
pub enum TransformerType {
    /// Simple regex-based replacement
    Regex { pattern: String, replacement: String },
    /// Custom function transformer
    Function(Arc<dyn Fn(&str) -> Result<String> + Send + Sync>),
    /// Conditional transformer with file pattern matching
    Conditional {
        file_pattern: String,
        transformer: Box<TransformerType>,
    },
}

impl std::fmt::Debug for TransformerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransformerType::Regex { pattern, replacement } => {
                f.debug_struct("Regex")
                    .field("pattern", pattern)
                    .field("replacement", replacement)
                    .finish()
            }
            TransformerType::Function(_) => {
                f.debug_tuple("Function").field(&"<closure>").finish()
            }
            TransformerType::Conditional { file_pattern, transformer } => {
                f.debug_struct("Conditional")
                    .field("file_pattern", file_pattern)
                    .field("transformer", transformer)
                    .finish()
            }
        }
    }
}

/// Custom transformer configuration
#[derive(Debug, Clone)]
pub struct CustomTransformer {
    pub name: String,
    pub transformer_type: TransformerType,
    pub enabled: bool,
}

impl CustomTransformer {
    /// Create a new regex-based transformer
    pub fn regex(name: impl Into<String>, pattern: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transformer_type: TransformerType::Regex {
                pattern: pattern.into(),
                replacement: replacement.into(),
            },
            enabled: true,
        }
    }

    /// Create a new function-based transformer
    pub fn function<F>(name: impl Into<String>, func: F) -> Self
    where
        F: Fn(&str) -> Result<String> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            transformer_type: TransformerType::Function(Arc::new(func)),
            enabled: true,
        }
    }

    /// Create a conditional transformer (only applies to matching files)
    pub fn conditional(name: impl Into<String>, file_pattern: impl Into<String>, transformer_type: TransformerType) -> Self {
        Self {
            name: name.into(),
            transformer_type: TransformerType::Conditional {
                file_pattern: file_pattern.into(),
                transformer: Box::new(transformer_type),
            },
            enabled: true,
        }
    }

    /// Enable or disable this transformer
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Apply the transformation to code
    pub fn transform(&self, code: &str, file_path: Option<&str>) -> Result<String> {
        if !self.enabled {
            return Ok(code.to_string());
        }

        match &self.transformer_type {
            TransformerType::Regex { pattern, replacement } => {
                let re = Regex::new(pattern)
                    .map_err(|e| UltraError::Build {
                        message: format!("Invalid regex pattern in transformer '{}': {}", self.name, e),
                        context: None,
                    })?;
                Ok(re.replace_all(code, replacement.as_str()).to_string())
            }
            TransformerType::Function(func) => {
                func(code)
            }
            TransformerType::Conditional { file_pattern, transformer } => {
                if let Some(path) = file_path {
                    // Simple pattern matching (could be enhanced with glob patterns)
                    if path.contains(file_pattern) {
                        let inner = CustomTransformer {
                            name: self.name.clone(),
                            transformer_type: (**transformer).clone(),
                            enabled: self.enabled,
                        };
                        inner.transform(code, file_path)
                    } else {
                        Ok(code.to_string())
                    }
                } else {
                    Ok(code.to_string())
                }
            }
        }
    }
}

/// Transformer builder for fluent API
pub struct TransformerBuilder {
    transformers: Vec<CustomTransformer>,
}

impl TransformerBuilder {
    pub fn new() -> Self {
        Self {
            transformers: Vec::new(),
        }
    }

    /// Add a regex-based transformer
    pub fn add_regex(mut self, name: impl Into<String>, pattern: impl Into<String>, replacement: impl Into<String>) -> Self {
        self.transformers.push(CustomTransformer::regex(name, pattern, replacement));
        self
    }

    /// Add a function-based transformer
    pub fn add_function<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(&str) -> Result<String> + Send + Sync + 'static,
    {
        self.transformers.push(CustomTransformer::function(name, func));
        self
    }

    /// Add a conditional transformer
    pub fn add_conditional(mut self, name: impl Into<String>, file_pattern: impl Into<String>, transformer_type: TransformerType) -> Self {
        self.transformers.push(CustomTransformer::conditional(name, file_pattern, transformer_type));
        self
    }

    /// Add a custom transformer
    pub fn add(mut self, transformer: CustomTransformer) -> Self {
        self.transformers.push(transformer);
        self
    }

    /// Build the transformer chain
    pub fn build(self) -> TransformerChain {
        TransformerChain {
            transformers: self.transformers,
        }
    }
}

impl Default for TransformerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Chain of transformers that are applied sequentially
#[derive(Debug, Clone)]
pub struct TransformerChain {
    transformers: Vec<CustomTransformer>,
}

impl TransformerChain {
    pub fn new() -> Self {
        Self {
            transformers: Vec::new(),
        }
    }

    /// Add a transformer to the chain
    pub fn add(&mut self, transformer: CustomTransformer) {
        self.transformers.push(transformer);
    }

    /// Apply all transformers in sequence
    pub fn transform(&self, mut code: String, file_path: Option<&str>) -> Result<String> {
        for transformer in &self.transformers {
            code = transformer.transform(&code, file_path)?;
        }
        Ok(code)
    }

    /// Get the number of transformers
    pub fn len(&self) -> usize {
        self.transformers.len()
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.transformers.is_empty()
    }
}

impl Default for TransformerChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin wrapper for transformer chain
pub struct TransformerPlugin {
    name: String,
    chain: TransformerChain,
}

impl TransformerPlugin {
    pub fn new(name: impl Into<String>, chain: TransformerChain) -> Self {
        Self {
            name: name.into(),
            chain,
        }
    }

    /// Create from builder
    pub fn from_builder(name: impl Into<String>, builder: TransformerBuilder) -> Self {
        Self::new(name, builder.build())
    }
}

#[async_trait]
impl Plugin for TransformerPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn transform_code(&self, module: &ModuleInfo, code: String) -> Result<String> {
        let file_path = module.path.to_str();
        self.chain.transform(code, file_path)
    }
}

/// Built-in common transformers
pub struct BuiltInTransformers;

impl BuiltInTransformers {
    /// Remove console.log statements
    pub fn remove_console_logs() -> CustomTransformer {
        CustomTransformer::regex(
            "remove-console-logs",
            r"console\.(log|debug|info|warn|error)\([^)]*\);?\s*",
            ""
        )
    }

    /// Remove comments (single-line and multi-line)
    pub fn remove_comments() -> CustomTransformer {
        CustomTransformer::function("remove-comments", |code| {
            // Simple comment removal (not AST-based, may have edge cases)
            let code = Regex::new(r"//.*$")
                .unwrap()
                .replace_all(code, "");
            let code = Regex::new(r"/\*[\s\S]*?\*/")
                .unwrap()
                .replace_all(&code, "");
            Ok(code.to_string())
        })
    }

    /// Remove debugger statements
    pub fn remove_debugger() -> CustomTransformer {
        CustomTransformer::regex(
            "remove-debugger",
            r"debugger;?\s*",
            ""
        )
    }

    /// Replace string literals (case-sensitive)
    pub fn replace_string(from: impl Into<String>, to: impl Into<String>) -> CustomTransformer {
        let from = from.into();
        let to = to.into();
        CustomTransformer::function(
            format!("replace-string-{}", from),
            move |code| Ok(code.replace(&from, &to))
        )
    }

    /// Transform arrow functions to regular functions (simple cases)
    pub fn arrow_to_function() -> CustomTransformer {
        CustomTransformer::regex(
            "arrow-to-function",
            r"const\s+(\w+)\s*=\s*\(([^)]*)\)\s*=>\s*\{",
            "function $1($2) {"
        )
    }

    /// Add 'use strict' directive
    pub fn add_use_strict() -> CustomTransformer {
        CustomTransformer::function("add-use-strict", |code| {
            if !code.contains("'use strict'") && !code.contains("\"use strict\"") {
                Ok(format!("'use strict';\n{}", code))
            } else {
                Ok(code.to_string())
            }
        })
    }

    /// Conditional transformer for test files only
    pub fn test_only(transformer: TransformerType) -> CustomTransformer {
        CustomTransformer::conditional("test-only", ".test.", transformer)
    }

    /// Conditional transformer for production only
    pub fn production_only(transformer: TransformerType) -> CustomTransformer {
        CustomTransformer::function("production-only", move |code| {
            if std::env::var("NODE_ENV").unwrap_or_default() == "production" {
                let temp_transformer = CustomTransformer {
                    name: "production".to_string(),
                    transformer_type: transformer.clone(),
                    enabled: true,
                };
                temp_transformer.transform(code, None)
            } else {
                Ok(code.to_string())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_transformer() {
        let transformer = CustomTransformer::regex("test", "foo", "bar");
        let result = transformer.transform("const x = foo;", None).unwrap();
        assert_eq!(result, "const x = bar;");
    }

    #[test]
    fn test_function_transformer() {
        let transformer = CustomTransformer::function("uppercase", |code| {
            Ok(code.to_uppercase())
        });
        let result = transformer.transform("hello", None).unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_conditional_transformer() {
        let inner = TransformerType::Regex {
            pattern: "foo".to_string(),
            replacement: "bar".to_string(),
        };
        let transformer = CustomTransformer::conditional("test", ".test.", inner);

        // Should transform test files
        let result = transformer.transform("foo", Some("file.test.js")).unwrap();
        assert_eq!(result, "bar");

        // Should not transform non-test files
        let result = transformer.transform("foo", Some("file.js")).unwrap();
        assert_eq!(result, "foo");
    }

    #[test]
    fn test_disabled_transformer() {
        let transformer = CustomTransformer::regex("test", "foo", "bar")
            .with_enabled(false);
        let result = transformer.transform("const x = foo;", None).unwrap();
        assert_eq!(result, "const x = foo;"); // Unchanged
    }

    #[test]
    fn test_transformer_chain() {
        let mut chain = TransformerChain::new();
        chain.add(CustomTransformer::regex("first", "foo", "bar"));
        chain.add(CustomTransformer::regex("second", "bar", "baz"));

        let result = chain.transform("foo".to_string(), None).unwrap();
        assert_eq!(result, "baz");
    }

    #[test]
    fn test_transformer_builder() {
        let chain = TransformerBuilder::new()
            .add_regex("first", "a", "b")
            .add_regex("second", "b", "c")
            .build();

        let result = chain.transform("a".to_string(), None).unwrap();
        assert_eq!(result, "c");
    }

    #[test]
    fn test_remove_console_logs() {
        let transformer = BuiltInTransformers::remove_console_logs();
        let code = "const x = 1;\nconsole.log('test');\nconst y = 2;";
        let result = transformer.transform(code, None).unwrap();
        assert!(!result.contains("console.log"));
        assert!(result.contains("const x = 1"));
        assert!(result.contains("const y = 2"));
    }

    #[test]
    fn test_remove_debugger() {
        let transformer = BuiltInTransformers::remove_debugger();
        let code = "const x = 1;\ndebugger;\nconst y = 2;";
        let result = transformer.transform(code, None).unwrap();
        assert!(!result.contains("debugger"));
    }

    #[test]
    fn test_replace_string() {
        let transformer = BuiltInTransformers::replace_string("hello", "world");
        let result = transformer.transform("hello there", None).unwrap();
        assert_eq!(result, "world there");
    }

    #[test]
    fn test_add_use_strict() {
        let transformer = BuiltInTransformers::add_use_strict();

        // Should add if missing
        let result = transformer.transform("const x = 1;", None).unwrap();
        assert!(result.starts_with("'use strict';"));

        // Should not duplicate
        let result = transformer.transform("'use strict';\nconst x = 1;", None).unwrap();
        assert_eq!(result.matches("'use strict'").count(), 1);
    }

    #[test]
    fn test_arrow_to_function() {
        let transformer = BuiltInTransformers::arrow_to_function();
        let code = "const add = (a, b) => {";
        let result = transformer.transform(code, None).unwrap();
        assert!(result.contains("function add(a, b) {"));
    }

    #[test]
    fn test_test_only_transformer() {
        let inner = TransformerType::Regex {
            pattern: "production".to_string(),
            replacement: "test".to_string(),
        };
        let transformer = BuiltInTransformers::test_only(inner);

        // Should transform test files
        let result = transformer.transform("production", Some("app.test.js")).unwrap();
        assert_eq!(result, "test");

        // Should not transform regular files
        let result = transformer.transform("production", Some("app.js")).unwrap();
        assert_eq!(result, "production");
    }

    #[test]
    fn test_chain_length() {
        let chain = TransformerBuilder::new()
            .add_regex("t1", "a", "b")
            .add_regex("t2", "b", "c")
            .build();

        assert_eq!(chain.len(), 2);
        assert!(!chain.is_empty());
    }

    #[test]
    fn test_empty_chain() {
        let chain = TransformerChain::new();
        assert_eq!(chain.len(), 0);
        assert!(chain.is_empty());

        let result = chain.transform("test".to_string(), None).unwrap();
        assert_eq!(result, "test");
    }

    #[tokio::test]
    async fn test_transformer_plugin() {
        let chain = TransformerBuilder::new()
            .add_regex("test", "old", "new")
            .build();

        let plugin = TransformerPlugin::new("test-plugin", chain);

        let module = ModuleInfo {
            path: std::path::PathBuf::from("test.js"),
            content: "old code".to_string(),
            module_type: crate::core::models::ModuleType::JavaScript,
            dependencies: Vec::new(),
            exports: Vec::new(),
        };

        let result = plugin.transform_code(&module, "old code".to_string()).await.unwrap();
        assert_eq!(result, "new code");
    }
}
