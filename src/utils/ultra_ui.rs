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
        self.clear_screen();

        // Ultra ASCII Art with gradient colors
        let banner = r#"
████████████████████████████████████████████████████████████████████████████████
██                                                                            ██
██  ██    ██ ██   ████████ ████████   █████                                   ██
██  ██    ██ ██      ██    ██     ██ ██   ██                                  ██
██  ██    ██ ██      ██    ████████  ███████                                  ██
██  ██    ██ ██      ██    ██   ██   ██   ██                                  ██
██   ██████  ████████████ ██    ██  ██   ██                                  ██
██                                                                            ██
██    🚀 THE FASTEST BUNDLER IN THE UNIVERSE 🚀                              ██
██                                                                            ██
████████████████████████████████████████████████████████████████████████████████"#;

        // Print banner with gradient effect
        for (i, line) in banner.lines().enumerate() {
            let color = match i % 6 {
                0 => Color::Magenta,
                1 => Color::Blue,
                2 => Color::Cyan,
                3 => Color::Green,
                4 => Color::Yellow,
                5 => Color::Red,
                _ => Color::White,
            };

            let _ = io::stdout()
                .execute(SetForegroundColor(color))
                .unwrap()
                .execute(Print(line))
                .unwrap()
                .execute(Print("\n"));
        }

        // Quick initialization message
        println!("\n⚡ {} {}", "Ultra Engine".bright_cyan().bold(), "READY!".bright_green().bold());
    }


    pub fn show_file_discovery(&self, js_count: usize, css_count: usize) {
        println!("📦 {} {} JS modules, {} CSS files",
            "Found:".bright_white().bold(),
            js_count.to_string().bright_yellow().bold(),
            css_count.to_string().bright_yellow().bold()
        );
    }

    pub fn show_processing_phase(&self, modules: &[String], phase: &str) {
        println!("{} {} {} modules",
            phase.bright_cyan().bold(),
            "processed".bright_white(),
            modules.len().to_string().bright_yellow().bold()
        );
    }

    pub fn show_tree_shaking_analysis(&self, modules_count: usize) {
        println!("🌳 {} {} modules",
            "Tree shaking analyzed".bright_green().bold(),
            modules_count.to_string().bright_yellow().bold()
        );
    }

    pub fn show_epic_completion(&self, stats: CompletionStats) {
        let build_time = self.start_time.elapsed();

        // Simple completion header
        println!("\n{}", "✅ BUILD COMPLETED!".bright_green().bold());
        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan());

        // Essential build info
        println!("⚡ {} {:.2}ms",
            "Build time:".bright_white().bold(),
            build_time.as_secs_f64() * 1000.0
        );

        println!("📦 {} {} JS, {} CSS",
            "Files:".bright_white().bold(),
            stats.js_count.to_string().bright_yellow(),
            stats.css_count.to_string().bright_yellow()
        );

        if stats.tree_shaking_info != "disabled (fast mode)" {
            println!("🌳 {} {}",
                "Tree shaking:".bright_white().bold(),
                stats.tree_shaking_info.bright_green()
            );
        }

        // Output files
        println!("💾 {} {}",
            "Generated:".bright_white().bold(),
            stats.output_files.iter()
                .map(|f| format!("{} ({:.1} KB)", f.name, f.size as f64 / 1024.0))
                .collect::<Vec<_>>()
                .join(", ")
                .bright_cyan()
        );

        println!();
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