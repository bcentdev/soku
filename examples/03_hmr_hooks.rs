// Example: Advanced HMR Hooks Usage
//
// This example shows how to use HMR hooks to customize
// Hot Module Replacement behavior.

use async_trait::async_trait;
use soku::infrastructure::{BuiltInHmrHooks, HmrHook, HmrHookContext, SokuHmrService};
use soku::utils::Result;
use std::path::PathBuf;
use std::sync::Arc;

/// Custom HMR hook that logs detailed update information
struct DetailedLoggingHook {
    name: String,
}

impl DetailedLoggingHook {
    pub fn new() -> Self {
        Self {
            name: "detailed-logger".to_string(),
        }
    }
}

#[async_trait]
impl HmrHook for DetailedLoggingHook {
    fn name(&self) -> &str {
        &self.name
    }

    async fn before_update(&self, context: &HmrHookContext) -> Result<()> {
        println!(
            "🔥 [{}] Preparing to update: {}",
            self.name,
            context.file_path.display()
        );
        println!("   - Update type: {:?}", context.update_kind);
        println!("   - Timestamp: {}", context.timestamp);
        println!("   - Connected clients: {}", context.client_count);
        Ok(())
    }

    async fn after_update(&self, context: &HmrHookContext) -> Result<()> {
        println!(
            "✅ [{}] Update sent to {} clients",
            self.name, context.client_count
        );
        Ok(())
    }

    async fn on_client_connect(&self, client_id: &str) -> Result<()> {
        println!("🔌 [{}] New client connected: {}", self.name, client_id);
        Ok(())
    }

    async fn on_client_disconnect(&self, client_id: &str) -> Result<()> {
        println!("🔌 [{}] Client disconnected: {}", self.name, client_id);
        Ok(())
    }

    async fn on_update_error(&self, context: &HmrHookContext, error: &str) -> Result<()> {
        eprintln!(
            "❌ [{}] Update failed for {}: {}",
            self.name,
            context.file_path.display(),
            error
        );
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let project_root = PathBuf::from("./demo-project");

    // Create HMR service with multiple hooks
    let hmr_service = SokuHmrService::new(project_root.clone())
        // Built-in logger hook
        .with_hook(Arc::new(BuiltInHmrHooks::logger().with_verbose(true)))
        .await
        // Full reload for config files
        .with_hook(Arc::new(BuiltInHmrHooks::full_reload_on_pattern(
            "config".to_string(),
        )))
        .await
        // Throttle updates (min 100ms between updates)
        .with_hook(Arc::new(BuiltInHmrHooks::throttle(100)))
        .await
        // Custom notification hook
        .with_hook(Arc::new(BuiltInHmrHooks::notification()))
        .await
        // Custom detailed logging hook
        .with_hook(Arc::new(DetailedLoggingHook::new()))
        .await
        // Transform hook to add timestamp to updates
        .with_hook(Arc::new(BuiltInHmrHooks::transform(
            "add-timestamp".to_string(),
            |content| {
                let timestamp = chrono::Utc::now().to_rfc3339();
                Ok(format!("// Updated at: {}\n{}", timestamp, content))
            },
        )))
        .await;

    println!("🚀 Starting Soku HMR Server with custom hooks...");
    println!("   Hooks registered:");
    println!("   - Logger (verbose mode)");
    println!("   - Full reload for config files");
    println!("   - Throttle (100ms min interval)");
    println!("   - Desktop notifications");
    println!("   - Detailed logging");
    println!("   - Timestamp transformer");

    // Start HMR server
    let server_handle = tokio::spawn(async move {
        if let Err(e) = hmr_service.start_server(3001).await {
            eprintln!("HMR server error: {}", e);
        }
    });

    // Start file watching
    println!("\n👀 Watching for file changes...");
    println!("   Connect your browser to: ws://localhost:3001");
    println!("   Press Ctrl+C to stop\n");

    // Wait for server
    server_handle.await.unwrap();

    Ok(())
}
