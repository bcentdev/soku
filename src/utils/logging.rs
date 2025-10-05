use tracing::{error, info, warn};

pub struct Logger;

impl Logger {
    pub fn init() {
        tracing_subscriber::fmt()
            .with_env_filter("soku=info")
            .with_target(false)
            .without_time()
            .init();
    }

    #[allow(dead_code)] // Logging utility - may be used in future
    pub fn build_start(root: &str, outdir: &str) {
        info!("🔨 Soku Bundler - Production Build");
        info!("═══════════════════════════════════════");
        info!("📁 Input: {}", root);
        info!("📦 Output: {}", outdir);
        info!("🎯 Target: Sub-2s builds");
    }

    #[allow(dead_code)] // Logging utility - may be used in future
    pub fn scanning_files() {
        info!("📁 Scanning project files...");
    }

    pub fn found_files(_js_count: usize, _css_count: usize) {}

    pub fn tree_shaking_enabled() {}

    #[allow(dead_code)]
    pub fn tree_shaking_disabled() {}

    pub fn analyzing_module(_name: &str) {}

    pub fn processing_file(_name: &str, _mode: &str) {}

    pub fn processing_css(_name: &str) {}

    #[allow(dead_code)] // Logging utility - may be used in future
    pub fn processing_typescript(_name: &str) {}

    #[allow(dead_code)]
    pub fn debug(_message: &str) {}

    #[allow(dead_code)] // Logging utility - may be used in future
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
        info!("🚀 Soku with oxc + Lightning CSS");
    }

    pub fn error(msg: &str) {
        error!("❌ {}", msg);
    }

    pub fn warn(msg: &str) {
        warn!("⚠️  {}", msg);
    }

    pub fn info(msg: &str) {
        info!("{}", msg);
    }
}

pub struct Timer;

impl Timer {
    pub fn start(_name: &str) -> Self {
        Self
    }
}
