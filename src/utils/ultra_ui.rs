use colored::*;
use console::Term;
use crossterm::{
    style::{Color, Print, SetForegroundColor},
    ExecutableCommand,
};
use std::io;
use std::time::Instant;

pub struct UltraUI {
    term: Term,
    start_time: Instant,
}

impl UltraUI {
    pub fn new() -> Self {
        let term = Term::stdout();
        Self {
            term,
            start_time: Instant::now(),
        }
    }

    pub fn show_epic_banner(&self) {
        // Simple, clean output like Vite
        println!("\n  {} {}", "ULTRA".bright_cyan().bold(), "v0.1.0".bright_white());
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

        println!();
        println!("  {} built in {}",
            "✓".bright_green(),
            format!("{:.0}ms", build_time.as_secs_f64() * 1000.0).bright_white().bold()
        );
    }


    fn clear_screen(&self) {
        let _ = self.term.clear_screen();
    }
}

#[derive(Clone)]
pub struct CompletionStats {
    pub js_count: usize,
    pub css_count: usize,
    pub tree_shaking_info: String,
    pub output_files: Vec<OutputFileInfo>,
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