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

        // Show detailed timing breakdown if available
        if let Some(timing) = &stats.timing_breakdown {
            self.show_timing_breakdown(timing);
        }
    }

    pub fn show_timing_breakdown(&self, timing: &TimingBreakdown) {
        println!();
        println!("  {} Timing breakdown:",
            "⏱️".bright_yellow()
        );

        let phases = [
            ("File scanning", timing.file_scan_ms),
            ("JS processing", timing.js_processing_ms),
            ("CSS processing", timing.css_processing_ms),
            ("Tree shaking", timing.tree_shaking_ms),
            ("Minification", timing.minification_ms),
            ("Output writing", timing.output_write_ms),
        ];

        for (phase, time_ms) in phases {
            if time_ms > 0 {
                println!("    {} {}ms",
                    format!("{}:", phase).bright_blue(),
                    time_ms.to_string().bright_white()
                );
            }
        }
    }
}

#[derive(Clone)]
pub struct CompletionStats {
    pub output_files: Vec<OutputFileInfo>,
    pub node_modules_optimized: Option<usize>,
    pub timing_breakdown: Option<TimingBreakdown>,
}

#[derive(Clone)]
pub struct OutputFileInfo {
    pub name: String,
    pub size: usize,
}

#[derive(Clone, Debug)]
pub struct TimingBreakdown {
    pub file_scan_ms: u64,
    pub js_processing_ms: u64,
    pub css_processing_ms: u64,
    pub tree_shaking_ms: u64,
    pub minification_ms: u64,
    pub output_write_ms: u64,
}

impl Default for UltraUI {
    fn default() -> Self {
        Self::new()
    }
}