use crate::utils::{Result, UltraError};
use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_minifier::{Minifier, MinifierOptions, CompressOptions, MangleOptions};
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::sync::Arc;
use std::io::Write;
use flate2::{Compression, write::GzEncoder};

/// Ultra-fast JavaScript minification using oxc
pub struct OxcMinifier {
    options: MinifierOptions,
}

impl OxcMinifier {
    pub fn new() -> Self {
        Self {
            options: MinifierOptions {
                mangle: Some(MangleOptions::default()),
                compress: Some(CompressOptions::default()),
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

        // Minify the AST with oxc 0.90 API
        let mut program = parse_result.program;
        let minifier = Minifier::new(self.options.clone());
        let _minifier_result = minifier.minify(&allocator, &mut program);

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
        let minifier_options = MinifierOptions {
            mangle: if mangle { Some(MangleOptions::default()) } else { None },
            compress: if compress { Some(CompressOptions::default()) } else { None },
        };

        // Minify the AST with oxc 0.90 API
        let mut program = parse_result.program;
        let minifier = Minifier::new(minifier_options);
        let _minifier_result = minifier.minify(&allocator, &mut program);

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

    /// Get minification statistics with compression analysis
    pub fn get_stats(&self, original: &str, minified: &str) -> MinificationStats {
        let gzip_original = self.gzip_compress(original.as_bytes()).unwrap_or_default();
        let gzip_minified = self.gzip_compress(minified.as_bytes()).unwrap_or_default();

        MinificationStats {
            original_size: original.len(),
            minified_size: minified.len(),
            reduction_percentage: self.minifier.calculate_reduction(original, minified),
            saved_bytes: original.len().saturating_sub(minified.len()),
            gzip_original_size: gzip_original.len(),
            gzip_minified_size: gzip_minified.len(),
            gzip_reduction_percentage: self.calculate_gzip_reduction(&gzip_original, &gzip_minified),
        }
    }

    /// Compress content with gzip for analysis
    pub fn gzip_compress(&self, content: &[u8]) -> Result<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(content)
            .map_err(|e| UltraError::Build(format!("Gzip compression failed: {}", e)))?;

        encoder.finish()
            .map_err(|e| UltraError::Build(format!("Gzip finish failed: {}", e)))
    }

    /// Advanced minification with optimal settings for production
    pub async fn minify_with_advanced_optimization(&self, bundle: String, filename: &str) -> Result<AdvancedMinificationResult> {
        let minifier = self.minifier.clone();
        let filename = filename.to_string();

        // Run advanced minification in a blocking task
        tokio::task::spawn_blocking(move || {
            // First pass: standard minification
            let standard_minified = minifier.minify(&bundle, &filename)?;

            // For now, use the standard minified result
            // TODO: Implement advanced aggressive compression when oxc API is stable
            let final_minified = standard_minified;

            Ok(AdvancedMinificationResult {
                code: final_minified.clone(),
                original_size: bundle.len(),
                minified_size: final_minified.len(),
                compression_ratio: (bundle.len() as f64 - final_minified.len() as f64) / bundle.len() as f64 * 100.0,
            })
        })
        .await
        .map_err(|e| UltraError::Build(format!("Advanced minification task failed: {}", e)))?
    }

    /// Calculate gzip compression reduction
    fn calculate_gzip_reduction(&self, original_gzip: &[u8], minified_gzip: &[u8]) -> f64 {
        if original_gzip.is_empty() { return 0.0; }

        let original_size = original_gzip.len() as f64;
        let minified_size = minified_gzip.len() as f64;

        ((original_size - minified_size) / original_size) * 100.0
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
    pub gzip_original_size: usize,
    pub gzip_minified_size: usize,
    pub gzip_reduction_percentage: f64,
}

#[derive(Debug, Clone)]
pub struct AdvancedMinificationResult {
    pub code: String,
    pub original_size: usize,
    pub minified_size: usize,
    pub compression_ratio: f64,
}

impl std::fmt::Display for MinificationStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Minification: {:.1}% reduction ({} → {} bytes, saved {}), Gzip: {:.1}% ({} → {} bytes)",
            self.reduction_percentage,
            self.original_size,
            self.minified_size,
            self.saved_bytes,
            self.gzip_reduction_percentage,
            self.gzip_original_size,
            self.gzip_minified_size
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