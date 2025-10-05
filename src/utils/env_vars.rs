use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::utils::{Result, SokuError, Logger};

/// Environment variables manager for build-time variable injection
pub struct EnvVarsManager {
    variables: HashMap<String, String>,
}

impl EnvVarsManager {
    /// Create a new environment variables manager
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Load environment variables from .env files and system environment
    pub fn load_from_files(root: &Path, mode: &str) -> Result<Self> {
        let mut manager = Self::new();

        // Priority order (highest to lowest):
        // 1. .env.{mode}.local (highest priority, gitignored)
        // 2. .env.{mode}
        // 3. .env.local (gitignored)
        // 4. .env (committed to repo)

        let env_files = vec![
            root.join(".env"),
            root.join(".env.local"),
            root.join(format!(".env.{}", mode)),
            root.join(format!(".env.{}.local", mode)),
        ];

        // Load files in order (later files override earlier ones)
        for env_file in env_files {
            if env_file.exists() {
                manager.load_env_file(&env_file)?;
            }
        }

        // Add built-in variables
        manager.variables.insert("NODE_ENV".to_string(), mode.to_string());
        manager.variables.insert("MODE".to_string(), mode.to_string());

        // Add import.meta.env specific variables
        manager.variables.insert("DEV".to_string(),
            if mode == "development" { "true" } else { "false" }.to_string());
        manager.variables.insert("PROD".to_string(),
            if mode == "production" { "true" } else { "false" }.to_string());

        Logger::debug(&format!("Loaded {} environment variables", manager.variables.len()));

        Ok(manager)
    }

    /// Load variables from a specific .env file
    fn load_env_file(&mut self, path: &PathBuf) -> Result<()> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SokuError::Io(e))?;

        Logger::debug(&format!("Loading env file: {}", path.display()));

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse KEY=VALUE format
            if let Some((key, value)) = self.parse_env_line(line) {
                self.variables.insert(key, value);
            } else {
                Logger::warn(&format!(
                    "Invalid env line in {} at line {}: {}",
                    path.display(),
                    line_num + 1,
                    line
                ));
            }
        }

        Ok(())
    }

    /// Parse a single environment variable line
    fn parse_env_line(&self, line: &str) -> Option<(String, String)> {
        // Find the first = sign
        let eq_pos = line.find('=')?;

        let key = line[..eq_pos].trim();
        let value = line[eq_pos + 1..].trim();

        // Validate key (must start with letter or underscore, contain only alphanumeric + underscore)
        if !key.chars().next()?.is_alphabetic() && key.chars().next()? != '_' {
            return None;
        }

        if !key.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }

        // Remove quotes from value if present
        let value = if (value.starts_with('"') && value.ends_with('"')) ||
                       (value.starts_with('\'') && value.ends_with('\'')) {
            &value[1..value.len() - 1]
        } else {
            value
        };

        Some((key.to_string(), value.to_string()))
    }

    /// Add a custom environment variable
    #[allow(dead_code)] // Public API method
    pub fn set(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }

    /// Get an environment variable value
    #[allow(dead_code)] // Public API method
    pub fn get(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    /// Get all environment variables
    pub fn get_all(&self) -> &HashMap<String, String> {
        &self.variables
    }

    /// Replace environment variables in code
    /// Supports both process.env.VAR and import.meta.env.VAR
    pub fn replace_in_code(&self, code: &str) -> String {
        let mut result = code.to_string();

        // Replace process.env.VARIABLE
        for (key, value) in &self.variables {
            let pattern = format!("process.env.{}", key);
            let replacement = self.format_value_for_js(value);
            result = result.replace(&pattern, &replacement);
        }

        // Replace import.meta.env.VARIABLE
        for (key, value) in &self.variables {
            let pattern = format!("import.meta.env.{}", key);
            let replacement = self.format_value_for_js(value);
            result = result.replace(&pattern, &replacement);
        }

        result
    }

    /// Format a value for JavaScript code injection
    fn format_value_for_js(&self, value: &str) -> String {
        // Check if it's a boolean
        if value == "true" || value == "false" {
            return value.to_string();
        }

        // Check if it's a number
        if value.parse::<f64>().is_ok() {
            return value.to_string();
        }

        // Otherwise, wrap in quotes as a string
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }

    /// Generate TypeScript declaration for import.meta.env
    #[allow(dead_code)] // Future feature for TypeScript support
    pub fn generate_env_dts(&self) -> String {
        let mut dts = String::from("interface ImportMetaEnv {\n");

        for key in self.variables.keys() {
            dts.push_str(&format!("  readonly {}: string;\n", key));
        }

        dts.push_str("}\n\n");
        dts.push_str("interface ImportMeta {\n");
        dts.push_str("  readonly env: ImportMetaEnv;\n");
        dts.push_str("}\n");

        dts
    }
}

impl Default for EnvVarsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::io::Write;

    #[test]
    fn test_parse_env_line() {
        let manager = EnvVarsManager::new();

        assert_eq!(
            manager.parse_env_line("KEY=value"),
            Some(("KEY".to_string(), "value".to_string()))
        );

        assert_eq!(
            manager.parse_env_line("KEY=\"quoted value\""),
            Some(("KEY".to_string(), "quoted value".to_string()))
        );

        assert_eq!(
            manager.parse_env_line("KEY='single quoted'"),
            Some(("KEY".to_string(), "single quoted".to_string()))
        );

        // Invalid lines
        assert_eq!(manager.parse_env_line("INVALID"), None);
        assert_eq!(manager.parse_env_line("123KEY=value"), None);
    }

    #[test]
    fn test_replace_in_code() {
        let mut manager = EnvVarsManager::new();
        manager.set("API_URL".to_string(), "https://api.example.com".to_string());
        manager.set("DEBUG".to_string(), "true".to_string());
        manager.set("PORT".to_string(), "3000".to_string());

        let code = r#"
            const url = process.env.API_URL;
            const debug = import.meta.env.DEBUG;
            const port = process.env.PORT;
        "#;

        let result = manager.replace_in_code(code);

        assert!(result.contains(r#"const url = "https://api.example.com";"#));
        assert!(result.contains("const debug = true;"));
        assert!(result.contains("const port = 3000;"));
    }

    #[test]
    fn test_load_from_file() {
        let temp_dir = tempdir().unwrap();
        let env_file = temp_dir.path().join(".env");

        let mut file = std::fs::File::create(&env_file).unwrap();
        writeln!(file, "# Comment line").unwrap();
        writeln!(file, "API_KEY=secret123").unwrap();
        writeln!(file, "DEBUG=true").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "PORT=8080").unwrap();

        let manager = EnvVarsManager::load_from_files(temp_dir.path(), "development").unwrap();

        assert_eq!(manager.get("API_KEY"), Some(&"secret123".to_string()));
        assert_eq!(manager.get("DEBUG"), Some(&"true".to_string()));
        assert_eq!(manager.get("PORT"), Some(&"8080".to_string()));
        assert_eq!(manager.get("NODE_ENV"), Some(&"development".to_string()));
        assert_eq!(manager.get("DEV"), Some(&"true".to_string()));
    }

    #[test]
    fn test_format_value_for_js() {
        let manager = EnvVarsManager::new();

        assert_eq!(manager.format_value_for_js("true"), "true");
        assert_eq!(manager.format_value_for_js("false"), "false");
        assert_eq!(manager.format_value_for_js("123"), "123");
        assert_eq!(manager.format_value_for_js("3.14"), "3.14");
        assert_eq!(manager.format_value_for_js("hello"), r#""hello""#);
        assert_eq!(manager.format_value_for_js("hello \"world\""), r#""hello \"world\"""#);
    }
}
