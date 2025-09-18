use crate::config::Config;
use crate::graph::ModuleGraph;
use crate::transform_simple::SimpleTransformer;
use crate::css::{LightningCssProcessor, CssOptions};
use anyhow::Result;
use axum::{
    extract::{Path as AxumPath, Query, State, WebSocketUpgrade},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, get_service},
    Router,
};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{sink::SinkExt, stream::StreamExt};
use notify::{Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

#[derive(Clone)]
pub struct DevServer {
    graph: Arc<ModuleGraph>,
    config: Arc<Config>,
    port: u16,
    hmr_tx: Arc<broadcast::Sender<HmrMessage>>,
    file_events: Arc<RwLock<VecDeque<FileEvent>>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum HmrMessage {
    #[serde(rename = "connected")]
    Connected,
    #[serde(rename = "update")]
    Update { updates: Vec<HmrUpdate> },
    #[serde(rename = "full-reload")]
    FullReload { reason: String },
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct HmrUpdate {
    pub path: String,
    pub update_type: UpdateType,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateType {
    JsUpdate,
    CssUpdate,
    AssetUpdate,
}

#[derive(Debug, Clone)]
struct FileEvent {
    path: PathBuf,
    kind: EventKind,
    timestamp: Instant,
}

#[derive(Debug, Deserialize)]
struct ModuleQuery {
    import: Option<String>,
    t: Option<String>, // timestamp for cache busting
}

impl DevServer {
    pub fn new(graph: ModuleGraph, config: Config, port: u16) -> Self {
        let (hmr_tx, _) = broadcast::channel(1000);

        Self {
            graph: Arc::new(graph),
            config: Arc::new(config),
            port,
            hmr_tx: Arc::new(hmr_tx),
            file_events: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    pub async fn run(self) -> Result<()> {
        // Start file watcher
        let watcher_task = self.start_file_watcher().await?;

        // Start event processor
        let event_processor_task = self.start_event_processor();

        // Create HTTP router
        let app = self.create_router();

        // Start server
        let listener = tokio::net::TcpListener::bind(format!("{}:{}", self.config.server.host, self.port)).await?;

        println!("🚀 Ultra bundler dev server running on http://{}:{}", self.config.server.host, self.port);

        // Run server and background tasks
        tokio::select! {
            result = axum::serve(listener, app) => {
                result?;
            }
            _ = watcher_task => {
                println!("File watcher stopped");
            }
            _ = event_processor_task => {
                println!("Event processor stopped");
            }
        }

        Ok(())
    }

    fn create_router(self) -> Router {
        let serve_dir = get_service(ServeDir::new(&self.config.root))
            .handle_error(|error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            });

        let cors = if self.config.server.cors {
            CorsLayer::permissive()
        } else {
            CorsLayer::new()
        };

        Router::new()
            .route("/__ultra/hmr", get(Self::handle_hmr))
            .route("/__ultra/modules/*path", get(Self::handle_module))
            .route("/*path", get(Self::handle_static))
            .fallback_service(serve_dir)
            .layer(ServiceBuilder::new().layer(cors))
            .with_state(self)
    }

    async fn handle_hmr(
        ws: WebSocketUpgrade,
        State(server): State<DevServer>,
    ) -> impl IntoResponse {
        ws.on_upgrade(|socket| Self::handle_hmr_websocket(socket, server))
    }

    async fn handle_hmr_websocket(socket: WebSocket, server: DevServer) {
        let (mut sender, mut receiver) = socket.split();
        let mut hmr_rx = server.hmr_tx.subscribe();

        // Send initial connection message
        let _ = sender.send(Message::Text(
            serde_json::to_string(&HmrMessage::Connected).unwrap()
        )).await;

        // Handle incoming messages and HMR updates
        loop {
            tokio::select! {
                msg = receiver.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            // Handle client messages (like pings)
                            if text == "ping" {
                                let _ = sender.send(Message::Text("pong".to_string())).await;
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            break;
                        }
                        _ => {}
                    }
                }
                update = hmr_rx.recv() => {
                    match update {
                        Ok(hmr_msg) => {
                            let json = serde_json::to_string(&hmr_msg).unwrap();
                            if sender.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    }

    async fn handle_module(
        AxumPath(path): AxumPath<String>,
        Query(query): Query<ModuleQuery>,
        State(server): State<DevServer>,
    ) -> impl IntoResponse {
        let module_path = server.config.root.join(&path);

        match server.transform_module(&module_path).await {
            Ok(content) => {
                let content_type = match module_path.extension().and_then(|ext| ext.to_str()) {
                    Some("js") | Some("mjs") | Some("ts") | Some("tsx") | Some("jsx") => "application/javascript",
                    Some("css") => "text/css",
                    Some("json") => "application/json",
                    _ => "text/plain",
                };

                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, content_type)
                    .header(header::CACHE_CONTROL, "no-cache")
                    .body(content)
                    .unwrap()
            }
            Err(err) => {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(format!("Error transforming module: {}", err))
                    .unwrap()
            }
        }
    }

    async fn handle_static(
        AxumPath(path): AxumPath<String>,
        State(server): State<DevServer>,
    ) -> impl IntoResponse {
        // Serve index.html for SPA routes
        if !path.contains('.') {
            let index_path = server.config.root.join("index.html");
            if index_path.exists() {
                match tokio::fs::read_to_string(&index_path).await {
                    Ok(content) => {
                        let injected = server.inject_hmr_client(&content);
                        return Html(injected).into_response();
                    }
                    Err(_) => {}
                }
            }
        }

        StatusCode::NOT_FOUND.into_response()
    }

    async fn transform_module(&self, path: &std::path::Path) -> Result<String> {
        // Add to graph if not already present
        self.graph.add_entry_point(path)?;

        let content = tokio::fs::read_to_string(path).await?;
        let module_type = self.detect_module_type(path);

        match module_type {
            crate::cache::ModuleType::JavaScript
            | crate::cache::ModuleType::TypeScript
            | crate::cache::ModuleType::Jsx
            | crate::cache::ModuleType::Tsx => {
                // Use simple transformer for now
                let transformer = SimpleTransformer::new();
                let result = transformer.transform(&content, path)?;

                // Inject HMR runtime for JS modules
                let hmr_runtime = format!(
                    r#"
// HMR Runtime
if (import.meta.hot) {{
  import.meta.hot.accept((newModule) => {{
    // Handle hot reload
    console.log('🔥 Module updated: {}');
  }});
}}

"#,
                    path.display()
                );

                Ok(format!("{}\n{}", hmr_runtime, result.code))
            }
            crate::cache::ModuleType::Css => {
                // Use Lightning CSS for ultra-fast processing
                self.process_css_with_lightning(&content, path).await
            }
            crate::cache::ModuleType::Json => {
                // JSON files are exported as ES modules
                Ok(format!("export default {};", content))
            }
            _ => {
                // For other assets, return the content as-is
                Ok(content)
            }
        }
    }

    async fn process_css_with_lightning(&self, content: &str, path: &std::path::Path) -> Result<String> {
        // Create Lightning CSS processor with dev-optimized options
        let css_options = CssOptions {
            minify: false, // Don't minify in development
            modules: path.to_string_lossy().contains(".module.css"),
            autoprefixer: true,
            nesting: true,
            custom_properties: true,
            ..CssOptions::default()
        };

        let processor = LightningCssProcessor::new(css_options);
        let result = processor.transform(content, path)?;

        // Convert to JS module for HMR
        let css_js_module = if result.exports.is_empty() {
            // Regular CSS file
            format!(
                r#"// CSS Module: {}
const css = `{}`;

// Inject CSS into head
const styleElement = document.createElement('style');
styleElement.textContent = css;
styleElement.setAttribute('data-ultra-css', '{}');
document.head.appendChild(styleElement);

// HMR Support
if (import.meta.hot) {{
  import.meta.hot.accept(() => {{
    // Update existing style element
    const existing = document.querySelector('[data-ultra-css="{}"]');
    if (existing) {{
      existing.textContent = css;
    }}
  }});
}}

export default css;
"#,
                path.display(),
                result.code.replace('`', r"\`").replace('\\', r"\\"),
                path.display(),
                path.display()
            )
        } else {
            // CSS Modules
            let exports_js = result.exports
                .iter()
                .map(|(key, value)| format!("  {}: '{}'", key, value))
                .collect::<Vec<_>>()
                .join(",\n");

            format!(
                r#"// CSS Module: {}
const css = `{}`;

// Inject CSS into head
const styleElement = document.createElement('style');
styleElement.textContent = css;
styleElement.setAttribute('data-ultra-css-module', '{}');
document.head.appendChild(styleElement);

// CSS Modules exports
const cssModules = {{
{}
}};

// HMR Support
if (import.meta.hot) {{
  import.meta.hot.accept(() => {{
    const existing = document.querySelector('[data-ultra-css-module="{}"]');
    if (existing) {{
      existing.textContent = css;
    }}
  }});
}}

export default cssModules;
export {{ cssModules as classes }};
"#,
                path.display(),
                result.code.replace('`', r"\`").replace('\\', r"\\"),
                path.display(),
                exports_js,
                path.display()
            )
        };

        Ok(css_js_module)
    }

    fn detect_module_type(&self, path: &std::path::Path) -> crate::cache::ModuleType {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("ts") => crate::cache::ModuleType::TypeScript,
            Some("tsx") => crate::cache::ModuleType::Tsx,
            Some("jsx") => crate::cache::ModuleType::Jsx,
            Some("css") => crate::cache::ModuleType::Css,
            Some("json") => crate::cache::ModuleType::Json,
            Some("js") | Some("mjs") => crate::cache::ModuleType::JavaScript,
            _ => crate::cache::ModuleType::Asset,
        }
    }

    fn inject_hmr_client(&self, html: &str) -> String {
        let hmr_script = format!(
            r#"
<script type="module">
  const socket = new WebSocket('ws://{}:{}/__ultra/hmr');

  socket.addEventListener('message', (event) => {{
    const data = JSON.parse(event.data);

    if (data.type === 'update') {{
      for (const update of data.updates) {{
        if (update.update_type === 'css-update') {{
          updateCss(update.path);
        }} else if (update.update_type === 'js-update') {{
          // For now, just reload the page
          // In a real implementation, we'd do hot module replacement
          window.location.reload();
        }}
      }}
    }} else if (data.type === 'full-reload') {{
      window.location.reload();
    }} else if (data.type === 'error') {{
      console.error('HMR Error:', data.message);
    }}
  }});

  function updateCss(path) {{
    const links = document.querySelectorAll(`link[href*="${{path}}"]`);
    links.forEach(link => {{
      const newLink = link.cloneNode();
      newLink.href = link.href.split('?')[0] + '?t=' + Date.now();
      link.parentNode.insertBefore(newLink, link.nextSibling);
      link.remove();
    }});
  }}

  // Send ping every 30 seconds to keep connection alive
  setInterval(() => {{
    if (socket.readyState === WebSocket.OPEN) {{
      socket.send('ping');
    }}
  }}, 30000);
</script>
"#,
            self.config.server.host, self.port
        );

        if let Some(head_end) = html.find("</head>") {
            let mut result = html.to_string();
            result.insert_str(head_end, &hmr_script);
            result
        } else {
            format!("{}{}", hmr_script, html)
        }
    }

    async fn start_file_watcher(self) -> Result<tokio::task::JoinHandle<()>> {
        let events = self.file_events.clone();
        let config = self.config.clone();

        let handle = tokio::task::spawn_blocking(move || {
            let mut watcher = RecommendedWatcher::new(
                move |res: Result<Event, notify::Error>| {
                    if let Ok(event) = res {
                        let events = events.clone();
                        tokio::spawn(async move {
                            let mut queue = events.write().await;
                            for path in event.paths {
                                // Filter out non-source files
                                if Self::should_watch_file(&path) {
                                    queue.push_back(FileEvent {
                                        path,
                                        kind: event.kind,
                                        timestamp: Instant::now(),
                                    });
                                }
                            }
                        });
                    }
                },
                NotifyConfig::default(),
            ).unwrap();

            watcher.watch(&config.root, RecursiveMode::Recursive).unwrap();

            // Keep the watcher alive
            loop {
                std::thread::sleep(Duration::from_secs(1));
            }
        });

        Ok(handle)
    }

    fn start_event_processor(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(16)); // ~60fps

            loop {
                interval.tick().await;

                let events = {
                    let mut queue = self.file_events.write().await;
                    let mut batch = Vec::new();

                    // Collect events from the last 16ms (coalescing)
                    let cutoff = Instant::now() - Duration::from_millis(16);
                    while let Some(event) = queue.front() {
                        if event.timestamp < cutoff {
                            batch.push(queue.pop_front().unwrap());
                        } else {
                            break;
                        }
                    }

                    batch
                };

                if !events.is_empty() {
                    if let Err(err) = self.process_file_events(events).await {
                        eprintln!("Error processing file events: {}", err);
                    }
                }
            }
        })
    }

    async fn process_file_events(&self, events: Vec<FileEvent>) -> Result<()> {
        let mut affected_modules = Vec::new();

        // Group events by path and take the latest
        let mut latest_events: HashMap<PathBuf, &FileEvent> = HashMap::new();
        for event in &events {
            latest_events.insert(event.path.clone(), event);
        }

        for (path, _event) in latest_events {
            match self.graph.invalidate_module(&path) {
                Ok(affected) => {
                    affected_modules.extend(affected);
                }
                Err(err) => {
                    let _ = self.hmr_tx.send(HmrMessage::Error {
                        message: format!("Failed to invalidate module {}: {}", path.display(), err),
                    });
                }
            }
        }

        if !affected_modules.is_empty() {
            let updates = affected_modules.into_iter().map(|module_id| {
                let update_type = match PathBuf::from(&module_id).extension().and_then(|ext| ext.to_str()) {
                    Some("css") => UpdateType::CssUpdate,
                    Some("js") | Some("ts") | Some("jsx") | Some("tsx") | Some("mjs") => UpdateType::JsUpdate,
                    _ => UpdateType::AssetUpdate,
                };

                HmrUpdate {
                    path: module_id,
                    update_type,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                }
            }).collect();

            let _ = self.hmr_tx.send(HmrMessage::Update { updates });
        }

        Ok(())
    }

    fn should_watch_file(path: &std::path::Path) -> bool {
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            matches!(extension, "js" | "ts" | "jsx" | "tsx" | "css" | "json" | "html" | "vue" | "svelte")
        } else {
            false
        }
    }
}

pub async fn preview_server(dir: String, port: u16) -> Result<()> {
    let serve_dir = get_service(ServeDir::new(&dir))
        .handle_error(|error| async move {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unhandled internal error: {}", error),
            )
        });

    let app = Router::new()
        .fallback_service(serve_dir);

    let listener = tokio::net::TcpListener::bind(format!("localhost:{}", port)).await?;

    println!("📦 Preview server running on http://localhost:{}", port);

    axum::serve(listener, app).await?;

    Ok(())
}