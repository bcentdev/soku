use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::utils::Logger;

/// Path alias resolver for import path resolution
pub struct PathAliasResolver {
    aliases: HashMap<String, String>,
    root: PathBuf,
}

impl PathAliasResolver {
    /// Create a new path alias resolver
    pub fn new(aliases: HashMap<String, String>, root: PathBuf) -> Self {
        Logger::debug(&format!("🔗 Initialized PathAliasResolver with {} aliases", aliases.len()));
        for (alias, target) in &aliases {
            Logger::debug(&format!("  {} → {}", alias, target));
        }

        Self {
            aliases,
            root,
        }
    }

    /// Resolve an import path using aliases
    /// Returns the resolved path if an alias matches, or None if no alias found
    pub fn resolve(&self, import_path: &str) -> Option<PathBuf> {
        // Try exact match first (e.g., "@" or "~")
        if let Some(target) = self.aliases.get(import_path) {
            return Some(self.resolve_target(target));
        }

        // Try pattern matches (e.g., "@/components/Button")
        for (alias, target) in &self.aliases {
            // Check if import starts with alias + "/"
            let alias_prefix = format!("{}/", alias);
            if import_path.starts_with(&alias_prefix) {
                // Get the rest of the path after the alias
                let rest = &import_path[alias_prefix.len()..];

                // Resolve the target and append the rest
                let resolved_target = self.resolve_target(target);
                let final_path = resolved_target.join(rest);

                Logger::debug(&format!("🔗 Resolved alias: {} → {}", import_path, final_path.display()));
                return Some(final_path);
            }

            // Also check for exact alias match without trailing slash
            if import_path == alias {
                let resolved = self.resolve_target(target);
                Logger::debug(&format!("🔗 Resolved alias: {} → {}", import_path, resolved.display()));
                return Some(resolved);
            }
        }

        None
    }

    /// Resolve a target path relative to root
    fn resolve_target(&self, target: &str) -> PathBuf {
        let path = Path::new(target);

        if path.is_absolute() {
            path.to_path_buf()
        } else {
            // Remove leading "./" if present
            let target = if target.starts_with("./") {
                &target[2..]
            } else {
                target
            };

            self.root.join(target)
        }
    }

    /// Check if an import path matches any alias
    #[allow(dead_code)] // Public API method
    pub fn matches_alias(&self, import_path: &str) -> bool {
        // Check exact match
        if self.aliases.contains_key(import_path) {
            return true;
        }

        // Check pattern matches
        for alias in self.aliases.keys() {
            let alias_prefix = format!("{}/", alias);
            if import_path.starts_with(&alias_prefix) || import_path == alias {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_alias_match() {
        let mut aliases = HashMap::new();
        aliases.insert("@".to_string(), "./src".to_string());

        let resolver = PathAliasResolver::new(aliases, PathBuf::from("/project"));
        let resolved = resolver.resolve("@");

        assert_eq!(resolved, Some(PathBuf::from("/project/src")));
    }

    #[test]
    fn test_pattern_alias_match() {
        let mut aliases = HashMap::new();
        aliases.insert("@".to_string(), "./src".to_string());

        let resolver = PathAliasResolver::new(aliases, PathBuf::from("/project"));
        let resolved = resolver.resolve("@/components/Button.js");

        assert_eq!(resolved, Some(PathBuf::from("/project/src/components/Button.js")));
    }

    #[test]
    fn test_multiple_aliases() {
        let mut aliases = HashMap::new();
        aliases.insert("@".to_string(), "./src".to_string());
        aliases.insert("~".to_string(), ".".to_string());
        aliases.insert("#components".to_string(), "./src/components".to_string());

        let resolver = PathAliasResolver::new(aliases, PathBuf::from("/project"));

        assert_eq!(
            resolver.resolve("@/utils/helpers.js"),
            Some(PathBuf::from("/project/src/utils/helpers.js"))
        );

        assert_eq!(
            resolver.resolve("~/main.js"),
            Some(PathBuf::from("/project/main.js"))
        );

        assert_eq!(
            resolver.resolve("#components/Button.js"),
            Some(PathBuf::from("/project/src/components/Button.js"))
        );
    }

    #[test]
    fn test_no_alias_match() {
        let mut aliases = HashMap::new();
        aliases.insert("@".to_string(), "./src".to_string());

        let resolver = PathAliasResolver::new(aliases, PathBuf::from("/project"));
        let resolved = resolver.resolve("./relative/path.js");

        assert_eq!(resolved, None);
    }

    #[test]
    fn test_matches_alias() {
        let mut aliases = HashMap::new();
        aliases.insert("@".to_string(), "./src".to_string());

        let resolver = PathAliasResolver::new(aliases, PathBuf::from("/project"));

        assert!(resolver.matches_alias("@"));
        assert!(resolver.matches_alias("@/components/Button.js"));
        assert!(!resolver.matches_alias("./relative/path.js"));
        assert!(!resolver.matches_alias("node_modules/package"));
    }

    #[test]
    fn test_absolute_target_path() {
        let mut aliases = HashMap::new();
        aliases.insert("@shared".to_string(), "/absolute/shared".to_string());

        let resolver = PathAliasResolver::new(aliases, PathBuf::from("/project"));
        let resolved = resolver.resolve("@shared/utils.js");

        assert_eq!(resolved, Some(PathBuf::from("/absolute/shared/utils.js")));
    }
}
