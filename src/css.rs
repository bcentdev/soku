use crate::cache::{ImportInfo, ImportKind};
use anyhow::{anyhow, Result};
// Temporarily comment out lightningcss imports due to API changes
// use lightningcss::{
//     stylesheet::{StyleSheet, ParserOptions},
//     targets::{Browsers, Targets},
//     visit_types,
//     visitor::{Visit, VisitTypes, Visitor},
//     rules::CssRule,
//     values::url::Url,
//     properties::Property,
//     declaration::DeclarationBlock,
// };
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssTransformResult {
    pub code: String,
    pub source_map: Option<String>,
    pub imports: Vec<ImportInfo>,
    pub exports: HashMap<String, String>, // CSS Modules exports
    pub dependencies: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CssOptions {
    pub minify: bool,
    pub modules: bool,
    pub autoprefixer: bool,
    pub targets: Option<Browsers>,
    pub nesting: bool,
    pub custom_properties: bool,
}

impl Default for CssOptions {
    fn default() -> Self {
        Self {
            minify: false,
            modules: false,
            autoprefixer: true,
            targets: Some(Browsers::default()),
            nesting: true,
            custom_properties: true,
        }
    }
}

pub struct LightningCssProcessor {
    options: CssOptions,
}

impl LightningCssProcessor {
    pub fn new(options: CssOptions) -> Self {
        Self { options }
    }

    pub fn transform(&self, source: &str, file_path: &Path) -> Result<CssTransformResult> {
        let filename = file_path.to_string_lossy().to_string();

        // Parse CSS with Lightning CSS
        let mut stylesheet = StyleSheet::parse(
            &filename,
            source,
            ParserOptions {
                nesting: self.options.nesting,
                custom_media: true,
                css_modules: self.options.modules,
                ..ParserOptions::default()
            },
        ).map_err(|e| anyhow!("CSS parse error: {:?}", e))?;

        // Extract imports and dependencies
        let mut import_visitor = ImportVisitor::new();
        stylesheet.visit(&mut import_visitor)?;

        let imports = import_visitor.imports;
        let dependencies = import_visitor.dependencies;

        // Apply transformations
        if self.options.autoprefixer {
            if let Some(targets) = &self.options.targets {
                stylesheet.transform_features(Targets::from(*targets))?;
            }
        }

        // Generate output
        let to_css_options = lightningcss::stylesheet::ToCssOptions {
            minify: self.options.minify,
            source_map: true,
            ..lightningcss::stylesheet::ToCssOptions::default()
        };

        let result = stylesheet.to_css(to_css_options)
            .map_err(|e| anyhow!("CSS generation error: {:?}", e))?;

        // Handle CSS Modules
        let exports = if self.options.modules {
            stylesheet.css_modules.as_ref()
                .map(|modules| {
                    modules.iter()
                        .map(|(key, value)| (key.clone(), value.name.clone()))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            HashMap::new()
        };

        Ok(CssTransformResult {
            code: result.code,
            source_map: result.map.map(|m| m.to_json().unwrap()),
            imports,
            exports,
            dependencies,
        })
    }

    pub fn minify(&self, source: &str, file_path: &Path) -> Result<String> {
        let filename = file_path.to_string_lossy().to_string();

        let stylesheet = StyleSheet::parse(
            &filename,
            source,
            ParserOptions::default(),
        ).map_err(|e| anyhow!("CSS parse error: {:?}", e))?;

        let result = stylesheet.to_css(lightningcss::stylesheet::ToCssOptions {
            minify: true,
            ..lightningcss::stylesheet::ToCssOptions::default()
        }).map_err(|e| anyhow!("CSS minification error: {:?}", e))?;

        Ok(result.code)
    }

    pub fn transform_css_modules(&self, source: &str, file_path: &Path) -> Result<CssTransformResult> {
        let mut options = self.options.clone();
        options.modules = true;

        let processor = LightningCssProcessor::new(options);
        processor.transform(source, file_path)
    }

    pub fn extract_urls(&self, source: &str, file_path: &Path) -> Result<Vec<String>> {
        let filename = file_path.to_string_lossy().to_string();

        let stylesheet = StyleSheet::parse(
            &filename,
            source,
            ParserOptions::default(),
        ).map_err(|e| anyhow!("CSS parse error: {:?}", e))?;

        let mut url_visitor = UrlVisitor::new();
        stylesheet.visit(&mut url_visitor)?;

        Ok(url_visitor.urls)
    }
}

// Visitor to extract @import statements and dependencies
struct ImportVisitor {
    imports: Vec<ImportInfo>,
    dependencies: Vec<PathBuf>,
}

impl ImportVisitor {
    fn new() -> Self {
        Self {
            imports: Vec::new(),
            dependencies: Vec::new(),
        }
    }
}

impl<'i> Visitor<'i> for ImportVisitor {
    type Error = lightningcss::error::Error<lightningcss::error::ParserError<'i>>;

    const VISIT_TYPES: VisitTypes = visit_types!(RULES);

    fn visit_rule(&mut self, rule: &mut CssRule<'i>) -> Result<(), Self::Error> {
        match rule {
            CssRule::Import(import_rule) => {
                let url = import_rule.url.as_ref();

                // Extract the URL string
                let url_string = match url {
                    Url::Literal(s) => s.clone(),
                    Url::Raw(s) => s.clone(),
                };

                // Skip data URLs and external URLs
                if !url_string.starts_with("data:") &&
                   !url_string.starts_with("http:") &&
                   !url_string.starts_with("https:") {

                    self.imports.push(ImportInfo {
                        specifier: url_string.clone(),
                        kind: ImportKind::Css,
                        source_location: None,
                    });

                    // Add as dependency for file watching
                    self.dependencies.push(PathBuf::from(url_string));
                }
            }
            _ => {}
        }

        Ok(())
    }
}

// Visitor to extract URL references (for asset dependencies)
struct UrlVisitor {
    urls: Vec<String>,
}

impl UrlVisitor {
    fn new() -> Self {
        Self {
            urls: Vec::new(),
        }
    }
}

impl<'i> Visitor<'i> for UrlVisitor {
    type Error = lightningcss::error::Error<lightningcss::error::ParserError<'i>>;

    const VISIT_TYPES: VisitTypes = visit_types!(PROPERTIES);

    fn visit_property(&mut self, property: &mut Property<'i>) -> Result<(), Self::Error> {
        match property {
            Property::Background(bg) => {
                for image in &bg.background_image {
                    if let lightningcss::properties::background::BackgroundImage::Url(url) = image {
                        let url_string = match url {
                            Url::Literal(s) => s.clone(),
                            Url::Raw(s) => s.clone(),
                        };

                        // Skip data URLs and external URLs
                        if !url_string.starts_with("data:") &&
                           !url_string.starts_with("http:") &&
                           !url_string.starts_with("https:") {
                            self.urls.push(url_string);
                        }
                    }
                }
            }
            Property::BackgroundImage(images) => {
                for image in images {
                    if let lightningcss::properties::background::BackgroundImage::Url(url) = image {
                        let url_string = match url {
                            Url::Literal(s) => s.clone(),
                            Url::Raw(s) => s.clone(),
                        };

                        if !url_string.starts_with("data:") &&
                           !url_string.starts_with("http:") &&
                           !url_string.starts_with("https:") {
                            self.urls.push(url_string);
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}

// CSS Modules class name generator
pub struct CssModulesGenerator {
    pattern: String,
    counter: std::sync::atomic::AtomicUsize,
}

impl CssModulesGenerator {
    pub fn new(pattern: String) -> Self {
        Self {
            pattern,
            counter: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn generate_class_name(&self, original: &str, file_path: &Path) -> String {
        let file_name = file_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let hash = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};

            let mut hasher = DefaultHasher::new();
            file_path.hash(&mut hasher);
            original.hash(&mut hasher);
            format!("{:x}", hasher.finish())[..6].to_string()
        };

        let counter = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        self.pattern
            .replace("[name]", file_name)
            .replace("[local]", original)
            .replace("[hash]", &hash)
            .replace("[counter]", &counter.to_string())
    }
}

impl Default for CssModulesGenerator {
    fn default() -> Self {
        Self::new("[name]_[local]_[hash]".to_string())
    }
}

// PostCSS-style plugin system for CSS transformations
pub trait CssPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn transform(&self, css: &str, file_path: &Path) -> Result<String>;
}

pub struct CssPluginManager {
    plugins: Vec<Box<dyn CssPlugin>>,
}

impl CssPluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn add_plugin(&mut self, plugin: Box<dyn CssPlugin>) {
        self.plugins.push(plugin);
    }

    pub fn process(&self, css: &str, file_path: &Path) -> Result<String> {
        let mut result = css.to_string();

        for plugin in &self.plugins {
            result = plugin.transform(&result, file_path)?;
        }

        Ok(result)
    }
}

impl Default for CssPluginManager {
    fn default() -> Self {
        Self::new()
    }
}

// Built-in plugins
pub struct AutoprefixerPlugin {
    targets: Browsers,
}

impl AutoprefixerPlugin {
    pub fn new(targets: Browsers) -> Self {
        Self { targets }
    }
}

impl CssPlugin for AutoprefixerPlugin {
    fn name(&self) -> &str {
        "autoprefixer"
    }

    fn transform(&self, css: &str, file_path: &Path) -> Result<String> {
        let processor = LightningCssProcessor::new(CssOptions {
            autoprefixer: true,
            targets: Some(self.targets),
            ..CssOptions::default()
        });

        let result = processor.transform(css, file_path)?;
        Ok(result.code)
    }
}

pub struct CssNestingPlugin;

impl CssPlugin for CssNestingPlugin {
    fn name(&self) -> &str {
        "css-nesting"
    }

    fn transform(&self, css: &str, file_path: &Path) -> Result<String> {
        let processor = LightningCssProcessor::new(CssOptions {
            nesting: true,
            ..CssOptions::default()
        });

        let result = processor.transform(css, file_path)?;
        Ok(result.code)
    }
}

// CSS bundling for production
pub struct CssBundler {
    processor: LightningCssProcessor,
}

impl CssBundler {
    pub fn new(options: CssOptions) -> Self {
        Self {
            processor: LightningCssProcessor::new(options),
        }
    }

    pub fn bundle_css_files(&self, files: &[PathBuf]) -> Result<String> {
        let mut combined_css = String::new();

        for file_path in files {
            let content = std::fs::read_to_string(file_path)?;
            let result = self.processor.transform(&content, file_path)?;
            combined_css.push_str(&result.code);
            combined_css.push('\n');
        }

        // Final minification pass
        if self.processor.options.minify {
            let temp_path = PathBuf::from("bundle.css");
            self.processor.minify(&combined_css, &temp_path)
        } else {
            Ok(combined_css)
        }
    }
}