use std::path::PathBuf;
use thiserror::Error;

/// Enhanced error with file location context
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub file_path: Option<PathBuf>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub code_snippet: Option<String>,
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorContext {
    pub fn new() -> Self {
        Self {
            file_path: None,
            line: None,
            column: None,
            code_snippet: None,
        }
    }

    pub fn with_file(mut self, path: PathBuf) -> Self {
        self.file_path = Some(path);
        self
    }

    #[allow(dead_code)] // Future use for detailed error reporting
    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    pub fn with_snippet(mut self, snippet: String) -> Self {
        self.code_snippet = Some(snippet);
        self
    }
}

#[derive(Error, Debug)]
pub enum SokuError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {message}")]
    Parse {
        message: String,
        context: Option<ErrorContext>,
    },

    #[error("Build error: {message}")]
    Build {
        message: String,
        context: Option<ErrorContext>,
    },

    #[error("Tree shaking error: {0}")]
    #[allow(dead_code)] // Future error handling
    TreeShaking(String),

    #[error("CSS processing error: {0}")]
    #[allow(dead_code)] // Future error handling
    CssProcessing(String),

    #[error("Configuration error: {0}")]
    #[allow(dead_code)] // Future error handling
    Config(String),

    #[error("File not found: {0}")]
    #[allow(dead_code)] // Future error handling
    FileNotFound(String),

    #[error("Invalid path: {0}")]
    #[allow(dead_code)] // Future error handling
    InvalidPath(String),

    #[error("{0}")]
    #[allow(dead_code)] // Generic error variant for future use
    Other(String),
}

impl SokuError {
    /// Create a simple parse error without context
    pub fn parse(message: String) -> Self {
        Self::Parse {
            message,
            context: None,
        }
    }

    /// Create a parse error with context
    pub fn parse_with_context(message: String, context: ErrorContext) -> Self {
        Self::Parse {
            message,
            context: Some(context),
        }
    }

    /// Create a simple build error without context
    pub fn build(message: String) -> Self {
        Self::Build {
            message,
            context: None,
        }
    }

    /// Create a build error with context
    #[allow(dead_code)] // Future use for detailed build error reporting
    pub fn build_with_context(message: String, context: ErrorContext) -> Self {
        Self::Build {
            message,
            context: Some(context),
        }
    }

    /// Create a configuration error
    pub fn config(message: String) -> Self {
        Self::Config(message)
    }

    /// Format error with enhanced context display
    pub fn format_detailed(&self) -> String {
        match self {
            SokuError::Parse { message, context } => {
                self.format_error_with_context("Parse Error", message, context)
            }
            SokuError::Build { message, context } => {
                self.format_error_with_context("Build Error", message, context)
            }
            _ => self.to_string(),
        }
    }

    fn format_error_with_context(
        &self,
        error_type: &str,
        message: &str,
        context: &Option<ErrorContext>,
    ) -> String {
        let mut output = format!("❌ {}: {}", error_type, message);

        if let Some(ctx) = context {
            if let Some(ref file_path) = ctx.file_path {
                output.push_str(&format!("\n📁 File: {}", file_path.display()));
            }

            if let (Some(line), Some(column)) = (ctx.line, ctx.column) {
                output.push_str(&format!("\n📍 Location: line {}, column {}", line, column));
            }

            if let Some(ref snippet) = ctx.code_snippet {
                output.push_str(&format!(
                    "\n📝 Code:\n{}",
                    self.format_code_snippet(snippet, ctx.line)
                ));
            }
        }

        output
    }

    fn format_code_snippet(&self, snippet: &str, error_line: Option<usize>) -> String {
        let lines: Vec<&str> = snippet.lines().collect();
        let mut output = String::new();

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;
            let is_error_line = error_line == Some(line_num);

            if is_error_line {
                output.push_str(&format!("→ {:3} │ {}\n", line_num, line));
                output.push_str(&format!("     │ {}\n", "^^^".repeat(line.len().min(20))));
            } else {
                output.push_str(&format!("  {:3} │ {}\n", line_num, line));
            }
        }

        output
    }
}

pub type Result<T> = std::result::Result<T, SokuError>;

impl From<regex::Error> for SokuError {
    fn from(err: regex::Error) -> Self {
        SokuError::parse(format!("Regex error: {}", err))
    }
}

impl From<anyhow::Error> for SokuError {
    fn from(err: anyhow::Error) -> Self {
        SokuError::build(err.to_string())
    }
}
