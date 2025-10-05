use crate::core::{interfaces::FileSystemService, models::*};
use crate::utils::{Result, UltraError};
use std::path::Path;
use tokio::fs;

pub struct TokioFileSystemService;

#[async_trait::async_trait]
impl FileSystemService for TokioFileSystemService {
    async fn scan_directory(&self, path: &Path) -> Result<ProjectStructure> {
        let mut structure = ProjectStructure::default();
        let mut entries = fs::read_dir(path).await
            .map_err(UltraError::Io)?;

        while let Some(entry) = entries.next_entry().await
            .map_err(UltraError::Io)? {

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let extension = path.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            match extension.as_str() {
                "js" | "jsx" | "ts" | "tsx" => {
                    structure.js_modules.push(path);
                }
                "css" | "scss" | "sass" => {
                    structure.css_files.push(path);
                }
                "html" | "htm" => {
                    structure.html_files.push(path);
                }
                _ => {
                    structure.other_files.push(path);
                }
            }
        }

        Ok(structure)
    }

    async fn read_file(&self, path: &Path) -> Result<String> {
        fs::read_to_string(path).await
            .map_err(UltraError::Io)
    }

    async fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            self.create_directory(parent).await?;
        }

        fs::write(path, content).await
            .map_err(UltraError::Io)
    }

    async fn create_directory(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path).await
            .map_err(UltraError::Io)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio;

    #[tokio::test]
    async fn test_file_operations() {
        let fs_service = TokioFileSystemService;
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        // Test write and read
        let content = "Hello, Ultra!";
        fs_service.write_file(&test_file, content).await.unwrap();

        let read_content = fs_service.read_file(&test_file).await.unwrap();
        assert_eq!(content, read_content);

        // Test file exists (use std::path instead)
        assert!(test_file.exists());
    }

    #[tokio::test]
    async fn test_directory_scan() {
        let fs_service = TokioFileSystemService;
        let temp_dir = tempdir().unwrap();

        // Create test files
        let js_file = temp_dir.path().join("test.js");
        let css_file = temp_dir.path().join("test.css");
        let html_file = temp_dir.path().join("test.html");

        fs_service.write_file(&js_file, "console.log('test');").await.unwrap();
        fs_service.write_file(&css_file, "body { color: red; }").await.unwrap();
        fs_service.write_file(&html_file, "<html></html>").await.unwrap();

        // Scan directory
        let structure = fs_service.scan_directory(temp_dir.path()).await.unwrap();

        assert_eq!(structure.js_modules.len(), 1);
        assert_eq!(structure.css_files.len(), 1);
        assert_eq!(structure.html_files.len(), 1);
    }
}