// Processors module
pub mod asset_processor;
pub mod ast_tree_shaker;
pub mod code_splitter;
pub mod common; // Shared functionality between processors
pub mod css_processor;
pub mod dynamic_import_splitter;
pub mod enhanced_js_processor;
pub mod minifier;
pub mod scss_processor;
pub mod tree_shaker;

// Re-export unified processing components (recommended for new code)
pub use common::{ProcessingStrategy, UnifiedJsProcessor};

// Re-export processors
pub use asset_processor::*;
pub use ast_tree_shaker::*;
pub use code_splitter::*;
pub use css_processor::*;
pub use minifier::*;
pub use scss_processor::*;
pub use tree_shaker::*;
// dynamic_import_splitter available via mod but not re-exported (future integration)
