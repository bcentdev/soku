use colored::*;
use std::time::Instant;

pub struct UltraUI {
    start_time: Instant,
}

impl UltraUI {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }

    pub fn show_epic_banner(&self) {
        // Simple, clean output like Vite
        println!("\n  {} {}", "ULTRA".bright_cyan().bold(), "v0.3.0".bright_white());
        println!();
    }


    pub fn show_file_discovery(&self, _js_count: usize, _css_count: usize) {}

    pub fn show_processing_phase(&self, _modules: &[String], _phase: &str) {}

    pub fn show_tree_shaking_analysis(&self, _modules_count: usize) {}

    pub fn show_epic_completion(&self, stats: CompletionStats) {
        let build_time = self.start_time.elapsed();

        // Vite-style clean output
        println!();
        for file in &stats.output_files {
            let size_kb = file.size as f64 / 1024.0;
            let size_str = if size_kb < 1.0 {
                format!("{:.2} B", file.size)
            } else {
                format!("{:.2} kB", size_kb)
            };

            println!("  {} {} {}",
                "dist/".bright_black(),
                file.name.bright_cyan(),
                format!("({})", size_str).bright_black()
            );
        }

        // Show node_modules optimization stats if present
        if let Some(node_count) = stats.node_modules_optimized {
            if node_count > 0 {
                println!();
                println!("  {} {} node_modules optimized",
                    "🌳".bright_green(),
                    node_count.to_string().bright_cyan().bold()
                );
            }
        }

        println!();
        println!("  {} built in {}",
            "✓".bright_green(),
            format!("{:.0}ms", build_time.as_secs_f64() * 1000.0).bright_white().bold()
        );
    }


}

#[derive(Clone)]
pub struct CompletionStats {
    pub output_files: Vec<OutputFileInfo>,
    pub node_modules_optimized: Option<usize>,
}

#[derive(Clone)]
pub struct OutputFileInfo {
    pub name: String,
    pub size: usize,
}

impl Default for UltraUI {
    fn default() -> Self {
        Self::new()
    }
}