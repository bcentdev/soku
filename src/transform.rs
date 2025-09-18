use crate::cache::{ImportInfo, ImportKind, ModuleType};
use anyhow::{anyhow, Result};
use oxc_allocator::Allocator;
use oxc_ast::ast::{Declaration, ModuleDeclaration, Statement};
use oxc_codegen::{CodeGenerator, CodegenOptions};
use oxc_parser::{Parser, ParserOptions};
use oxc_semantic::{SemanticBuilder, SemanticBuilderReturn};
use oxc_span::SourceType;
use oxc_transformer::{Transformer, TransformOptions as OxcTransformOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformResult {
    pub code: String,
    pub source_map: Option<String>,
    pub imports: Vec<ImportInfo>,
    pub exports: Vec<String>,
    pub has_jsx: bool,
    pub has_typescript: bool,
}

#[derive(Debug, Clone)]
pub struct TransformOptions {
    pub jsx: JsxOptions,
    pub typescript: TypeScriptOptions,
    pub target: EcmaScriptTarget,
    pub module_format: ModuleFormat,
    pub minify: bool,
    pub source_map: bool,
}

#[derive(Debug, Clone)]
pub struct JsxOptions {
    pub pragma: String,
    pub pragma_frag: String,
    pub development: bool,
    pub refresh: bool, // React Fast Refresh
}

#[derive(Debug, Clone)]
pub struct TypeScriptOptions {
    pub jsx: bool,
    pub declaration: bool,
}

#[derive(Debug, Clone)]
pub enum EcmaScriptTarget {
    Es5,
    Es2015,
    Es2016,
    Es2017,
    Es2018,
    Es2019,
    Es2020,
    Es2021,
    Es2022,
    Es2023,
    EsNext,
}

#[derive(Debug, Clone)]
pub enum ModuleFormat {
    Esm,
    Cjs,
    Umd,
    Iife,
}

pub struct OxcTransformer {
    options: TransformOptions,
}

impl Default for TransformOptions {
    fn default() -> Self {
        Self {
            jsx: JsxOptions {
                pragma: "React.createElement".to_string(),
                pragma_frag: "React.Fragment".to_string(),
                development: true,
                refresh: false,
            },
            typescript: TypeScriptOptions {
                jsx: true,
                declaration: false,
            },
            target: EcmaScriptTarget::Es2020,
            module_format: ModuleFormat::Esm,
            minify: false,
            source_map: true,
        }
    }
}

impl OxcTransformer {
    pub fn new(options: TransformOptions) -> Self {
        Self { options }
    }

    pub fn transform(&self, source: &str, file_path: &Path) -> Result<TransformResult> {
        let allocator = Allocator::default();
        let source_type = self.detect_source_type(file_path);

        // Parse the source code
        let parser_options = ParserOptions::default()
            .with_jsx(source_type.is_jsx())
            .with_typescript(source_type.is_typescript());

        let parser_result = Parser::new(&allocator, source, source_type)
            .with_options(parser_options)
            .parse();

        if !parser_result.errors.is_empty() {
            let errors: Vec<String> = parser_result
                .errors
                .iter()
                .map(|e| format!("Parse error: {}", e))
                .collect();
            return Err(anyhow!("Parse errors: {}", errors.join(", ")));
        }

        let mut program = parser_result.program;

        // Extract imports and exports before transformation
        let (imports, exports) = self.analyze_module(&program, source)?;

        // For now, we'll do basic transformation
        // In production, we'd apply full oxc transformations here

        // Generate code
        let codegen_options = CodegenOptions {
            minify: self.options.minify,
            ..CodegenOptions::default()
        };

        let codegen_result = CodeGenerator::new()
            .with_options(codegen_options)
            .build(&program);

        Ok(TransformResult {
            code: codegen_result.source_text,
            source_map: codegen_result.source_map.map(|sm| sm.to_json_string()),
            imports,
            exports,
            has_jsx: source_type.is_jsx(),
            has_typescript: source_type.is_typescript(),
        })
    }

    fn detect_source_type(&self, file_path: &Path) -> SourceType {
        let extension = file_path.extension().and_then(|ext| ext.to_str());

        match extension {
            Some("ts") => SourceType::default().with_typescript(true),
            Some("tsx") => SourceType::default()
                .with_typescript(true)
                .with_jsx(true),
            Some("jsx") => SourceType::default().with_jsx(true),
            Some("mjs") => SourceType::default().with_module(true),
            Some("js") => {
                // Try to detect if it's a module based on imports/exports
                SourceType::default().with_module(true)
            }
            _ => SourceType::default(),
        }
    }

    fn build_oxc_transform_options(&self, source_type: SourceType) -> OxcTransformOptions {
        // Create basic transform options
        // The exact API may vary depending on oxc version
        OxcTransformOptions::default()
    }

    fn analyze_module(&self, program: &oxc_ast::ast::Program, source: &str) -> Result<(Vec<ImportInfo>, Vec<String>)> {
        let mut imports = Vec::new();
        let mut exports = Vec::new();

        for stmt in &program.body {
            match stmt {
                Statement::ModuleDeclaration(module_decl) => {
                    match &module_decl.unbox() {
                        ModuleDeclaration::ImportDeclaration(import_decl) => {
                            let specifier = import_decl.source.value.to_string();
                            let span = import_decl.span;

                            imports.push(ImportInfo {
                                specifier,
                                kind: ImportKind::Static,
                                source_location: Some((span.start as usize, span.end as usize)),
                            });
                        }
                        ModuleDeclaration::ExportAllDeclaration(_) => {
                            exports.push("*".to_string());
                        }
                        ModuleDeclaration::ExportDefaultDeclaration(_) => {
                            exports.push("default".to_string());
                        }
                        ModuleDeclaration::ExportNamedDeclaration(export_decl) => {
                            if let Some(declaration) = &export_decl.declaration {
                                match declaration {
                                    Declaration::VariableDeclaration(var_decl) => {
                                        for declarator in &var_decl.declarations {
                                            if let Some(id) = declarator.id.get_identifier() {
                                                exports.push(id.to_string());
                                            }
                                        }
                                    }
                                    Declaration::FunctionDeclaration(func_decl) => {
                                        if let Some(id) = &func_decl.id {
                                            exports.push(id.name.to_string());
                                        }
                                    }
                                    Declaration::ClassDeclaration(class_decl) => {
                                        if let Some(id) = &class_decl.id {
                                            exports.push(id.name.to_string());
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            // Handle export specifiers
                            if let Some(specifiers) = &export_decl.specifiers {
                                for specifier in specifiers {
                                    exports.push(specifier.exported.name().to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Statement::ExpressionStatement(expr_stmt) => {
                    // Look for dynamic imports
                    if let Some(call_expr) = expr_stmt.expression.as_call_expression() {
                        if let Some(import_expr) = call_expr.callee.as_import() {
                            if let Some(first_arg) = call_expr.arguments.first() {
                                if let Some(literal) = first_arg.as_expression()
                                    .and_then(|e| e.as_string_literal()) {
                                    imports.push(ImportInfo {
                                        specifier: literal.value.to_string(),
                                        kind: ImportKind::Dynamic,
                                        source_location: Some((
                                            call_expr.span.start as usize,
                                            call_expr.span.end as usize,
                                        )),
                                    });
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok((imports, exports))
    }

    pub fn minify(&self, source: &str, file_path: &Path) -> Result<String> {
        let allocator = Allocator::default();
        let source_type = self.detect_source_type(file_path);

        let parser_result = Parser::new(&allocator, source, source_type)
            .parse();

        if !parser_result.errors.is_empty() {
            return Err(anyhow!("Parse errors during minification"));
        }

        let program = parser_result.program;

        // Use oxc minifier for production builds
        let minifier = oxc_minifier::Minifier::new();
        let minified_program = minifier.build(&allocator, &program);

        let codegen_result = CodeGenerator::new()
            .with_options(CodegenOptions {
                minify: true,
                ..CodegenOptions::default()
            })
            .build(&minified_program);

        Ok(codegen_result.source_text)
    }
}

pub fn detect_module_type_from_path(path: &Path) -> ModuleType {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("ts") => ModuleType::TypeScript,
        Some("tsx") => ModuleType::Tsx,
        Some("jsx") => ModuleType::Jsx,
        Some("js") | Some("mjs") => ModuleType::JavaScript,
        Some("css") => ModuleType::Css,
        Some("json") => ModuleType::Json,
        _ => ModuleType::Asset,
    }
}