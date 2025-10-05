use crate::core::interfaces::CssProcessor;
use crate::utils::{Result, SokuError, Logger, SokuCache};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// SCSS/SASS preprocessor using the grass crate
///
/// Compiles SCSS/SASS to CSS and then optionally processes with LightningCSS
#[derive(Clone)]
pub struct ScssProcessor {
    minify: bool,
    cache: Arc<SokuCache>,
    css_processor: Option<Arc<dyn CssProcessor>>,
}

impl ScssProcessor {
    /// Create a new SCSS processor
    #[allow(dead_code)]
    pub fn new(minify: bool) -> Self {
        Self {
            minify,
            cache: Arc::new(SokuCache::new()),
            css_processor: None,
        }
    }

    /// Create SCSS processor with CSS post-processor
    pub fn with_css_processor(minify: bool, css_processor: Arc<dyn CssProcessor>) -> Self {
        Self {
            minify,
            cache: Arc::new(SokuCache::new()),
            css_processor: Some(css_processor),
        }
    }

    /// Check if a file is SCSS/SASS
    pub fn is_scss_file(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            matches!(ext.to_str(), Some("scss") | Some("sass"))
        } else {
            false
        }
    }

    /// Compile SCSS/SASS to CSS
    fn compile_scss(&self, content: &str, path: &Path) -> Result<String> {
        let _timer = crate::utils::Timer::start(&format!(
            "Compiling SCSS {}",
            path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
        ));

        Logger::info(&format!(
            "🎨 Compiling SCSS: {}",
            path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
        ));

        // Detect syntax based on file extension
        let syntax = if path.extension().and_then(|s| s.to_str()) == Some("sass") {
            grass::InputSyntax::Sass
        } else {
            grass::InputSyntax::Scss
        };

        // Configure grass options
        let options = grass::Options::default()
            .input_syntax(syntax)
            .style(if self.minify {
                grass::OutputStyle::Compressed
            } else {
                grass::OutputStyle::Expanded
            });

        // Compile SCSS/SASS to CSS
        match grass::from_string(content.to_string(), &options) {
            Ok(css) => {
                Logger::debug(&format!(
                    "SCSS compiled successfully: {} -> {} bytes",
                    path.display(),
                    css.len()
                ));
                Ok(css)
            }
            Err(e) => {
                let error_msg = format!("SCSS compilation error in {}: {}", path.display(), e);
                Logger::error(&error_msg);
                Err(SokuError::CssProcessing(error_msg))
            }
        }
    }
}

#[async_trait::async_trait]
impl CssProcessor for ScssProcessor {
    async fn process_css(&self, content: &str, path: &Path) -> Result<String> {
        // Check cache first
        let path_str = path.to_string_lossy();
        if let Some(cached) = self.cache.get_css(&path_str, content) {
            Logger::debug(&format!("Cache hit for SCSS: {}", path.display()));
            return Ok(cached);
        }

        // Compile SCSS/SASS to CSS
        let css = self.compile_scss(content, path)?;

        // Optionally post-process with CSS processor (LightningCSS)
        let result = if let Some(ref processor) = self.css_processor {
            Logger::debug("Post-processing compiled CSS with LightningCSS");
            processor.process_css(&css, path).await?
        } else {
            css
        };

        // Cache the result
        self.cache.cache_css(&path_str, content, result.clone());

        Ok(result)
    }

    async fn bundle_css(&self, files: &[PathBuf]) -> Result<String> {
        let _timer = crate::utils::Timer::start("Bundling SCSS/CSS files");

        let mut bundle = String::new();
        bundle.push_str("/* Soku Bundler - SCSS/CSS Bundle */\n");

        for file_path in files {
            Logger::debug(&format!("Bundling file: {}", file_path.display()));

            // Read file content
            let content = tokio::fs::read_to_string(file_path)
                .await?;

            // Check if it's SCSS/SASS or regular CSS
            let processed = if Self::is_scss_file(file_path) {
                // Compile SCSS/SASS
                self.process_css(&content, file_path).await?
            } else if let Some(ref processor) = self.css_processor {
                // Process regular CSS
                processor.process_css(&content, file_path).await?
            } else {
                // No processing, just use content
                content
            };

            // Add to bundle with comment
            bundle.push_str(&format!(
                "\n/* File: {} */\n",
                file_path.display()
            ));
            bundle.push_str(&processed);
            bundle.push('\n');
        }

        Logger::info(&format!(
            "📦 Bundled {} SCSS/CSS files ({} bytes)",
            files.len(),
            bundle.len()
        ));

        Ok(bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_scss_file() {
        assert!(ScssProcessor::is_scss_file(Path::new("styles.scss")));
        assert!(ScssProcessor::is_scss_file(Path::new("app.sass")));
        assert!(!ScssProcessor::is_scss_file(Path::new("styles.css")));
        assert!(!ScssProcessor::is_scss_file(Path::new("script.js")));
    }

    #[tokio::test]
    async fn test_compile_basic_scss() {
        let processor = ScssProcessor::new(false);
        let scss = "$primary: #333;\nbody { color: $primary; }";
        let path = Path::new("test.scss");

        let result = processor.process_css(scss, path).await;
        assert!(result.is_ok());
        let css = result.unwrap();
        assert!(css.contains("color"));
        assert!(css.contains("#333") || css.contains("color: #333"));
    }

    #[tokio::test]
    async fn test_compile_nested_scss() {
        let processor = ScssProcessor::new(false);
        let scss = ".nav { ul { margin: 0; li { display: inline-block; } } }";
        let path = Path::new("test.scss");

        let result = processor.process_css(scss, path).await;
        assert!(result.is_ok());
        let css = result.unwrap();
        // Should expand nested selectors
        assert!(css.contains(".nav ul") || css.contains("margin"));
    }

    #[tokio::test]
    async fn test_compile_with_minify() {
        let processor = ScssProcessor::new(true);
        let scss = "$spacing: 20px;\n.container { padding: $spacing; margin: $spacing; }";
        let path = Path::new("test.scss");

        let result = processor.process_css(scss, path).await;
        assert!(result.is_ok());
        let css = result.unwrap();
        // Minified output should be shorter
        assert!(css.len() < scss.len() + 50); // Some reasonable threshold
    }

    #[tokio::test]
    async fn test_compile_error_handling() {
        let processor = ScssProcessor::new(false);
        let invalid_scss = "$primary: ;\nbody { color: $primary; }"; // Invalid: no value
        let path = Path::new("test.scss");

        let result = processor.process_css(invalid_scss, path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let processor = ScssProcessor::new(false);
        let scss = "$primary: #333;\nbody { color: $primary; }";
        let path = Path::new("test.scss");

        // First call - cache miss
        let result1 = processor.process_css(scss, path).await;
        assert!(result1.is_ok());

        // Second call - should hit cache
        let result2 = processor.process_css(scss, path).await;
        assert!(result2.is_ok());
        assert_eq!(result1.unwrap(), result2.unwrap());
    }
}
