// Processors module
pub mod common;  // Shared functionality between processors
pub mod js_processor;
pub mod css_processor;
pub mod tree_shaker;
pub mod ast_tree_shaker;
pub mod enhanced_js_processor;
pub mod minifier;
pub mod code_splitter;

// Re-export unified processing components (recommended for new code)
pub use common::{
    ProcessingStrategy,
    ProcessingOptions,
    UnifiedJsProcessor,
};

// Re-export legacy processors (deprecated - use UnifiedJsProcessor instead)
pub use js_processor::*;
pub use css_processor::*;
pub use tree_shaker::*;
pub use ast_tree_shaker::*;
pub use minifier::*;
pub use code_splitter::*;
pub use enhanced_js_processor::*;