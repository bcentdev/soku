use crate::utils::{Result, UltraError};
use crate::infrastructure::HmrHookManager;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::broadcast;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use dashmap::DashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HmrUpdate {
    pub id: String,
    pub kind: HmrUpdateKind,
    pub path: PathBuf,
    pub content: Option<String>,
    pub dependencies: Vec<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HmrUpdateKind {
    FileChanged,
    FileAdded,
    FileRemoved,
    ModuleUpdated,
    CssUpdated,
    FullReload,
    BuildError,
    BuildSuccess,
}

#[derive(Debug, Clone)]
pub struct HmrClient {
    #[allow(dead_code)] // Used for logging and debugging
    pub id: String,
    pub sender: tokio::sync::mpsc::UnboundedSender<String>,
}

/// Ultra-fast Hot Module Replacement system
#[derive(Clone)]
pub struct UltraHmrService {
    clients: Arc<DashMap<String, HmrClient>>,
    update_sender: broadcast::Sender<HmrUpdate>,
    root_path: PathBuf,
    hook_manager: Arc<tokio::sync::Mutex<HmrHookManager>>,
}

impl UltraHmrService {
    pub fn new(root_path: PathBuf) -> Self {
        let (update_sender, _) = broadcast::channel(1000);

        Self {
            clients: Arc::new(DashMap::new()),
            update_sender,
            root_path,
            hook_manager: Arc::new(tokio::sync::Mutex::new(HmrHookManager::new())),
        }
    }

    pub async fn with_hook(self, hook: Arc<dyn crate::infrastructure::HmrHook>) -> Self {
        self.hook_manager.lock().await.register(hook);
        self
    }

    /// Start HMR server with WebSocket support
    pub async fn start_server(&self, port: u16) -> Result<()> {
        let addr = format!("127.0.0.1:{}", port);
        let listener = tokio::net::TcpListener::bind(&addr).await
            .map_err(|e| UltraError::build(format!("HMR server bind failed: {}", e)))?;

        tracing::info!("🔥 HMR server started on ws://{}", addr);

        let clients = self.clients.clone();
        let _update_sender = self.update_sender.clone();
        let mut update_receiver = self.update_sender.subscribe();

        // Spawn update broadcaster
        let broadcaster_clients = clients.clone();
        tokio::spawn(async move {
            while let Ok(update) = update_receiver.recv().await {
                let message = match serde_json::to_string(&update) {
                    Ok(msg) => msg,
                    Err(e) => {
                        tracing::warn!("Failed to serialize HMR update: {}", e);
                        continue;
                    }
                };

                let mut clients_to_remove = Vec::new();

                // Send to all connected clients
                for entry in broadcaster_clients.iter() {
                    let client_id = entry.key().clone();
                    let client = entry.value();

                    if let Err(_) = client.sender.send(message.clone()) {
                        // Client channel closed, mark for removal
                        clients_to_remove.push(client_id);
                    }
                }

                // Remove disconnected clients
                for client_id in clients_to_remove {
                    broadcaster_clients.remove(&client_id);
                    tracing::info!("🔌 Removed disconnected HMR client: {}", client_id);
                }
            }
        });

        // Handle WebSocket connections
        let hook_manager = self.hook_manager.clone();
        while let Ok((stream, addr)) = listener.accept().await {
            let clients = clients.clone();
            let hook_manager = hook_manager.clone();

            tokio::spawn(async move {
                if let Err(e) = Self::handle_client(stream, clients, hook_manager).await {
                    crate::utils::Logger::warn(&format!("HMR client error {}: {}", addr, e));
                }
            });
        }

        Ok(())
    }

    async fn handle_client(
        stream: tokio::net::TcpStream,
        clients: Arc<DashMap<String, HmrClient>>,
        hook_manager: Arc<tokio::sync::Mutex<HmrHookManager>>,
    ) -> Result<()> {
        let ws_stream = accept_async(stream).await
            .map_err(|e| UltraError::build(format!("WebSocket handshake failed: {}", e)))?;

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let client_id = Uuid::new_v4().to_string();

        // Create channel for this client
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        // Register client with sender channel
        let client = HmrClient {
            id: client_id.clone(),
            sender: tx,
        };
        clients.insert(client_id.clone(), client);

        tracing::info!("🔌 HMR client connected: {}", client_id);

        // 🪝 HMR HOOK: Client connect
        let _ = hook_manager.lock().await.trigger_client_connect(&client_id).await;

        // Send initial connection message
        let welcome = HmrUpdate {
            id: Uuid::new_v4().to_string(),
            kind: HmrUpdateKind::ModuleUpdated,
            path: PathBuf::from("__hmr_connected__"),
            content: Some("Connected to Ultra HMR".to_string()),
            dependencies: vec![],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        if let Ok(welcome_msg) = serde_json::to_string(&welcome) {
            let _ = ws_sender.send(Message::Text(welcome_msg)).await;
        }

        // Spawn task to forward messages from channel to websocket
        let client_id_clone = client_id.clone();
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = ws_sender.send(Message::Text(message)).await {
                    tracing::warn!("Failed to send to HMR client {}: {}", client_id_clone, e);
                    break;
                }
            }
        });

        // Handle incoming client messages (ping/pong)
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(_)) => {
                    // Ping/pong handled automatically by tungstenite
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }

        // Remove client on disconnect
        clients.remove(&client_id);
        tracing::info!("🔌 HMR client disconnected: {}", client_id);

        // 🪝 HMR HOOK: Client disconnect
        let _ = hook_manager.lock().await.trigger_client_disconnect(&client_id).await;

        Ok(())
    }

    /// Start file watching for HMR
    pub async fn start_watching(&self) -> Result<()> {
        let update_sender = self.update_sender.clone();
        let root_path = self.root_path.clone();

        let (tx, mut rx) = tokio::sync::mpsc::channel(1000);

        // Setup file watcher
        let mut watcher: RecommendedWatcher = Watcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    let _ = tx.try_send(event);
                }
            },
            Config::default(),
        ).map_err(|e| UltraError::build(format!("File watcher setup failed: {}", e)))?;

        // Watch the root directory
        watcher.watch(&root_path, RecursiveMode::Recursive)
            .map_err(|e| UltraError::build(format!("Watch setup failed: {}", e)))?;

        tracing::info!("👁️  File watcher started for: {}", root_path.display());

        // Process file events
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Err(e) = Self::process_file_event(event, &update_sender, &root_path).await {
                    tracing::warn!("HMR event processing error: {}", e);
                }
            }
        });

        // Keep watcher alive
        tokio::spawn(async move {
            let _watcher = watcher;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        Ok(())
    }

    async fn process_file_event(
        event: Event,
        update_sender: &broadcast::Sender<HmrUpdate>,
        root_path: &Path,
    ) -> Result<()> {
        for path in event.paths {
            // Skip non-source files
            if !Self::is_source_file(&path) {
                continue;
            }

            let relative_path = path.strip_prefix(root_path)
                .unwrap_or(&path)
                .to_path_buf();

            let update_kind = match event.kind {
                EventKind::Create(_) => HmrUpdateKind::FileAdded,
                EventKind::Remove(_) => HmrUpdateKind::FileRemoved,
                EventKind::Modify(_) => {
                    if Self::is_js_file(&path) {
                        HmrUpdateKind::ModuleUpdated
                    } else if Self::is_css_file(&path) {
                        HmrUpdateKind::CssUpdated
                    } else {
                        HmrUpdateKind::FileChanged
                    }
                }
                _ => continue,
            };

            // Read file content for updates
            let content = if matches!(update_kind, HmrUpdateKind::FileRemoved) {
                None
            } else {
                tokio::fs::read_to_string(&path).await.ok()
            };

            let update = HmrUpdate {
                id: Uuid::new_v4().to_string(),
                kind: update_kind,
                path: relative_path,
                content,
                dependencies: vec![], // TODO: Extract dependencies
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            };

            // Send update to all clients
            if let Err(e) = update_sender.send(update.clone()) {
                tracing::warn!("Failed to broadcast HMR update: {}", e);
            } else {
                tracing::info!(
                    "🔥 HMR: {:?} - {}",
                    update.kind,
                    update.path.display()
                );
            }
        }

        Ok(())
    }

    fn is_source_file(path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            matches!(ext, "js" | "ts" | "tsx" | "jsx" | "css" | "scss" | "less" | "html" | "vue")
        } else {
            false
        }
    }

    fn is_js_file(path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            matches!(ext, "js" | "ts" | "tsx" | "jsx")
        } else {
            false
        }
    }

    fn is_css_file(path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            matches!(ext, "css" | "scss" | "less")
        } else {
            false
        }
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_hmr_service_creation() {
        let temp_dir = tempdir().unwrap();
        let _hmr = UltraHmrService::new(temp_dir.path().to_path_buf());

        // TODO: Add stats() method to UltraHmrService
        // let stats = hmr.stats();
        // assert_eq!(stats.connected_clients, 0);
    }

    #[test]
    fn test_file_type_detection() {
        assert!(UltraHmrService::is_js_file(&PathBuf::from("test.js")));
        assert!(UltraHmrService::is_js_file(&PathBuf::from("test.ts")));
        assert!(UltraHmrService::is_css_file(&PathBuf::from("test.css")));
        assert!(!UltraHmrService::is_js_file(&PathBuf::from("test.txt")));
    }
}