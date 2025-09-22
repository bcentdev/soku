use crate::utils::{Result, UltraError};
use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_minifier::{Minifier, MinifierOptions, CompressOptions};
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::sync::Arc;

/// Ultra-fast JavaScript minification using oxc
pub struct OxcMinifier {
    options: MinifierOptions,
}

impl OxcMinifier {
    pub fn new() -> Self {
        Self {
            options: MinifierOptions {
                mangle: true,
                compress: CompressOptions::default(),
            },
        }
    }

    /// Minify JavaScript code
    pub fn minify(&self, source_code: &str, filename: &str) -> Result<String> {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(filename)
            .unwrap_or_else(|_| SourceType::default());

        // Parse the source code
        let parser = Parser::new(&allocator, source_code, source_type);
        let parse_result = parser.parse();

        if !parse_result.errors.is_empty() {
            let errors: Vec<String> = parse_result.errors
                .iter()
                .map(|e| format!("Parse error: {}", e))
                .collect();
            return Err(UltraError::Build(errors.join("\n")));
        }

        // Minify the AST
        let mut program = parse_result.program;
        let minifier = Minifier::new(self.options.clone());
        minifier.build(&allocator, &mut program);

        // Generate minified code
        let codegen_options = CodegenOptions {
            minify: true,
            ..Default::default()
        };

        let mut codegen = Codegen::new();
        codegen = codegen.with_options(codegen_options);
        let minified_code = codegen.build(&program).code;

        Ok(minified_code)
    }

    /// Minify with custom options
    pub fn minify_with_options(
        &self,
        source_code: &str,
        filename: &str,
        mangle: bool,
        compress: bool,
    ) -> Result<String> {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(filename)
            .unwrap_or_else(|_| SourceType::default());

        // Parse the source code
        let parser = Parser::new(&allocator, source_code, source_type);
        let parse_result = parser.parse();

        if !parse_result.errors.is_empty() {
            let errors: Vec<String> = parse_result.errors
                .iter()
                .map(|e| format!("Parse error: {}", e))
                .collect();
            return Err(UltraError::Build(errors.join("\n")));
        }

        // Create custom minifier options
        let options = MinifierOptions {
            mangle,
            compress: if compress { CompressOptions::default() } else { CompressOptions::all_false() },
        };

        // Minify the AST
        let mut program = parse_result.program;
        let minifier = Minifier::new(options);
        minifier.build(&allocator, &mut program);

        // Generate minified code
        let codegen_options = CodegenOptions {
            minify: true,
            ..Default::default()
        };

        let mut codegen = Codegen::new();
        codegen = codegen.with_options(codegen_options);
        let minified_code = codegen.build(&program).code;

        Ok(minified_code)
    }

    /// Estimate size reduction percentage
    pub fn calculate_reduction(&self, original: &str, minified: &str) -> f64 {
        let original_size = original.len() as f64;
        let minified_size = minified.len() as f64;

        if original_size == 0.0 {
            return 0.0;
        }

        ((original_size - minified_size) / original_size) * 100.0
    }
}

impl Default for OxcMinifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Async wrapper for minification in the build pipeline
pub struct MinificationService {
    minifier: Arc<OxcMinifier>,
}

impl MinificationService {
    pub fn new() -> Self {
        Self {
            minifier: Arc::new(OxcMinifier::new()),
        }
    }

    /// Minify JavaScript bundle asynchronously
    pub async fn minify_bundle(&self, bundle: String, filename: &str) -> Result<String> {
        let minifier = self.minifier.clone();
        let filename = filename.to_string();

        // Run minification in a blocking task since oxc is CPU-intensive
        tokio::task::spawn_blocking(move || {
            minifier.minify(&bundle, &filename)
        })
        .await
        .map_err(|e| UltraError::Build(format!("Minification task failed: {}", e)))?
    }

    /// Get minification statistics
    pub fn get_stats(&self, original: &str, minified: &str) -> MinificationStats {
        MinificationStats {
            original_size: original.len(),
            minified_size: minified.len(),
            reduction_percentage: self.minifier.calculate_reduction(original, minified),
            saved_bytes: original.len().saturating_sub(minified.len()),
        }
    }
}

impl Default for MinificationService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct MinificationStats {
    pub original_size: usize,
    pub minified_size: usize,
    pub reduction_percentage: f64,
    pub saved_bytes: usize,
}

impl std::fmt::Display for MinificationStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Minification: {:.1}% reduction ({} → {} bytes, saved {})",
            self.reduction_percentage,
            self.original_size,
            self.minified_size,
            self.saved_bytes
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_minification() {
        let minifier = OxcMinifier::new();
        let source = r#"
            function hello(name) {
                const message = "Hello, " + name;
                console.log(message);
                return message;
            }
        "#;

        let result = minifier.minify(source, "test.js");
        assert!(result.is_ok());

        let minified = result.unwrap();
        assert!(minified.len() < source.len());
    }

    #[test]
    fn test_reduction_calculation() {
        let minifier = OxcMinifier::new();
        let original = "function hello() { return 'world'; }";
        let minified = "function hello(){return'world'}";

        let reduction = minifier.calculate_reduction(original, minified);
        assert!(reduction > 0.0);
    }
}