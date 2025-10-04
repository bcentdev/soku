use crate::utils::{Result, UltraError, Logger};
use std::path::Path;

/// Asset processor for handling non-JS assets (JSON, images, etc.)
pub struct AssetProcessor;

impl AssetProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Process JSON file and convert to ES module
    pub fn process_json(&self, content: &str, file_path: &Path) -> Result<String> {
        // Validate JSON
        let _: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| UltraError::build(format!(
                "Invalid JSON in {}: {}",
                file_path.display(),
                e
            )))?;

        Logger::debug(&format!("📦 Processing JSON asset: {}", file_path.display()));

        // Convert JSON to ES module
        // We export the JSON as a default export
        let module = format!(
            "// JSON Module: {}\nconst data = {};\nexport default data;\n",
            file_path.display(),
            content
        );

        Ok(module)
    }

    /// Check if a file should be treated as an asset
    #[allow(dead_code)] // Public API method
    pub fn is_asset_file(path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            matches!(ext.to_lowercase().as_str(), "json")
        } else {
            false
        }
    }

    /// Get asset type from file extension
    #[allow(dead_code)]
    pub fn get_asset_type(path: &Path) -> Option<AssetType> {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            match ext.to_lowercase().as_str() {
                "json" => Some(AssetType::Json),
                "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" => Some(AssetType::Image),
                "woff" | "woff2" | "ttf" | "otf" | "eot" => Some(AssetType::Font),
                _ => None,
            }
        } else {
            None
        }
    }
}

impl Default for AssetProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Types of assets that can be processed
#[derive(Debug, Clone, PartialEq)]
pub enum AssetType {
    Json,
    Image,
    Font,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_process_json_valid() {
        let processor = AssetProcessor::new();
        let json = r#"{"name": "test", "version": "1.0.0"}"#;
        let path = PathBuf::from("config.json");

        let result = processor.process_json(json, &path);
        assert!(result.is_ok());

        let module = result.unwrap();
        assert!(module.contains("const data ="));
        assert!(module.contains("export default data"));
        assert!(module.contains(json));
    }

    #[test]
    fn test_process_json_invalid() {
        let processor = AssetProcessor::new();
        let invalid_json = r#"{"name": "test", invalid}"#;
        let path = PathBuf::from("config.json");

        let result = processor.process_json(invalid_json, &path);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_asset_file() {
        assert!(AssetProcessor::is_asset_file(&PathBuf::from("config.json")));
        assert!(AssetProcessor::is_asset_file(&PathBuf::from("data/config.json")));
        assert!(!AssetProcessor::is_asset_file(&PathBuf::from("main.js")));
        assert!(!AssetProcessor::is_asset_file(&PathBuf::from("style.css")));
    }

    #[test]
    fn test_get_asset_type() {
        assert_eq!(
            AssetProcessor::get_asset_type(&PathBuf::from("config.json")),
            Some(AssetType::Json)
        );
        assert_eq!(
            AssetProcessor::get_asset_type(&PathBuf::from("logo.png")),
            Some(AssetType::Image)
        );
        assert_eq!(
            AssetProcessor::get_asset_type(&PathBuf::from("font.woff2")),
            Some(AssetType::Font)
        );
        assert_eq!(
            AssetProcessor::get_asset_type(&PathBuf::from("main.js")),
            None
        );
    }

    #[test]
    fn test_process_json_array() {
        let processor = AssetProcessor::new();
        let json = r#"[1, 2, 3, 4, 5]"#;
        let path = PathBuf::from("numbers.json");

        let result = processor.process_json(json, &path);
        assert!(result.is_ok());

        let module = result.unwrap();
        assert!(module.contains(json));
    }

    #[test]
    fn test_process_json_nested() {
        let processor = AssetProcessor::new();
        let json = r#"{"user": {"name": "John", "age": 30, "roles": ["admin", "user"]}}"#;
        let path = PathBuf::from("user.json");

        let result = processor.process_json(json, &path);
        assert!(result.is_ok());
    }
}
