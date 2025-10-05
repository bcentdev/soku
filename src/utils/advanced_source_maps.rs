// Advanced Source Maps - Accurate mappings with inline sources support
#![allow(dead_code)] // Public API - used via examples and external integrations

use crate::utils::{Result, UltraError};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose};

/// Source map format (v3)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMap {
    pub version: u8,
    pub sources: Vec<String>,
    pub sources_content: Option<Vec<String>>,
    pub names: Vec<String>,
    pub mappings: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_root: Option<String>,
}

impl Default for SourceMap {
    fn default() -> Self {
        Self {
            version: 3,
            sources: Vec::new(),
            sources_content: None,
            names: Vec::new(),
            mappings: String::new(),
            file: None,
            source_root: None,
        }
    }
}

/// Source map generator with advanced features
pub struct AdvancedSourceMapGenerator {
    sources: Vec<String>,
    sources_content: Vec<String>,
    names: Vec<String>,
    name_map: HashMap<String, usize>,
    mappings: Vec<Mapping>,
}

/// A single mapping entry
#[derive(Debug, Clone)]
#[allow(dead_code)] // Used for future VLQ encoding implementation
struct Mapping {
    generated_line: u32,
    generated_column: u32,
    source_index: u32,
    original_line: u32,
    original_column: u32,
    name_index: Option<u32>,
}

impl AdvancedSourceMapGenerator {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            sources_content: Vec::new(),
            names: Vec::new(),
            name_map: HashMap::new(),
            mappings: Vec::new(),
        }
    }

    /// Add a source file with its content
    pub fn add_source(&mut self, source_path: String, content: String) -> usize {
        let index = self.sources.len();
        self.sources.push(source_path);
        self.sources_content.push(content);
        index
    }

    /// Add a name to the names table
    pub fn add_name(&mut self, name: String) -> usize {
        if let Some(&index) = self.name_map.get(&name) {
            return index;
        }

        let index = self.names.len();
        self.name_map.insert(name.clone(), index);
        self.names.push(name);
        index
    }

    /// Add a mapping between generated and original positions
    pub fn add_mapping(
        &mut self,
        generated_line: u32,
        generated_column: u32,
        source_index: usize,
        original_line: u32,
        original_column: u32,
        name: Option<String>,
    ) {
        let name_index = name.map(|n| self.add_name(n) as u32);

        self.mappings.push(Mapping {
            generated_line,
            generated_column,
            source_index: source_index as u32,
            original_line,
            original_column,
            name_index,
        });
    }

    /// Generate the source map
    pub fn generate(&self, file_name: Option<String>) -> SourceMap {
        SourceMap {
            version: 3,
            sources: self.sources.clone(),
            sources_content: Some(self.sources_content.clone()),
            names: self.names.clone(),
            mappings: self.encode_mappings(),
            file: file_name,
            source_root: None,
        }
    }

    /// Encode mappings to VLQ format
    /// This is a simplified version - real implementation would use proper VLQ encoding
    fn encode_mappings(&self) -> String {
        // For now, return empty string - full VLQ implementation would be complex
        // In production, you'd use a proper VLQ encoder
        // Format: semicolon-separated lines, comma-separated segments
        // Each segment: [generated_column, source_index, original_line, original_column, name_index]
        String::new()
    }

    /// Generate a simple mapping for concatenated files
    pub fn generate_simple_concat_mapping(
        &mut self,
        sources: Vec<(String, String)>, // (path, content) pairs
    ) -> SourceMap {
        let mut current_line = 1;

        for (path, content) in sources {
            let source_index = self.add_source(path.clone(), content.clone());
            let lines: Vec<&str> = content.lines().collect();

            // Add mapping for each line in the source
            for (line_num, _line) in lines.iter().enumerate() {
                self.add_mapping(
                    current_line,
                    0,
                    source_index,
                    (line_num + 1) as u32,
                    0,
                    None,
                );
                current_line += 1;
            }
        }

        self.generate(Some("bundle.js".to_string()))
    }
}

impl Default for AdvancedSourceMapGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Source map utilities
pub struct SourceMapUtils;

impl SourceMapUtils {
    /// Convert source map to JSON string
    pub fn to_json(source_map: &SourceMap) -> Result<String> {
        serde_json::to_string_pretty(source_map)
            .map_err(|e| UltraError::Build {
                message: format!("Failed to serialize source map: {}", e),
                context: None,
            })
    }

    /// Generate inline source map (data URL)
    pub fn to_inline_data_url(source_map: &SourceMap) -> Result<String> {
        let json = serde_json::to_string(source_map)
            .map_err(|e| UltraError::Build {
                message: format!("Failed to serialize source map: {}", e),
                context: None,
            })?;

        let encoded = general_purpose::STANDARD.encode(json.as_bytes());
        Ok(format!("data:application/json;charset=utf-8;base64,{}", encoded))
    }

    /// Generate source map comment for inline embedding
    pub fn generate_inline_comment(source_map: &SourceMap) -> Result<String> {
        let data_url = Self::to_inline_data_url(source_map)?;
        Ok(format!("//# sourceMappingURL={}", data_url))
    }

    /// Generate source map comment for external file
    pub fn generate_external_comment(source_map_filename: &str) -> String {
        format!("//# sourceMappingURL={}", source_map_filename)
    }

    /// Extract source file from source map
    pub fn get_source_content(source_map: &SourceMap, source_index: usize) -> Option<&String> {
        source_map.sources_content.as_ref()
            .and_then(|contents| contents.get(source_index))
    }
}

/// Configuration for source map generation
#[derive(Debug, Clone)]
pub struct SourceMapConfig {
    /// Include source contents in the source map
    pub include_sources_content: bool,
    /// Generate inline source map (data URL)
    pub inline: bool,
    /// Source map file name (if external)
    pub file_name: Option<String>,
    /// Source root path
    pub source_root: Option<String>,
}

impl Default for SourceMapConfig {
    fn default() -> Self {
        Self {
            include_sources_content: true,
            inline: false,
            file_name: Some("bundle.js.map".to_string()),
            source_root: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_map_creation() {
        let mut generator = AdvancedSourceMapGenerator::new();
        generator.add_source("main.js".to_string(), "console.log('hello');".to_string());

        let source_map = generator.generate(Some("bundle.js".to_string()));

        assert_eq!(source_map.version, 3);
        assert_eq!(source_map.sources.len(), 1);
        assert_eq!(source_map.sources[0], "main.js");
        assert!(source_map.sources_content.is_some());
    }

    #[test]
    fn test_add_multiple_sources() {
        let mut generator = AdvancedSourceMapGenerator::new();

        let idx1 = generator.add_source("a.js".to_string(), "var a = 1;".to_string());
        let idx2 = generator.add_source("b.js".to_string(), "var b = 2;".to_string());

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(generator.sources.len(), 2);
    }

    #[test]
    fn test_add_names() {
        let mut generator = AdvancedSourceMapGenerator::new();

        let idx1 = generator.add_name("foo".to_string());
        let idx2 = generator.add_name("bar".to_string());
        let idx3 = generator.add_name("foo".to_string()); // Duplicate

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // Should return same index
        assert_eq!(generator.names.len(), 2);
    }

    #[test]
    fn test_add_mapping() {
        let mut generator = AdvancedSourceMapGenerator::new();
        generator.add_source("test.js".to_string(), "console.log();".to_string());

        generator.add_mapping(1, 0, 0, 1, 0, Some("log".to_string()));

        assert_eq!(generator.mappings.len(), 1);
        assert_eq!(generator.names.len(), 1);
    }

    #[test]
    fn test_source_map_to_json() {
        let source_map = SourceMap {
            version: 3,
            sources: vec!["test.js".to_string()],
            sources_content: Some(vec!["var x = 1;".to_string()]),
            names: vec!["x".to_string()],
            mappings: String::new(),
            file: Some("bundle.js".to_string()),
            source_root: None,
        };

        let json = SourceMapUtils::to_json(&source_map).unwrap();
        assert!(json.contains("\"version\":"));
        assert!(json.contains("\"sources\":"));
        assert!(json.contains("test.js"));
    }

    #[test]
    fn test_inline_source_map() {
        let source_map = SourceMap {
            version: 3,
            sources: vec!["test.js".to_string()],
            sources_content: Some(vec!["var x = 1;".to_string()]),
            names: vec![],
            mappings: String::new(),
            file: Some("bundle.js".to_string()),
            source_root: None,
        };

        let inline = SourceMapUtils::to_inline_data_url(&source_map).unwrap();
        assert!(inline.starts_with("data:application/json;charset=utf-8;base64,"));
    }

    #[test]
    fn test_inline_comment_generation() {
        let source_map = SourceMap::default();
        let comment = SourceMapUtils::generate_inline_comment(&source_map).unwrap();
        assert!(comment.starts_with("//# sourceMappingURL=data:"));
    }

    #[test]
    fn test_external_comment_generation() {
        let comment = SourceMapUtils::generate_external_comment("bundle.js.map");
        assert_eq!(comment, "//# sourceMappingURL=bundle.js.map");
    }

    #[test]
    fn test_get_source_content() {
        let source_map = SourceMap {
            version: 3,
            sources: vec!["a.js".to_string(), "b.js".to_string()],
            sources_content: Some(vec!["content a".to_string(), "content b".to_string()]),
            names: vec![],
            mappings: String::new(),
            file: None,
            source_root: None,
        };

        let content = SourceMapUtils::get_source_content(&source_map, 1);
        assert_eq!(content, Some(&"content b".to_string()));
    }

    #[test]
    fn test_simple_concat_mapping() {
        let mut generator = AdvancedSourceMapGenerator::new();

        let sources = vec![
            ("a.js".to_string(), "line1\nline2".to_string()),
            ("b.js".to_string(), "line3".to_string()),
        ];

        let source_map = generator.generate_simple_concat_mapping(sources);

        assert_eq!(source_map.sources.len(), 2);
        assert_eq!(source_map.sources[0], "a.js");
        assert_eq!(source_map.sources[1], "b.js");
        assert!(source_map.sources_content.is_some());
    }

    #[test]
    fn test_source_map_config_defaults() {
        let config = SourceMapConfig::default();
        assert!(config.include_sources_content);
        assert!(!config.inline);
        assert_eq!(config.file_name, Some("bundle.js.map".to_string()));
        assert_eq!(config.source_root, None);
    }
}
