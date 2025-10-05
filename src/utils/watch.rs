// Watch mode for Ultra Bundler
// Monitors file changes and triggers automatic rebuilds

use crate::core::models::BuildConfig;
use crate::core::interfaces::BuildService;
use crate::utils::{Result, Logger, UltraError};
use notify::{Watcher, RecursiveMode, Event, EventKind, RecommendedWatcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};
use std::collections::HashSet;

/// Configuration for watch mode
#[derive(Debug, Clone)]
pub struct WatchConfig {
    /// Paths to watch for changes
    pub watch_paths: Vec<PathBuf>,
    /// Debounce delay in milliseconds
    pub debounce_ms: u64,
    /// Whether to clear console on rebuild
    pub clear_console: bool,
    /// Whether to show detailed logging
    pub verbose: bool,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            watch_paths: vec![PathBuf::from(".")],
            debounce_ms: 100,
            clear_console: false,
            verbose: false,
        }
    }
}

/// File watcher that monitors changes and triggers rebuilds
pub struct UltraWatcher {
    config: WatchConfig,
    build_config: BuildConfig,
}

impl UltraWatcher {
    /// Create a new file watcher
    pub fn new(config: WatchConfig, build_config: BuildConfig) -> Self {
        Self {
            config,
            build_config,
        }
    }

    /// Start watching for file changes
    pub async fn watch<B: BuildService>(&self, build_service: &mut B) -> Result<()> {
        Logger::info("👀 Watch mode started - monitoring for changes...");
        Logger::info(&format!("   Watching: {}", self.config.watch_paths.iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")));
        Logger::info("   Press Ctrl+C to stop");

        // Create channel for file system events
        let (tx, rx) = channel();

        // Create watcher
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            notify::Config::default(),
        ).map_err(|e| UltraError::Build {
            message: format!("Failed to create watcher: {}", e),
            context: None,
        })?;

        // Watch configured paths
        for path in &self.config.watch_paths {
            if path.exists() {
                watcher.watch(path, RecursiveMode::Recursive)
                    .map_err(|e| UltraError::Build {
                        message: format!("Failed to watch {}: {}", path.display(), e),
                        context: None,
                    })?;
            }
        }

        // Do initial build
        Logger::info("\n🔨 Initial build...");
        match build_service.build(&self.build_config).await {
            Ok(_) => Logger::info("✅ Initial build complete\n"),
            Err(e) => Logger::error(&format!("❌ Initial build failed: {}\n", e)),
        }

        // Process file events
        self.process_events(rx, build_service).await?;

        Ok(())
    }

    /// Process file system events with debouncing
    async fn process_events<B: BuildService>(
        &self,
        rx: Receiver<Event>,
        build_service: &mut B,
    ) -> Result<()> {
        let mut changed_files = HashSet::new();
        let mut last_change_time = Instant::now();
        let debounce_duration = Duration::from_millis(self.config.debounce_ms);

        // Spawn a task to handle Ctrl+C
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            let _ = shutdown_tx.send(()).await;
        });

        loop {
            // Check for shutdown signal
            if shutdown_rx.try_recv().is_ok() {
                Logger::info("\n👋 Stopping watch mode...");
                break;
            }

            // Try to receive events with timeout
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(event) => {
                    // Filter relevant events
                    if !self.should_ignore_event(&event) {
                        // Collect changed files
                        for path in &event.paths {
                            if self.is_source_file(path) {
                                changed_files.insert(path.clone());
                                last_change_time = Instant::now();

                                if self.config.verbose {
                                    Logger::debug(&format!("Changed: {}", path.display()));
                                }
                            }
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Check if enough time has passed since last change
                    if !changed_files.is_empty()
                        && last_change_time.elapsed() >= debounce_duration
                    {
                        self.trigger_rebuild(&changed_files, build_service).await;
                        changed_files.clear();
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    Logger::warn("Watch channel disconnected");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Trigger a rebuild
    async fn trigger_rebuild<B: BuildService>(
        &self,
        changed_files: &HashSet<PathBuf>,
        build_service: &mut B,
    ) {
        if self.config.clear_console {
            // Clear console (platform-specific)
            print!("\x1B[2J\x1B[1;1H");
        }

        Logger::info(&format!("\n🔄 Rebuilding... ({} files changed)", changed_files.len()));

        if self.config.verbose {
            for path in changed_files {
                Logger::debug(&format!("  • {}", path.display()));
            }
        }

        let start = Instant::now();

        match build_service.build(&self.build_config).await {
            Ok(result) => {
                let elapsed = start.elapsed();
                Logger::info(&format!(
                    "✅ Rebuild complete in {:.0}ms ({} JS, {} CSS)\n",
                    elapsed.as_millis(),
                    result.js_modules_processed,
                    result.css_files_processed
                ));
            }
            Err(e) => {
                Logger::error(&format!("❌ Rebuild failed: {}\n", e));
            }
        }
    }

    /// Check if event should be ignored
    fn should_ignore_event(&self, event: &Event) -> bool {
        match &event.kind {
            // Ignore metadata-only changes
            EventKind::Access(_) | EventKind::Other => true,
            // Ignore temporary files and directories
            _ => event.paths.iter().any(|p| {
                let path_str = p.to_string_lossy();
                path_str.contains(".git")
                    || path_str.contains("node_modules")
                    || path_str.contains("dist")
                    || path_str.contains(".soku-cache")
                    || path_str.ends_with('~')
                    || path_str.ends_with(".swp")
                    || path_str.contains(".tmp")
            }),
        }
    }

    /// Check if path is a source file that should trigger rebuild
    fn is_source_file(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            matches!(
                ext,
                "js" | "jsx" | "ts" | "tsx" | "css" | "json" | "mjs" | "cjs"
            )
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_config_default() {
        let config = WatchConfig::default();
        assert_eq!(config.debounce_ms, 100);
        assert!(!config.clear_console);
        assert!(!config.verbose);
    }

    #[test]
    fn test_is_source_file() {
        let config = WatchConfig::default();
        let build_config = BuildConfig {
            root: PathBuf::from("."),
            outdir: PathBuf::from("dist"),
            enable_tree_shaking: false,
            enable_minification: false,
            enable_source_maps: false,
            enable_code_splitting: false,
            max_chunk_size: None,
            mode: "development".to_string(),
            alias: std::collections::HashMap::new(),
            external: Vec::new(),
            vendor_chunk: false,
            entries: std::collections::HashMap::new(),
        };
        let watcher = UltraWatcher::new(config, build_config);

        assert!(watcher.is_source_file(Path::new("test.js")));
        assert!(watcher.is_source_file(Path::new("test.tsx")));
        assert!(watcher.is_source_file(Path::new("test.css")));
        assert!(!watcher.is_source_file(Path::new("test.txt")));
        assert!(!watcher.is_source_file(Path::new("README.md")));
    }
}
