use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    pub module_id: String,
    pub importer: Option<String>,
    pub conditions: Vec<String>,
}

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;

    fn resolve(&self, _id: &str, _importer: Option<&str>, _context: &PluginContext) -> Result<Option<String>> {
        Ok(None)
    }

    fn load(&self, _id: &str, _context: &PluginContext) -> Result<Option<String>> {
        Ok(None)
    }

    fn transform(&self, _code: &str, _id: &str, _context: &PluginContext) -> Result<Option<String>> {
        Ok(None)
    }
}

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn add_plugin(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    pub fn resolve(&self, id: &str, importer: Option<&str>, context: &PluginContext) -> Result<Option<String>> {
        for plugin in &self.plugins {
            if let Some(result) = plugin.resolve(id, importer, context)? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    pub fn load(&self, id: &str, context: &PluginContext) -> Result<Option<String>> {
        for plugin in &self.plugins {
            if let Some(result) = plugin.load(id, context)? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    pub fn transform(&self, code: &str, id: &str, context: &PluginContext) -> Result<String> {
        let mut current_code = code.to_string();

        for plugin in &self.plugins {
            if let Some(transformed) = plugin.transform(&current_code, id, context)? {
                current_code = transformed;
            }
        }

        Ok(current_code)
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}