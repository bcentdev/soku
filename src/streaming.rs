// Streaming architecture for ultra-fast builds - key to beating Bun
use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;

/// Streaming build system that processes modules as they become available
pub struct StreamingBuilder {
    input_rx: mpsc::Receiver<BuildTask>,
    output_tx: mpsc::Sender<BuildResult>,
    worker_pool: WorkerPool,
    dependency_tracker: Arc<RwLock<DependencyTracker>>,
    config: BuildConfig,
}

#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub target: String,
    pub output_dir: PathBuf,
    pub chunk_size_limit: usize,
    pub parallel_workers: usize,
    pub streaming_threshold: usize, // Start writing chunks when this many are ready
    pub memory_limit: usize,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            target: "es2020".to_string(),
            output_dir: PathBuf::from("dist"),
            chunk_size_limit: 500 * 1024, // 500KB
            parallel_workers: num_cpus::get(),
            streaming_threshold: 5, // Write when 5 chunks are ready
            memory_limit: 512 * 1024 * 1024, // 512MB
        }
    }
}

#[derive(Debug, Clone)]
pub enum BuildTask {
    ProcessModule {
        id: String,
        path: PathBuf,
        is_entry: bool,
    },
    CreateChunk {
        modules: Vec<String>,
        chunk_id: String,
    },
    WriteChunk {
        chunk: ProcessedChunk,
    },
    Finalize,
}

#[derive(Debug, Clone)]
pub enum BuildResult {
    ModuleProcessed {
        id: String,
        code: String,
        dependencies: Vec<String>,
        size: usize,
    },
    ChunkCreated {
        chunk_id: String,
        size: usize,
        file_path: PathBuf,
    },
    BuildComplete {
        stats: BuildStats,
    },
    Error {
        task: String,
        error: String,
    },
}

#[derive(Debug, Clone)]
pub struct ProcessedChunk {
    pub id: String,
    pub code: String,
    pub source_map: Option<String>,
    pub dependencies: Vec<String>,
    pub size: usize,
    pub hash: String,
}

#[derive(Debug, Clone)]
pub struct BuildStats {
    pub total_modules: usize,
    pub total_chunks: usize,
    pub total_size: usize,
    pub build_time_ms: u64,
    pub modules_per_second: f64,
}

/// Worker pool for parallel processing
pub struct WorkerPool {
    workers: Vec<Worker>,
    task_tx: mpsc::Sender<WorkerTask>,
    result_rx: mpsc::Receiver<WorkerResult>,
}

struct Worker {
    id: usize,
    handle: JoinHandle<()>,
}

#[derive(Debug)]
enum WorkerTask {
    Transform {
        id: String,
        code: String,
        path: PathBuf,
    },
    Bundle {
        modules: Vec<ProcessedModule>,
        chunk_id: String,
    },
    Minify {
        code: String,
        chunk_id: String,
    },
    Shutdown,
}

#[derive(Debug)]
enum WorkerResult {
    Transformed {
        id: String,
        result: TransformResult,
    },
    Bundled {
        chunk_id: String,
        chunk: ProcessedChunk,
    },
    Minified {
        chunk_id: String,
        code: String,
        size: usize,
    },
    Error {
        task: String,
        error: String,
    },
}

#[derive(Debug, Clone)]
pub struct ProcessedModule {
    pub id: String,
    pub code: String,
    pub dependencies: Vec<String>,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct TransformResult {
    pub code: String,
    pub source_map: Option<String>,
    pub dependencies: Vec<String>,
}

/// Tracks module dependencies for optimal chunking
struct DependencyTracker {
    dependencies: HashMap<String, Vec<String>>,
    dependents: HashMap<String, Vec<String>>,
    processed: std::collections::HashSet<String>,
}

impl DependencyTracker {
    fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
            processed: std::collections::HashSet::new(),
        }
    }

    fn add_module(&mut self, id: String, dependencies: Vec<String>) {
        // Update dependencies
        self.dependencies.insert(id.clone(), dependencies.clone());

        // Update dependents
        for dep in dependencies {
            self.dependents
                .entry(dep)
                .or_insert_with(Vec::new)
                .push(id.clone());
        }

        self.processed.insert(id);
    }

    fn get_optimal_chunks(&self, entry_points: &[String]) -> Vec<Vec<String>> {
        let mut chunks = Vec::new();
        let mut assigned = std::collections::HashSet::new();

        // Create entry chunks
        for entry in entry_points {
            if !assigned.contains(entry) {
                let chunk = self.collect_chunk_modules(entry, &mut assigned);
                if !chunk.is_empty() {
                    chunks.push(chunk);
                }
            }
        }

        // Create shared chunks for common dependencies
        let common_deps = self.find_common_dependencies();
        for common_dep in common_deps {
            if !assigned.contains(&common_dep) {
                let chunk = self.collect_chunk_modules(&common_dep, &mut assigned);
                if !chunk.is_empty() {
                    chunks.push(chunk);
                }
            }
        }

        chunks
    }

    fn collect_chunk_modules(
        &self,
        start_module: &str,
        assigned: &mut std::collections::HashSet<String>,
    ) -> Vec<String> {
        let mut chunk = Vec::new();
        let mut queue = std::collections::VecDeque::new();

        queue.push_back(start_module.to_string());

        while let Some(module_id) = queue.pop_front() {
            if assigned.contains(&module_id) {
                continue;
            }

            assigned.insert(module_id.clone());
            chunk.push(module_id.clone());

            // Add direct dependencies
            if let Some(deps) = self.dependencies.get(&module_id) {
                for dep in deps {
                    if !assigned.contains(dep) {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        chunk
    }

    fn find_common_dependencies(&self) -> Vec<String> {
        let mut dep_count: HashMap<String, usize> = HashMap::new();

        for deps in self.dependencies.values() {
            for dep in deps {
                *dep_count.entry(dep.clone()).or_insert(0) += 1;
            }
        }

        // Return dependencies used by multiple modules
        dep_count
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(dep, _)| dep)
            .collect()
    }
}

impl WorkerPool {
    fn new(num_workers: usize) -> Self {
        let (task_tx, mut task_rx) = mpsc::channel::<WorkerTask>(1000);
        let (result_tx, result_rx) = mpsc::channel::<WorkerResult>(1000);

        let mut workers = Vec::new();

        for worker_id in 0..num_workers {
            let task_rx = task_rx.clone();
            let result_tx = result_tx.clone();

            let handle = tokio::spawn(async move {
                Self::worker_loop(worker_id, task_rx, result_tx).await;
            });

            workers.push(Worker {
                id: worker_id,
                handle,
            });
        }

        Self {
            workers,
            task_tx,
            result_rx,
        }
    }

    async fn worker_loop(
        worker_id: usize,
        mut task_rx: mpsc::Receiver<WorkerTask>,
        result_tx: mpsc::Sender<WorkerResult>,
    ) {
        while let Some(task) = task_rx.recv().await {
            match task {
                WorkerTask::Shutdown => break,
                task => {
                    let result = Self::process_task(worker_id, task).await;
                    if result_tx.send(result).await.is_err() {
                        break;
                    }
                }
            }
        }
    }

    async fn process_task(worker_id: usize, task: WorkerTask) -> WorkerResult {
        match task {
            WorkerTask::Transform { id, code, path } => {
                // Transform the module (simplified)
                let result = TransformResult {
                    code: format!("// Transformed by worker {}\n{}", worker_id, code),
                    source_map: None,
                    dependencies: Vec::new(), // Would extract from actual transformation
                };

                WorkerResult::Transformed { id, result }
            }
            WorkerTask::Bundle { modules, chunk_id } => {
                // Bundle modules into a chunk
                let combined_code = modules
                    .iter()
                    .map(|m| &m.code)
                    .collect::<Vec<_>>()
                    .join("\n");

                let chunk = ProcessedChunk {
                    id: chunk_id.clone(),
                    code: combined_code.clone(),
                    source_map: None,
                    dependencies: modules
                        .iter()
                        .flat_map(|m| m.dependencies.clone())
                        .collect(),
                    size: combined_code.len(),
                    hash: format!("{:x}", md5::compute(&combined_code)),
                };

                WorkerResult::Bundled { chunk_id, chunk }
            }
            WorkerTask::Minify { code, chunk_id } => {
                // Minify the code (simplified)
                let minified = code
                    .lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join("");

                WorkerResult::Minified {
                    chunk_id,
                    size: minified.len(),
                    code: minified,
                }
            }
            WorkerTask::Shutdown => unreachable!(),
        }
    }

    async fn submit_task(&self, task: WorkerTask) -> Result<()> {
        self.task_tx.send(task).await
            .map_err(|e| anyhow::anyhow!("Failed to submit task: {}", e))
    }

    async fn get_result(&mut self) -> Option<WorkerResult> {
        self.result_rx.recv().await
    }

    async fn shutdown(self) {
        // Send shutdown signals
        for _ in 0..self.workers.len() {
            let _ = self.task_tx.send(WorkerTask::Shutdown).await;
        }

        // Wait for workers to finish
        for worker in self.workers {
            let _ = worker.handle.await;
        }
    }
}

impl StreamingBuilder {
    pub fn new(
        config: BuildConfig,
        input_rx: mpsc::Receiver<BuildTask>,
        output_tx: mpsc::Sender<BuildResult>,
    ) -> Self {
        let worker_pool = WorkerPool::new(config.parallel_workers);
        let dependency_tracker = Arc::new(RwLock::new(DependencyTracker::new()));

        Self {
            input_rx,
            output_tx,
            worker_pool,
            dependency_tracker,
            config,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        let start_time = std::time::Instant::now();
        let mut processed_modules = 0;
        let mut ready_chunks = Vec::new();

        while let Some(task) = self.input_rx.recv().await {
            match task {
                BuildTask::ProcessModule { id, path, is_entry } => {
                    // Read and submit for transformation
                    let code = tokio::fs::read_to_string(&path).await?;

                    self.worker_pool
                        .submit_task(WorkerTask::Transform { id, code, path })
                        .await?;
                }
                BuildTask::CreateChunk { modules, chunk_id } => {
                    // Collect processed modules
                    let processed_modules: Vec<ProcessedModule> = modules
                        .into_iter()
                        .map(|id| ProcessedModule {
                            id: id.clone(),
                            code: format!("// Module: {}", id), // Would get from cache
                            dependencies: Vec::new(),
                            size: 100, // Placeholder
                        })
                        .collect();

                    self.worker_pool
                        .submit_task(WorkerTask::Bundle {
                            modules: processed_modules,
                            chunk_id,
                        })
                        .await?;
                }
                BuildTask::WriteChunk { chunk } => {
                    ready_chunks.push(chunk);

                    // Stream chunks to disk when threshold is reached
                    if ready_chunks.len() >= self.config.streaming_threshold {
                        self.write_chunks_to_disk(&mut ready_chunks).await?;
                    }
                }
                BuildTask::Finalize => {
                    // Write remaining chunks
                    self.write_chunks_to_disk(&mut ready_chunks).await?;

                    // Send completion
                    let stats = BuildStats {
                        total_modules: processed_modules,
                        total_chunks: ready_chunks.len(),
                        total_size: ready_chunks.iter().map(|c| c.size).sum(),
                        build_time_ms: start_time.elapsed().as_millis() as u64,
                        modules_per_second: processed_modules as f64
                            / start_time.elapsed().as_secs_f64(),
                    };

                    self.output_tx.send(BuildResult::BuildComplete { stats }).await?;
                    break;
                }
            }

            // Process worker results
            while let Ok(result) = tokio::time::timeout(
                std::time::Duration::from_millis(1),
                self.worker_pool.get_result(),
            )
            .await
            {
                if let Some(worker_result) = result {
                    self.handle_worker_result(worker_result).await?;
                    processed_modules += 1;
                }
            }
        }

        self.worker_pool.shutdown().await;
        Ok(())
    }

    async fn handle_worker_result(&self, result: WorkerResult) -> Result<()> {
        match result {
            WorkerResult::Transformed { id, result } => {
                // Add to dependency tracker
                {
                    let mut tracker = self.dependency_tracker.write().await;
                    tracker.add_module(id.clone(), result.dependencies.clone());
                }

                // Send result
                self.output_tx
                    .send(BuildResult::ModuleProcessed {
                        id,
                        code: result.code,
                        dependencies: result.dependencies,
                        size: result.code.len(),
                    })
                    .await?;
            }
            WorkerResult::Bundled { chunk_id, chunk } => {
                // Submit for minification
                self.worker_pool
                    .submit_task(WorkerTask::Minify {
                        code: chunk.code.clone(),
                        chunk_id: chunk_id.clone(),
                    })
                    .await?;
            }
            WorkerResult::Minified { chunk_id, code, size } => {
                // Create final chunk
                let chunk = ProcessedChunk {
                    id: chunk_id,
                    code,
                    source_map: None,
                    dependencies: Vec::new(),
                    size,
                    hash: format!("{:x}", md5::compute(&code)),
                };

                // Submit for writing
                self.input_rx
                    .send(BuildTask::WriteChunk { chunk })
                    .await
                    .map_err(|_| anyhow::anyhow!("Failed to submit write task"))?;
            }
            WorkerResult::Error { task, error } => {
                self.output_tx
                    .send(BuildResult::Error { task, error })
                    .await?;
            }
        }

        Ok(())
    }

    async fn write_chunks_to_disk(&self, chunks: &mut Vec<ProcessedChunk>) -> Result<()> {
        // Write chunks in parallel
        let write_tasks: Vec<_> = chunks
            .drain(..)
            .map(|chunk| {
                let output_dir = self.config.output_dir.clone();
                tokio::spawn(async move {
                    let file_path = output_dir.join(format!("{}.js", chunk.id));

                    // Ensure directory exists
                    if let Some(parent) = file_path.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }

                    // Write chunk
                    tokio::fs::write(&file_path, &chunk.code).await?;

                    Ok::<_, anyhow::Error>((chunk.id, file_path, chunk.size))
                })
            })
            .collect();

        // Wait for all writes to complete
        for task in write_tasks {
            let (chunk_id, file_path, size) = task.await??;

            self.output_tx
                .send(BuildResult::ChunkCreated {
                    chunk_id,
                    size,
                    file_path,
                })
                .await?;
        }

        Ok(())
    }
}

/// Streaming build coordinator
pub struct BuildCoordinator {
    config: BuildConfig,
}

impl BuildCoordinator {
    pub fn new(config: BuildConfig) -> Self {
        Self { config }
    }

    pub async fn build_streaming(
        &self,
        entry_points: Vec<PathBuf>,
        module_graph: &crate::graph::ModuleGraph,
    ) -> Result<BuildStats> {
        let (task_tx, task_rx) = mpsc::channel::<BuildTask>(1000);
        let (result_tx, mut result_rx) = mpsc::channel::<BuildResult>(1000);

        // Start streaming builder
        let builder = StreamingBuilder::new(self.config.clone(), task_rx, result_tx);
        let builder_handle = tokio::spawn(async move { builder.run().await });

        // Submit entry points
        for entry_path in entry_points {
            let id = entry_path.to_string_lossy().to_string();
            task_tx
                .send(BuildTask::ProcessModule {
                    id,
                    path: entry_path,
                    is_entry: true,
                })
                .await?;
        }

        // Process results
        let mut final_stats = None;

        while let Some(result) = result_rx.recv().await {
            match result {
                BuildResult::BuildComplete { stats } => {
                    final_stats = Some(stats);
                    break;
                }
                BuildResult::Error { task, error } => {
                    tracing::error!("Build error in {}: {}", task, error);
                }
                result => {
                    tracing::debug!("Build result: {:?}", result);
                }
            }
        }

        // Finalize
        task_tx.send(BuildTask::Finalize).await?;
        builder_handle.await??;

        final_stats.ok_or_else(|| anyhow::anyhow!("Build did not complete successfully"))
    }
}