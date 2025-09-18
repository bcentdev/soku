use tracing::{info, warn, error, debug};
use std::time::Instant;

pub struct Logger;

impl Logger {
    pub fn init() {
        tracing_subscriber::fmt()
            .with_env_filter("ultra=debug")
            .with_target(false)
            .init();
    }

    pub fn build_start(root: &str, outdir: &str) {
        info!("🔨 Ultra Bundler - Production Build");
        info!("═══════════════════════════════════════");
        info!("📁 Input: {}", root);
        info!("📦 Output: {}", outdir);
        info!("🎯 Target: Sub-2s builds");
    }

    pub fn scanning_files() {
        info!("📁 Scanning project files...");
    }

    pub fn found_files(js_count: usize, css_count: usize) {
        info!("📦 Found {} JS modules, {} CSS files", js_count, css_count);
    }

    pub fn tree_shaking_enabled() {
        info!("🌳 Initializing tree shaking analysis...");
    }

    pub fn tree_shaking_disabled() {
        info!("⚡ Tree shaking disabled - using fast build mode");
    }

    pub fn analyzing_module(name: &str) {
        debug!("🔍 Analyzing module: {}", name);
    }

    pub fn processing_file(name: &str, mode: &str) {
        debug!("⚡ Processing: {} ({})", name, mode);
    }

    pub fn processing_css(name: &str) {
        debug!("🎨 Processing CSS: {}", name);
    }


    pub fn build_complete(
        js_count: usize,
        css_count: usize,
        tree_shaking_stats: Option<&str>,
        build_time: std::time::Duration,
        outdir: &str,
    ) {
        info!("");
        info!("📊 Build Statistics:");
        info!("  • JS modules processed: {}", js_count);
        info!("  • CSS files processed: {}", css_count);

        if let Some(stats) = tree_shaking_stats {
            info!("  • {}", stats);
        } else {
            info!("  • Tree shaking: disabled (fast mode)");
        }

        info!("  • Build time: {:.2?}", build_time);
        info!("  • Output directory: {}", outdir);
        info!("");
        info!("✅ Real build completed successfully!");
        info!("🚀 Ultra with oxc + Lightning CSS");
    }

    pub fn error(msg: &str) {
        error!("❌ {}", msg);
    }

    pub fn warn(msg: &str) {
        warn!("⚠️  {}", msg);
    }
}

pub struct Timer {
    start: Instant,
    name: String,
}

impl Timer {
    pub fn start(name: &str) -> Self {
        debug!("⏱️  Starting: {}", name);
        Self {
            start: Instant::now(),
            name: name.to_string(),
        }
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        debug!("⏱️  Completed: {} in {:.2?}", self.name, self.elapsed());
    }
}