use crate::utils::{ErrorContext, Result, SokuError};
use flate2::{write::GzEncoder, Compression};
use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_diagnostics::OxcDiagnostic;
use oxc_minifier::{CompressOptions, MangleOptions, Minifier, MinifierOptions};
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

/// Lightning-fast JavaScript minification using oxc
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
        let source_type = SourceType::from_path(filename).unwrap_or_else(|_| SourceType::default());

        // Parse the source code
        let parser = Parser::new(&allocator, source_code, source_type);
        let parse_result = parser.parse();

        if !parse_result.errors.is_empty() {
            // Create detailed error context with location information
            let error_context = Self::create_parse_error_context(
                &parse_result.errors,
                source_code,
                Path::new(filename),
            );
            let first_error = &parse_result.errors[0];

            return Err(SokuError::parse_with_context(
                format!("Parse error: {}", first_error),
                error_context,
            ));
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
    #[allow(dead_code)]
    pub fn minify_with_options(
        &self,
        source_code: &str,
        filename: &str,
        mangle: bool,
        compress: bool,
    ) -> Result<String> {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(filename).unwrap_or_else(|_| SourceType::default());

        // Parse the source code
        let parser = Parser::new(&allocator, source_code, source_type);
        let parse_result = parser.parse();

        if !parse_result.errors.is_empty() {
            // Create detailed error context with location information
            let error_context = Self::create_parse_error_context(
                &parse_result.errors,
                source_code,
                Path::new(filename),
            );
            let first_error = &parse_result.errors[0];

            return Err(SokuError::parse_with_context(
                format!("Parse error: {}", first_error),
                error_context,
            ));
        }

        // Create custom minifier options
        let minifier_options = MinifierOptions {
            mangle: if mangle {
                Some(MangleOptions::default())
            } else {
                None
            },
            compress: if compress {
                Some(CompressOptions::default())
            } else {
                None
            },
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

impl OxcMinifier {
    /// Extract detailed error information from oxc parse errors
    fn create_parse_error_context(
        errors: &[OxcDiagnostic],
        content: &str,
        file_path: &Path,
    ) -> ErrorContext {
        // Try to extract span information from the first error
        let mut line_num = None;
        let mut col_num = None;

        if let Some(first_error) = errors.first() {
            let error_msg = format!("{:?}", first_error);
            // Try to parse line/column from debug output
            if let Some(start_pos) = Self::extract_span_from_debug(&error_msg) {
                // Convert byte offset to line/column
                let (line, col) = Self::byte_offset_to_line_col(content, start_pos);
                line_num = Some(line);
                col_num = Some(col);
            }
        }

        // Extract contextual code snippet around the error
        let code_snippet = if let (Some(line), Some(_)) = (line_num, col_num) {
            Self::extract_code_snippet(content, line, 2)
        } else {
            // Fallback: show first 5 lines
            content.lines().take(5).collect::<Vec<_>>().join("\n")
        };

        let mut context = ErrorContext::new()
            .with_file(file_path.to_path_buf())
            .with_snippet(code_snippet);

        // Add line/column if available
        if let (Some(line), Some(col)) = (line_num, col_num) {
            context = context.with_location(line, col);
        }

        context
    }

    /// Extract span start position from debug output
    fn extract_span_from_debug(debug_str: &str) -> Option<usize> {
        // Look for pattern: offset: SourceOffset(313)
        if let Some(offset_idx) = debug_str.find("offset: SourceOffset(") {
            let after_offset = &debug_str[offset_idx + 21..]; // Skip "offset: SourceOffset("
            let num_str: String = after_offset
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            num_str.parse::<usize>().ok()
        } else {
            None
        }
    }

    /// Convert byte offset to 1-based line and 0-based column numbers
    fn byte_offset_to_line_col(content: &str, byte_offset: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 0;
        let mut current_offset = 0;

        for ch in content.chars() {
            if current_offset >= byte_offset {
                break;
            }

            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }

            current_offset += ch.len_utf8();
        }

        (line, col)
    }

    /// Extract code snippet with context lines around the error line
    fn extract_code_snippet(content: &str, error_line: usize, context_lines: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Calculate range with context
        let start_line = error_line.saturating_sub(context_lines + 1);
        let end_line = (error_line + context_lines).min(total_lines);

        lines[start_line..end_line].join("\n")
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
        tokio::task::spawn_blocking(move || minifier.minify(&bundle, &filename))
            .await
            .map_err(|e| SokuError::build(format!("Minification task failed: {}", e)))?
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
            gzip_reduction_percentage: self
                .calculate_gzip_reduction(&gzip_original, &gzip_minified),
        }
    }

    /// Compress content with gzip for analysis
    pub fn gzip_compress(&self, content: &[u8]) -> Result<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(content)
            .map_err(|e| SokuError::build(format!("Gzip compression failed: {}", e)))?;

        encoder
            .finish()
            .map_err(|e| SokuError::build(format!("Gzip finish failed: {}", e)))
    }

    /// Advanced minification with optimal settings for production
    #[allow(dead_code)]
    pub async fn minify_with_advanced_optimization(
        &self,
        bundle: String,
        filename: &str,
    ) -> Result<AdvancedMinificationResult> {
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
                compression_ratio: (bundle.len() as f64 - final_minified.len() as f64)
                    / bundle.len() as f64
                    * 100.0,
            })
        })
        .await
        .map_err(|e| SokuError::build(format!("Advanced minification task failed: {}", e)))?
    }

    /// Calculate gzip compression reduction
    fn calculate_gzip_reduction(&self, original_gzip: &[u8], minified_gzip: &[u8]) -> f64 {
        if original_gzip.is_empty() {
            return 0.0;
        }

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
#[allow(dead_code)]
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
