// WebAssembly support for Ultra Bundler
// Handles WASM module loading and integration

use crate::utils::Result;
use std::path::Path;

/// WebAssembly module processor
#[allow(dead_code)] // Part of public API for WASM support
pub struct WasmProcessor {
    /// Base URL for WASM files (relative to bundle)
    pub base_url: String,
}

#[allow(dead_code)] // Part of public API
impl WasmProcessor {
    /// Create a new WASM processor
    pub fn new() -> Self {
        Self {
            base_url: "./".to_string(),
        }
    }

    /// Check if a file is a WASM module
    pub fn is_wasm_module(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("wasm"))
            .unwrap_or(false)
    }

    /// Generate JavaScript loader code for a WASM module
    ///
    /// This generates code that:
    /// 1. Fetches the WASM file
    /// 2. Compiles and instantiates it
    /// 3. Exports the instance
    pub fn generate_loader_code(&self, wasm_path: &Path, module_name: &str) -> Result<String> {
        let wasm_filename = wasm_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("module.wasm");

        let loader_code = format!(
            r#"// WebAssembly Module: {}
let _wasmInstance_{} = null;
let _wasmExports_{} = null;

async function _loadWasm_{}() {{
  if (_wasmInstance_{}) return _wasmExports_{};

  try {{
    const wasmUrl = '{}{}';

    // Fetch and instantiate WASM module
    const response = await fetch(wasmUrl);
    const bytes = await response.arrayBuffer();
    const {{ instance }} = await WebAssembly.instantiate(bytes, {{}});

    _wasmInstance_{} = instance;
    _wasmExports_{} = instance.exports;

    return _wasmExports_{};
  }} catch (error) {{
    console.error('Failed to load WASM module {}:', error);
    throw error;
  }}
}}

// Export lazy loader
export const {} = {{
  load: _loadWasm_{},
  get exports() {{
    if (!_wasmExports_{}) {{
      throw new Error('WASM module {} not loaded yet. Call await {}.load() first.');
    }}
    return _wasmExports_{};
  }}
}};
"#,
            wasm_filename,
            module_name,
            module_name,
            module_name,
            module_name,
            module_name,
            self.base_url,
            wasm_filename,
            module_name,
            module_name,
            module_name,
            wasm_filename,
            module_name,
            module_name,
            module_name,
            module_name,
            module_name,
            module_name
        );

        Ok(loader_code)
    }

    /// Get module name from WASM file path
    pub fn get_module_name(path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| {
                // Convert to camelCase identifier
                s.replace('-', "_")
                    .replace('.', "_")
            })
            .unwrap_or_else(|| "wasmModule".to_string())
    }
}

impl Default for WasmProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a WASM module in the bundle
#[derive(Debug, Clone)]
#[allow(dead_code)] // Part of WASM support API
pub struct WasmModuleInfo {
    /// Original path to the WASM file
    pub path: std::path::PathBuf,
    /// Module name for JS export
    pub module_name: String,
    /// Generated loader code
    pub loader_code: String,
    /// Size of the WASM file in bytes
    pub size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_wasm_module() {
        assert!(WasmProcessor::is_wasm_module(&PathBuf::from("module.wasm")));
        assert!(WasmProcessor::is_wasm_module(&PathBuf::from("math.WASM")));
        assert!(WasmProcessor::is_wasm_module(&PathBuf::from("path/to/module.wasm")));
        assert!(!WasmProcessor::is_wasm_module(&PathBuf::from("module.js")));
        assert!(!WasmProcessor::is_wasm_module(&PathBuf::from("module.wat")));
    }

    #[test]
    fn test_get_module_name() {
        assert_eq!(WasmProcessor::get_module_name(&PathBuf::from("math.wasm")), "math");
        assert_eq!(WasmProcessor::get_module_name(&PathBuf::from("my-module.wasm")), "my_module");
        assert_eq!(WasmProcessor::get_module_name(&PathBuf::from("complex.name.wasm")), "complex_name");
    }

    #[test]
    fn test_generate_loader_code() {
        let processor = WasmProcessor::new();
        let path = PathBuf::from("math.wasm");
        let code = processor.generate_loader_code(&path, "math").unwrap();

        assert!(code.contains("_wasmInstance_math"));
        assert!(code.contains("_wasmExports_math"));
        assert!(code.contains("_loadWasm_math"));
        assert!(code.contains("export const math"));
        assert!(code.contains("await fetch"));
        assert!(code.contains("WebAssembly.instantiate"));
        assert!(code.contains("math.wasm"));
    }

    #[test]
    fn test_loader_code_structure() {
        let processor = WasmProcessor::new();
        let path = PathBuf::from("test.wasm");
        let code = processor.generate_loader_code(&path, "test").unwrap();

        // Check for essential components
        assert!(code.contains("async function"));
        assert!(code.contains("load:"));
        assert!(code.contains("get exports()"));
        assert!(code.contains("throw new Error"));
    }
}
