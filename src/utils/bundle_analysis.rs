// Bundle analysis for Soku Bundler
// Provides size analysis and visualization of bundle contents

use crate::core::models::{ModuleInfo, ModuleType, BuildResult};
use crate::utils::{Result, Logger};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Statistics for a single module in the bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStats {
    /// Module path
    pub path: PathBuf,
    /// Module type
    pub module_type: ModuleType,
    /// Original size in bytes
    pub original_size: usize,
    /// Size in bundle (after processing)
    pub bundle_size: usize,
    /// Percentage of total bundle
    pub percentage: f64,
    /// Dependencies
    pub dependencies: Vec<String>,
}

/// Statistics by file type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeStats {
    /// File type
    pub file_type: String,
    /// Number of files
    pub count: usize,
    /// Total size in bytes
    pub total_size: usize,
    /// Percentage of total bundle
    pub percentage: f64,
}

/// Complete bundle analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleAnalysis {
    /// Total bundle size
    pub total_size: usize,
    /// Total number of modules
    pub total_modules: usize,
    /// Module statistics (sorted by size, descending)
    pub modules: Vec<ModuleStats>,
    /// Statistics by type
    pub type_stats: Vec<TypeStats>,
    /// Largest modules (top 10)
    pub largest_modules: Vec<ModuleStats>,
    /// Total dependencies
    pub total_dependencies: usize,
}

impl BundleAnalysis {
    /// Create a new bundle analysis from modules and build result
    pub fn analyze(modules: &[ModuleInfo], _result: &BuildResult) -> Self {
        // Calculate module stats
        let mut module_stats: Vec<ModuleStats> = modules
            .iter()
            .map(|m| {
                let original_size = m.content.len();
                ModuleStats {
                    path: m.path.clone(),
                    module_type: m.module_type.clone(),
                    original_size,
                    bundle_size: original_size, // Approximation
                    percentage: 0.0, // Calculate later
                    dependencies: m.dependencies.clone(),
                }
            })
            .collect();

        // Calculate total size
        let total_size: usize = module_stats.iter().map(|m| m.bundle_size).sum();

        // Calculate percentages
        for module in &mut module_stats {
            module.percentage = if total_size > 0 {
                (module.bundle_size as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };
        }

        // Sort by size descending
        module_stats.sort_by(|a, b| b.bundle_size.cmp(&a.bundle_size));

        // Calculate type stats
        let mut type_map: HashMap<String, (usize, usize)> = HashMap::new();
        for module in &module_stats {
            let type_name = format!("{:?}", module.module_type);
            let entry = type_map.entry(type_name).or_insert((0, 0));
            entry.0 += 1; // count
            entry.1 += module.bundle_size; // size
        }

        let mut type_stats: Vec<TypeStats> = type_map
            .into_iter()
            .map(|(file_type, (count, size))| TypeStats {
                file_type,
                count,
                total_size: size,
                percentage: if total_size > 0 {
                    (size as f64 / total_size as f64) * 100.0
                } else {
                    0.0
                },
            })
            .collect();

        // Sort by size descending
        type_stats.sort_by(|a, b| b.total_size.cmp(&a.total_size));

        // Get top 10 largest modules
        let largest_modules = module_stats.iter().take(10).cloned().collect();

        // Count total dependencies
        let total_dependencies: usize = module_stats.iter().map(|m| m.dependencies.len()).sum();

        Self {
            total_size,
            total_modules: module_stats.len(),
            modules: module_stats,
            type_stats,
            largest_modules,
            total_dependencies,
        }
    }

    /// Generate a human-readable report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();

        report.push('\n');
        report.push_str("╔════════════════════════════════════════════════════════════╗\n");
        report.push_str("║              📊 BUNDLE ANALYSIS REPORT                    ║\n");
        report.push_str("╚════════════════════════════════════════════════════════════╝\n");
        report.push('\n');

        // Overview
        report.push_str("📦 OVERVIEW\n");
        report.push_str("─────────────────────────────────────────────────────────────\n");
        report.push_str(&format!("  Total Size:      {}\n", Self::format_size(self.total_size)));
        report.push_str(&format!("  Total Modules:   {}\n", self.total_modules));
        report.push_str(&format!("  Dependencies:    {}\n", self.total_dependencies));
        report.push('\n');

        // By Type
        report.push_str("📂 BY FILE TYPE\n");
        report.push_str("─────────────────────────────────────────────────────────────\n");
        for type_stat in &self.type_stats {
            let bar = Self::create_bar(type_stat.percentage, 30);
            report.push_str(&format!(
                "  {:12} {:>8} ({:>2} files) {:>6.1}% {}\n",
                type_stat.file_type,
                Self::format_size(type_stat.total_size),
                type_stat.count,
                type_stat.percentage,
                bar
            ));
        }
        report.push('\n');

        // Top 10 Largest Modules
        report.push_str("🔝 TOP 10 LARGEST MODULES\n");
        report.push_str("─────────────────────────────────────────────────────────────\n");
        for (i, module) in self.largest_modules.iter().enumerate() {
            let filename = module.path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            let bar = Self::create_bar(module.percentage, 20);
            report.push_str(&format!(
                "  {:2}. {:30} {:>8} {:>5.1}% {}\n",
                i + 1,
                Self::truncate(filename, 30),
                Self::format_size(module.bundle_size),
                module.percentage,
                bar
            ));
        }
        report.push('\n');

        report
    }

    /// Save analysis to JSON file
    pub fn save_json(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| crate::utils::SokuError::Build {
                message: format!("Failed to serialize analysis: {}", e),
                context: None,
            })?;

        std::fs::write(path, json)
            .map_err(crate::utils::SokuError::Io)?;

        Ok(())
    }

    /// Format bytes as human-readable size
    fn format_size(bytes: usize) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;

        if bytes == 0 {
            "0 B".to_string()
        } else if bytes < KB as usize {
            format!("{} B", bytes)
        } else if bytes < MB as usize {
            format!("{:.2} KB", bytes as f64 / KB)
        } else {
            format!("{:.2} MB", bytes as f64 / MB)
        }
    }

    /// Create a progress bar for visualization
    fn create_bar(percentage: f64, width: usize) -> String {
        let filled = ((percentage / 100.0) * width as f64) as usize;
        let empty = width.saturating_sub(filled);
        format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
    }

    /// Truncate string to max length
    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            format!("{:width$}", s, width = max_len)
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }
}

/// Display bundle analysis to console
pub fn display_analysis(analysis: &BundleAnalysis) {
    Logger::info(&analysis.generate_report());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_format_size() {
        assert_eq!(BundleAnalysis::format_size(0), "0 B");
        assert_eq!(BundleAnalysis::format_size(512), "512 B");
        assert_eq!(BundleAnalysis::format_size(1024), "1.00 KB");
        assert_eq!(BundleAnalysis::format_size(1536), "1.50 KB");
        assert_eq!(BundleAnalysis::format_size(1048576), "1.00 MB");
    }

    #[test]
    fn test_create_bar() {
        let bar = BundleAnalysis::create_bar(50.0, 10);
        assert_eq!(bar.chars().count(), 12); // 10 chars + 2 brackets
        assert!(bar.contains("█"));
        assert!(bar.contains("░"));
        assert!(bar.starts_with('['));
        assert!(bar.ends_with(']'));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(BundleAnalysis::truncate("short", 10), "short     ");
        assert_eq!(BundleAnalysis::truncate("very_long_filename.js", 10), "very_lo...");
    }

    #[test]
    fn test_bundle_analysis() {
        let modules = vec![
            ModuleInfo {
                path: PathBuf::from("main.js"),
                content: "a".repeat(1000),
                module_type: ModuleType::JavaScript,
                dependencies: vec!["./utils.js".to_string()],
                exports: vec![],
            },
            ModuleInfo {
                path: PathBuf::from("utils.js"),
                content: "b".repeat(500),
                module_type: ModuleType::JavaScript,
                dependencies: vec![],
                exports: vec![],
            },
        ];

        let result = BuildResult {
            success: true,
            js_modules_processed: 2,
            css_files_processed: 0,
            errors: vec![],
            warnings: vec![],
            tree_shaking_stats: None,
            build_time: std::time::Duration::from_millis(50),
            output_files: vec![],
            modules: modules.clone(),
        };

        let analysis = BundleAnalysis::analyze(&modules, &result);

        assert_eq!(analysis.total_modules, 2);
        assert_eq!(analysis.total_size, 1500);
        assert_eq!(analysis.total_dependencies, 1);
        assert_eq!(analysis.largest_modules.len(), 2);
        assert_eq!(analysis.type_stats.len(), 1);
    }
}
