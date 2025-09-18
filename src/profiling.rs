// Performance profiling and monitoring for ultra-bundler
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};

/// Performance profiler for tracking bundler operations
pub struct UltraProfiler {
    sessions: RwLock<HashMap<String, ProfilingSession>>,
    metrics: RwLock<PerformanceMetrics>,
    config: ProfilerConfig,
    event_tx: broadcast::Sender<ProfilingEvent>,
}

#[derive(Debug, Clone)]
pub struct ProfilerConfig {
    pub enabled: bool,
    pub sample_rate: f64,
    pub memory_tracking: bool,
    pub flame_graph: bool,
    pub export_format: ExportFormat,
}

#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    FlameGraph,
    ChromeTracing,
    Summary,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sample_rate: 1.0,
            memory_tracking: true,
            flame_graph: false,
            export_format: ExportFormat::Summary,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProfilingSession {
    pub id: String,
    pub start_time: Instant,
    pub operations: Vec<ProfiledOperation>,
    pub memory_snapshots: Vec<MemorySnapshot>,
    pub active_operations: HashMap<String, Instant>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfiledOperation {
    pub name: String,
    pub start_time: u64, // Relative to session start (microseconds)
    pub duration: u64,   // Microseconds
    pub memory_delta: i64, // Bytes
    pub metadata: HashMap<String, String>,
    pub children: Vec<ProfiledOperation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemorySnapshot {
    pub timestamp: u64,
    pub heap_used: usize,
    pub heap_total: usize,
    pub modules_memory: usize,
    pub cache_memory: usize,
    pub string_memory: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerformanceMetrics {
    pub hmr_latency_p50: f64,
    pub hmr_latency_p95: f64,
    pub hmr_latency_p99: f64,
    pub cold_start_time: f64,
    pub build_time: f64,
    pub modules_per_second: f64,
    pub cache_hit_rate: f64,
    pub memory_usage_peak: usize,
    pub active_modules: usize,
    pub total_operations: u64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            hmr_latency_p50: 0.0,
            hmr_latency_p95: 0.0,
            hmr_latency_p99: 0.0,
            cold_start_time: 0.0,
            build_time: 0.0,
            modules_per_second: 0.0,
            cache_hit_rate: 0.0,
            memory_usage_peak: 0,
            active_modules: 0,
            total_operations: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ProfilingEvent {
    SessionStarted { session_id: String },
    OperationStarted { session_id: String, operation: String },
    OperationCompleted { session_id: String, operation: ProfiledOperation },
    MemorySnapshot { session_id: String, snapshot: MemorySnapshot },
    SessionCompleted { session_id: String, summary: SessionSummary },
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub total_duration: u64,
    pub operations_count: usize,
    pub peak_memory: usize,
    pub avg_memory: usize,
    pub top_operations: Vec<(String, u64)>, // (name, duration)
}

impl UltraProfiler {
    pub fn new(config: ProfilerConfig) -> Self {
        let (event_tx, _) = broadcast::channel(1000);

        Self {
            sessions: RwLock::new(HashMap::new()),
            metrics: RwLock::new(PerformanceMetrics::default()),
            config,
            event_tx,
        }
    }

    pub async fn start_session(&self, session_id: String) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let session = ProfilingSession {
            id: session_id.clone(),
            start_time: Instant::now(),
            operations: Vec::new(),
            memory_snapshots: Vec::new(),
            active_operations: HashMap::new(),
        };

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session);
        }

        let _ = self.event_tx.send(ProfilingEvent::SessionStarted { session_id });

        Ok(())
    }

    pub async fn start_operation(&self, session_id: &str, operation_name: String) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if rand::random::<f64>() > self.config.sample_rate {
            return Ok(());
        }

        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.active_operations.insert(operation_name.clone(), Instant::now());

            let _ = self.event_tx.send(ProfilingEvent::OperationStarted {
                session_id: session_id.to_string(),
                operation: operation_name,
            });
        }

        Ok(())
    }

    pub async fn end_operation(
        &self,
        session_id: &str,
        operation_name: &str,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            if let Some(start_time) = session.active_operations.remove(operation_name) {
                let duration = start_time.elapsed();
                let start_offset = start_time.duration_since(session.start_time);

                let operation = ProfiledOperation {
                    name: operation_name.to_string(),
                    start_time: start_offset.as_micros() as u64,
                    duration: duration.as_micros() as u64,
                    memory_delta: 0, // Would track actual memory delta
                    metadata: metadata.unwrap_or_default(),
                    children: Vec::new(),
                };

                session.operations.push(operation.clone());

                let _ = self.event_tx.send(ProfilingEvent::OperationCompleted {
                    session_id: session_id.to_string(),
                    operation,
                });

                // Update metrics
                self.update_metrics(operation_name, duration).await;
            }
        }

        Ok(())
    }

    pub async fn snapshot_memory(&self, session_id: &str) -> Result<()> {
        if !self.config.enabled || !self.config.memory_tracking {
            return Ok(());
        }

        let snapshot = MemorySnapshot {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            heap_used: self.get_heap_usage(),
            heap_total: self.get_heap_total(),
            modules_memory: 0, // Would get from module graph
            cache_memory: 0,   // Would get from cache
            string_memory: 0,  // Would get from string interner
        };

        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.memory_snapshots.push(snapshot.clone());
            }
        }

        let _ = self.event_tx.send(ProfilingEvent::MemorySnapshot {
            session_id: session_id.to_string(),
            snapshot,
        });

        Ok(())
    }

    pub async fn end_session(&self, session_id: &str) -> Result<SessionSummary> {
        let mut sessions = self.sessions.write().await;

        if let Some(session) = sessions.remove(session_id) {
            let total_duration = session.start_time.elapsed();

            let mut top_operations = session.operations
                .iter()
                .map(|op| (op.name.clone(), op.duration))
                .collect::<Vec<_>>();
            top_operations.sort_by(|a, b| b.1.cmp(&a.1));
            top_operations.truncate(10);

            let peak_memory = session.memory_snapshots
                .iter()
                .map(|s| s.heap_used)
                .max()
                .unwrap_or(0);

            let avg_memory = if session.memory_snapshots.is_empty() {
                0
            } else {
                session.memory_snapshots
                    .iter()
                    .map(|s| s.heap_used)
                    .sum::<usize>() / session.memory_snapshots.len()
            };

            let summary = SessionSummary {
                session_id: session_id.to_string(),
                total_duration: total_duration.as_micros() as u64,
                operations_count: session.operations.len(),
                peak_memory,
                avg_memory,
                top_operations,
            };

            let _ = self.event_tx.send(ProfilingEvent::SessionCompleted {
                session_id: session_id.to_string(),
                summary: summary.clone(),
            });

            Ok(summary)
        } else {
            Err(anyhow::anyhow!("Session not found: {}", session_id))
        }
    }

    async fn update_metrics(&self, operation_name: &str, duration: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.total_operations += 1;

        match operation_name {
            "hmr_update" => {
                let latency_ms = duration.as_millis() as f64;
                // Simplified percentile calculation - would use proper histogram
                metrics.hmr_latency_p50 = (metrics.hmr_latency_p50 + latency_ms) / 2.0;
                metrics.hmr_latency_p95 = metrics.hmr_latency_p95.max(latency_ms);
                metrics.hmr_latency_p99 = metrics.hmr_latency_p99.max(latency_ms);
            }
            "cold_start" => {
                metrics.cold_start_time = duration.as_millis() as f64;
            }
            "build" => {
                metrics.build_time = duration.as_millis() as f64;
            }
            _ => {}
        }
    }

    pub async fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<ProfilingEvent> {
        self.event_tx.subscribe()
    }

    pub async fn export_session(&self, session_id: &str, format: ExportFormat) -> Result<String> {
        let sessions = self.sessions.read().await;

        if let Some(session) = sessions.get(session_id) {
            match format {
                ExportFormat::Json => {
                    serde_json::to_string_pretty(session)
                        .map_err(|e| anyhow::anyhow!("JSON export failed: {}", e))
                }
                ExportFormat::Summary => {
                    Ok(self.generate_summary_report(session))
                }
                ExportFormat::FlameGraph => {
                    Ok(self.generate_flame_graph(session))
                }
                ExportFormat::ChromeTracing => {
                    Ok(self.generate_chrome_tracing(session))
                }
            }
        } else {
            Err(anyhow::anyhow!("Session not found: {}", session_id))
        }
    }

    fn generate_summary_report(&self, session: &ProfilingSession) -> String {
        let total_duration = session.start_time.elapsed();
        let total_ops = session.operations.len();

        let mut report = format!(
            "Ultra Bundler Performance Report\n\
             ================================\n\
             Session: {}\n\
             Total Duration: {:.2}ms\n\
             Total Operations: {}\n\
             Avg Operation Time: {:.2}ms\n\n",
            session.id,
            total_duration.as_millis(),
            total_ops,
            if total_ops > 0 {
                session.operations.iter().map(|op| op.duration).sum::<u64>() as f64
                    / total_ops as f64 / 1000.0
            } else {
                0.0
            }
        );

        // Top operations
        let mut ops_by_duration = session.operations.clone();
        ops_by_duration.sort_by(|a, b| b.duration.cmp(&a.duration));

        report.push_str("Top Operations by Duration:\n");
        for (i, op) in ops_by_duration.iter().take(10).enumerate() {
            report.push_str(&format!(
                "{}. {} - {:.2}ms\n",
                i + 1,
                op.name,
                op.duration as f64 / 1000.0
            ));
        }

        // Memory usage
        if !session.memory_snapshots.is_empty() {
            let peak_memory = session.memory_snapshots
                .iter()
                .map(|s| s.heap_used)
                .max()
                .unwrap_or(0);

            report.push_str(&format!(
                "\nMemory Usage:\n\
                 Peak: {:.2}MB\n\
                 Snapshots: {}\n",
                peak_memory as f64 / 1024.0 / 1024.0,
                session.memory_snapshots.len()
            ));
        }

        report
    }

    fn generate_flame_graph(&self, session: &ProfilingSession) -> String {
        // Simplified flame graph format
        let mut lines = Vec::new();

        for op in &session.operations {
            lines.push(format!("{} {}", op.name, op.duration));
        }

        lines.join("\n")
    }

    fn generate_chrome_tracing(&self, session: &ProfilingSession) -> String {
        let mut events = Vec::new();

        for op in &session.operations {
            events.push(serde_json::json!({
                "name": op.name,
                "ph": "X",
                "ts": op.start_time,
                "dur": op.duration,
                "pid": 1,
                "tid": 1
            }));
        }

        serde_json::json!({
            "traceEvents": events,
            "displayTimeUnit": "ms"
        })
        .to_string()
    }

    fn get_heap_usage(&self) -> usize {
        // Placeholder - would use actual memory tracking
        1024 * 1024 * 100 // 100MB
    }

    fn get_heap_total(&self) -> usize {
        // Placeholder - would use actual memory tracking
        1024 * 1024 * 500 // 500MB
    }
}

/// Convenience macros for profiling
#[macro_export]
macro_rules! profile_operation {
    ($profiler:expr, $session:expr, $name:expr, $block:block) => {{
        let op_name = $name.to_string();
        $profiler.start_operation($session, op_name.clone()).await?;
        let result = $block;
        $profiler.end_operation($session, &op_name, None).await?;
        result
    }};
}

#[macro_export]
macro_rules! profile_with_metadata {
    ($profiler:expr, $session:expr, $name:expr, $metadata:expr, $block:block) => {{
        let op_name = $name.to_string();
        $profiler.start_operation($session, op_name.clone()).await?;
        let result = $block;
        $profiler.end_operation($session, &op_name, Some($metadata)).await?;
        result
    }};
}

/// Performance budget checker
pub struct PerformanceBudget {
    pub hmr_p95_ms: f64,
    pub cold_start_ms: f64,
    pub build_time_ms: f64,
    pub memory_mb: f64,
}

impl Default for PerformanceBudget {
    fn default() -> Self {
        Self {
            hmr_p95_ms: 50.0,   // Sub-50ms HMR
            cold_start_ms: 500.0, // Sub-500ms cold start
            build_time_ms: 2000.0, // Sub-2s builds
            memory_mb: 150.0,   // <150MB memory usage
        }
    }
}

impl PerformanceBudget {
    pub fn check(&self, metrics: &PerformanceMetrics) -> Vec<String> {
        let mut violations = Vec::new();

        if metrics.hmr_latency_p95 > self.hmr_p95_ms {
            violations.push(format!(
                "HMR p95 latency exceeded: {:.2}ms > {:.2}ms",
                metrics.hmr_latency_p95, self.hmr_p95_ms
            ));
        }

        if metrics.cold_start_time > self.cold_start_ms {
            violations.push(format!(
                "Cold start time exceeded: {:.2}ms > {:.2}ms",
                metrics.cold_start_time, self.cold_start_ms
            ));
        }

        if metrics.build_time > self.build_time_ms {
            violations.push(format!(
                "Build time exceeded: {:.2}ms > {:.2}ms",
                metrics.build_time, self.build_time_ms
            ));
        }

        let memory_mb = metrics.memory_usage_peak as f64 / 1024.0 / 1024.0;
        if memory_mb > self.memory_mb {
            violations.push(format!(
                "Memory usage exceeded: {:.2}MB > {:.2}MB",
                memory_mb, self.memory_mb
            ));
        }

        violations
    }
}