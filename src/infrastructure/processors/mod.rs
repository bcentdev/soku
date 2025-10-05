// Processors module
pub mod common;  // Shared functionality between processors
pub mod css_processor;
pub mod scss_processor;
pub mod tree_shaker;
pub mod ast_tree_shaker;
pub mod enhanced_js_processor;
pub mod minifier;
pub mod code_splitter;
pub mod asset_processor;
pub mod dynamic_import_splitter;

// Re-export unified processing components (recommended for new code)
pub use common::{
    ProcessingStrategy,
    UnifiedJsProcessor,
};

// Re-export processors
pub use css_processor::*;
pub use scss_processor::*;
pub use tree_shaker::*;
pub use ast_tree_shaker::*;
pub use minifier::*;
pub use code_splitter::*;
pub use asset_processor::*;
// dynamic_import_splitter available via mod but not re-exported (future integration)