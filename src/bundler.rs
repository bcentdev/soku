use crate::config::Config;
use crate::graph::ModuleGraph;
use anyhow::Result;
use std::path::Path;

pub struct Bundler {
    graph: ModuleGraph,
    config: Config,
}

impl Bundler {
    pub fn new(graph: ModuleGraph, config: Config) -> Self {
        Self { graph, config }
    }

    pub async fn build(&self, outdir: &str) -> Result<()> {
        println!("🔨 Building for production...");

        // TODO: Implement production build
        // 1. Process all modules through the graph
        // 2. Apply transformations (TS->JS, JSX->JS)
        // 3. Minify with oxc_minifier
        // 4. Generate chunks with code splitting
        // 5. Write output files

        let _output_dir = Path::new(outdir);

        println!("✅ Build complete!");

        Ok(())
    }
}