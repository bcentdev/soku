use crate::core::interfaces::CssProcessor;
use crate::utils::{CssModulesProcessor, Logger, Result, SokuCache, SokuError};
use lightningcss::{
    printer::PrinterOptions,
    stylesheet::{ParserOptions as CssParserOptions, StyleSheet},
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub struct LightningCssProcessor {
    minify: bool,
    cache: Arc<SokuCache>,
}

impl LightningCssProcessor {
    pub fn new(minify: bool) -> Self {
        Self {
            minify,
            cache: Arc::new(SokuCache::new()),
        }
    }
}

#[async_trait::async_trait]
impl CssProcessor for LightningCssProcessor {
    async fn process_css(&self, content: &str, path: &Path) -> Result<String> {
        let _timer = crate::utils::Timer::start(&format!(
            "Processing CSS {}",
            path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
        ));

        // Check cache first
        let path_str = path.to_string_lossy();
        if let Some(cached) = self.cache.get_css(&path_str, content) {
            return Ok(cached);
        }

        Logger::processing_css(
            path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown"),
        );

        // Check if this is a CSS Module
        let css_content = if CssModulesProcessor::is_css_module(path) {
            Logger::info(&format!(
                "📦 CSS Module detected: {}",
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
            ));

            let css_modules = CssModulesProcessor::new();
            let module_result = css_modules.process(content, path)?;

            // TODO: Save exports for JS import resolution
            Logger::debug(&format!(
                "CSS Module exports: {:?}",
                module_result.exports.keys()
            ));

            module_result.css
        } else {
            content.to_string()
        };

        // Process CSS with lightningcss
        let result = match StyleSheet::parse(&css_content, CssParserOptions::default()) {
            Ok(stylesheet) => {
                match stylesheet.to_css(PrinterOptions {
                    minify: self.minify,
                    ..Default::default()
                }) {
                    Ok(result) => Ok(result.code),
                    Err(_) => {
                        Logger::warn(&format!(
                            "CSS processing failed for {}, using fallback minification",
                            path.display()
                        ));
                        Ok(self.fallback_minify(content))
                    }
                }
            }
            Err(_) => {
                Logger::warn(&format!(
                    "CSS parse error for {}, using fallback minification",
                    path.display()
                ));
                Ok(self.fallback_minify(content))
            }
        };

        // Cache the result
        if let Ok(ref processed) = result {
            self.cache.cache_css(&path_str, content, processed.clone());
        }

        result
    }

    async fn bundle_css(&self, files: &[PathBuf]) -> Result<String> {
        let _timer = crate::utils::Timer::start("Bundling CSS files");

        let mut bundle = String::new();
        bundle.push_str("/* Soku Bundler - CSS Bundle */\n");

        for css_file in files {
            let content = tokio::fs::read_to_string(css_file)
                .await
                .map_err(SokuError::Io)?;

            let processed = self.process_css(&content, css_file).await?;

            bundle.push_str(&format!(
                "/* From: {} */\n",
                css_file
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
            ));
            bundle.push_str(&processed);
            bundle.push('\n');
        }

        Ok(bundle)
    }
}

impl LightningCssProcessor {
    fn fallback_minify(&self, content: &str) -> String {
        if self.minify {
            content
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join("")
        } else {
            content.to_string()
        }
    }
}

impl Default for LightningCssProcessor {
    fn default() -> Self {
        Self::new(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_css_processing() {
        let processor = LightningCssProcessor::new(true);
        let path = PathBuf::from("test.css");

        let css = r#"
        body {
            color: red;
            background: blue;
        }

        .container {
            margin: 0 auto;
        }
        "#;

        let result = processor.process_css(css, &path).await.unwrap();

        // Should be processed (exact format depends on Lightning CSS)
        assert!(!result.is_empty());
        assert!(result.contains("body") || result.contains("red") || result.contains("blue"));
    }

    #[tokio::test]
    async fn test_css_bundling() {
        let processor = LightningCssProcessor::new(false);

        // Create temporary files for testing
        let temp_dir = tempfile::tempdir().unwrap();
        let css1_path = temp_dir.path().join("style1.css");
        let css2_path = temp_dir.path().join("style2.css");

        tokio::fs::write(&css1_path, "body { color: red; }")
            .await
            .unwrap();
        tokio::fs::write(&css2_path, ".container { margin: 0; }")
            .await
            .unwrap();

        let files = vec![css1_path, css2_path];
        let result = processor.bundle_css(&files).await.unwrap();

        assert!(result.contains("Soku Bundler"));
        assert!(result.contains("From: style1.css"));
        assert!(result.contains("From: style2.css"));
        assert!(result.contains("color: red") || result.contains("red"));
        assert!(result.contains("margin") || result.contains("container"));
    }
}
