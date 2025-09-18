// Memory optimization for large projects - key to beating Bun
use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock, Weak};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Memory pool for efficient allocation and reuse
pub struct MemoryPool<T> {
    pool: Vec<T>,
    factory: Box<dyn Fn() -> T + Send + Sync>,
    max_size: usize,
}

impl<T> MemoryPool<T> {
    pub fn new<F>(factory: F, max_size: usize) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            pool: Vec::with_capacity(max_size),
            factory: Box::new(factory),
            max_size,
        }
    }

    pub fn acquire(&mut self) -> T {
        self.pool.pop().unwrap_or_else(|| (self.factory)())
    }

    pub fn release(&mut self, item: T) {
        if self.pool.len() < self.max_size {
            self.pool.push(item);
        }
        // If pool is full, just drop the item (let GC handle it)
    }

    pub fn shrink(&mut self) {
        // Keep only half the items to reduce memory pressure
        let target_size = self.pool.len() / 2;
        self.pool.truncate(target_size);
        self.pool.shrink_to_fit();
    }
}

/// Memory-efficient string interning for file paths and module IDs
pub struct StringInterner {
    strings: HashMap<String, Arc<str>>,
    reverse_map: HashMap<*const str, Weak<str>>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            strings: HashMap::new(),
            reverse_map: HashMap::new(),
        }
    }

    pub fn intern(&mut self, s: &str) -> Arc<str> {
        if let Some(existing) = self.strings.get(s) {
            return existing.clone();
        }

        let interned: Arc<str> = s.into();
        let ptr = Arc::as_ptr(&interned);

        self.strings.insert(s.to_string(), interned.clone());
        self.reverse_map.insert(ptr, Arc::downgrade(&interned));

        interned
    }

    pub fn cleanup(&mut self) {
        // Remove dead weak references
        self.reverse_map.retain(|_, weak| weak.strong_count() > 0);

        // Remove strings that are no longer referenced
        self.strings.retain(|_, arc| Arc::strong_count(arc) > 1);
    }

    pub fn memory_usage(&self) -> usize {
        self.strings.iter()
            .map(|(k, v)| k.len() + v.len())
            .sum::<usize>() +
        self.reverse_map.len() * std::mem::size_of::<(*const str, Weak<str>)>()
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// LRU cache with memory pressure handling
pub struct LruCache<K, V> {
    map: HashMap<K, (V, Instant)>,
    access_order: VecDeque<K>,
    max_size: usize,
    max_memory: usize,
    current_memory: usize,
}

impl<K: Clone + Eq + std::hash::Hash, V: Clone> LruCache<K, V> {
    pub fn new(max_size: usize, max_memory: usize) -> Self {
        Self {
            map: HashMap::new(),
            access_order: VecDeque::new(),
            max_size,
            max_memory,
            current_memory: 0,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some((value, _)) = self.map.get_mut(key) {
            // Update access time
            let now = Instant::now();
            *value = (value.0.clone(), now);

            // Move to end of access order
            if let Some(pos) = self.access_order.iter().position(|k| k == key) {
                let key = self.access_order.remove(pos).unwrap();
                self.access_order.push_back(key);
            }

            Some(value.0.clone())
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        let memory_size = std::mem::size_of_val(&key) + std::mem::size_of_val(&value);

        // Remove existing entry if present
        if self.map.contains_key(&key) {
            self.remove(&key);
        }

        // Ensure we have space
        while (self.map.len() >= self.max_size) ||
              (self.current_memory + memory_size > self.max_memory) {
            self.evict_lru();
        }

        self.map.insert(key.clone(), (value, Instant::now()));
        self.access_order.push_back(key);
        self.current_memory += memory_size;
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some((value, _)) = self.map.remove(key) {
            if let Some(pos) = self.access_order.iter().position(|k| k == key) {
                self.access_order.remove(pos);
            }

            let memory_size = std::mem::size_of_val(key) + std::mem::size_of_val(&value);
            self.current_memory = self.current_memory.saturating_sub(memory_size);

            Some(value.0)
        } else {
            None
        }
    }

    fn evict_lru(&mut self) {
        if let Some(key) = self.access_order.pop_front() {
            self.remove(&key);
        }
    }

    pub fn shrink_to_fit(&mut self) {
        self.map.shrink_to_fit();
        self.access_order.shrink_to_fit();
    }

    pub fn memory_usage(&self) -> usize {
        self.current_memory
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Memory-aware module storage with automatic cleanup
pub struct ModuleStorage<T> {
    modules: LruCache<String, Arc<T>>,
    strong_refs: HashMap<String, Arc<T>>, // Keep important modules alive
    memory_threshold: usize,
    cleanup_interval: Duration,
    last_cleanup: Instant,
}

impl<T: Clone> ModuleStorage<T> {
    pub fn new(max_modules: usize, memory_threshold: usize) -> Self {
        Self {
            modules: LruCache::new(max_modules, memory_threshold),
            strong_refs: HashMap::new(),
            memory_threshold,
            cleanup_interval: Duration::from_secs(30),
            last_cleanup: Instant::now(),
        }
    }

    pub fn get(&mut self, id: &str) -> Option<Arc<T>> {
        // Check strong refs first (entry points, frequently used)
        if let Some(module) = self.strong_refs.get(id) {
            return Some(module.clone());
        }

        // Check LRU cache
        self.modules.get(id)
    }

    pub fn insert(&mut self, id: String, module: T, is_entry_point: bool) {
        let arc_module = Arc::new(module);

        if is_entry_point {
            // Keep entry points as strong references
            self.strong_refs.insert(id, arc_module);
        } else {
            self.modules.insert(id, arc_module);
        }

        self.maybe_cleanup();
    }

    pub fn remove(&mut self, id: &str) {
        self.strong_refs.remove(id);
        self.modules.remove(id);
    }

    pub fn mark_as_entry(&mut self, id: &str) {
        if let Some(module) = self.modules.remove(id) {
            self.strong_refs.insert(id.to_string(), module);
        }
    }

    pub fn unmark_as_entry(&mut self, id: &str) {
        if let Some(module) = self.strong_refs.remove(id) {
            self.modules.insert(id.to_string(), module);
        }
    }

    fn maybe_cleanup(&mut self) {
        if self.last_cleanup.elapsed() > self.cleanup_interval {
            self.cleanup();
            self.last_cleanup = Instant::now();
        }
    }

    pub fn cleanup(&mut self) {
        // Shrink collections
        self.modules.shrink_to_fit();
        self.strong_refs.shrink_to_fit();

        // Log memory usage
        let memory_usage = self.memory_usage();
        if memory_usage > self.memory_threshold * 3 / 4 {
            tracing::warn!("High memory usage: {} bytes", memory_usage);
        }
    }

    pub fn memory_usage(&self) -> usize {
        self.modules.memory_usage() +
        self.strong_refs.len() * std::mem::size_of::<(String, Arc<T>)>()
    }

    pub fn stats(&self) -> ModuleStorageStats {
        ModuleStorageStats {
            cached_modules: self.modules.len(),
            strong_refs: self.strong_refs.len(),
            memory_usage: self.memory_usage(),
            cache_hit_rate: 0.0, // Would need to track hits/misses
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModuleStorageStats {
    pub cached_modules: usize,
    pub strong_refs: usize,
    pub memory_usage: usize,
    pub cache_hit_rate: f64,
}

/// Memory monitoring and cleanup coordinator
pub struct MemoryManager {
    thresholds: MemoryThresholds,
    cleanup_tx: mpsc::Sender<CleanupRequest>,
    stats: Arc<RwLock<MemoryStats>>,
}

#[derive(Debug, Clone)]
pub struct MemoryThresholds {
    pub warning_bytes: usize,
    pub critical_bytes: usize,
    pub cleanup_interval: Duration,
}

impl Default for MemoryThresholds {
    fn default() -> Self {
        Self {
            warning_bytes: 500 * 1024 * 1024,   // 500 MB
            critical_bytes: 1024 * 1024 * 1024, // 1 GB
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_allocated: usize,
    pub cache_memory: usize,
    pub module_memory: usize,
    pub string_memory: usize,
    pub last_cleanup: Instant,
    pub cleanup_count: u64,
}

impl Default for MemoryStats {
    fn default() -> Self {
        Self {
            total_allocated: 0,
            cache_memory: 0,
            module_memory: 0,
            string_memory: 0,
            last_cleanup: Instant::now(),
            cleanup_count: 0,
        }
    }
}

#[derive(Debug)]
pub enum CleanupRequest {
    CacheCleanup,
    ModuleCleanup,
    StringCleanup,
    FullCleanup,
    Shutdown,
}

impl MemoryManager {
    pub fn new(thresholds: MemoryThresholds) -> Self {
        let (cleanup_tx, mut cleanup_rx) = mpsc::channel::<CleanupRequest>(100);
        let stats = Arc::new(RwLock::new(MemoryStats::default()));

        // Background cleanup task
        let cleanup_stats = stats.clone();
        tokio::spawn(async move {
            while let Some(request) = cleanup_rx.recv().await {
                match request {
                    CleanupRequest::Shutdown => break,
                    request => {
                        if let Err(e) = Self::handle_cleanup_request(request, &cleanup_stats).await {
                            tracing::error!("Cleanup error: {}", e);
                        }
                    }
                }
            }
        });

        Self {
            thresholds,
            cleanup_tx,
            stats,
        }
    }

    async fn handle_cleanup_request(
        request: CleanupRequest,
        stats: &Arc<RwLock<MemoryStats>>
    ) -> Result<()> {
        match request {
            CleanupRequest::CacheCleanup => {
                // Signal cache cleanup
                tracing::debug!("Performing cache cleanup");
            }
            CleanupRequest::ModuleCleanup => {
                // Signal module cleanup
                tracing::debug!("Performing module cleanup");
            }
            CleanupRequest::StringCleanup => {
                // Signal string interner cleanup
                tracing::debug!("Performing string cleanup");
            }
            CleanupRequest::FullCleanup => {
                // Full cleanup
                tracing::info!("Performing full memory cleanup");

                // Update stats
                if let Ok(mut stats) = stats.write() {
                    stats.last_cleanup = Instant::now();
                    stats.cleanup_count += 1;
                }
            }
            CleanupRequest::Shutdown => {
                // This is handled in the main loop
            }
        }
        Ok(())
    }

    pub fn check_memory_pressure(&self) -> MemoryPressure {
        if let Ok(stats) = self.stats.read() {
            if stats.total_allocated > self.thresholds.critical_bytes {
                MemoryPressure::Critical
            } else if stats.total_allocated > self.thresholds.warning_bytes {
                MemoryPressure::Warning
            } else {
                MemoryPressure::Normal
            }
        } else {
            MemoryPressure::Normal
        }
    }

    pub async fn request_cleanup(&self, request: CleanupRequest) -> Result<()> {
        self.cleanup_tx.send(request).await
            .map_err(|e| anyhow::anyhow!("Failed to send cleanup request: {}", e))
    }

    pub fn update_memory_usage(&self, category: MemoryCategory, bytes: usize) {
        if let Ok(mut stats) = self.stats.write() {
            match category {
                MemoryCategory::Cache => stats.cache_memory = bytes,
                MemoryCategory::Modules => stats.module_memory = bytes,
                MemoryCategory::Strings => stats.string_memory = bytes,
            }

            stats.total_allocated = stats.cache_memory +
                                   stats.module_memory +
                                   stats.string_memory;
        }
    }

    pub fn get_stats(&self) -> MemoryStats {
        self.stats.read().unwrap_or_else(|_| {
            std::sync::RwLockReadGuard::map(
                self.stats.read().unwrap(),
                |stats| stats
            )
        }).clone()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MemoryPressure {
    Normal,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy)]
pub enum MemoryCategory {
    Cache,
    Modules,
    Strings,
}

/// Zero-copy string slicing for parsing
pub struct ZeroCopyString {
    source: Arc<str>,
    start: usize,
    len: usize,
}

impl ZeroCopyString {
    pub fn new(source: Arc<str>) -> Self {
        let len = source.len();
        Self {
            source,
            start: 0,
            len,
        }
    }

    pub fn slice(&self, start: usize, end: usize) -> Self {
        assert!(start <= end && end <= self.len);
        Self {
            source: self.source.clone(),
            start: self.start + start,
            len: end - start,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.source[self.start..self.start + self.len]
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl AsRef<str> for ZeroCopyString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Memory-efficient AST node storage using arena allocation
pub struct AstArena {
    chunks: Vec<Vec<u8>>,
    current_chunk: usize,
    current_offset: usize,
    chunk_size: usize,
}

impl AstArena {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunks: vec![Vec::with_capacity(chunk_size)],
            current_chunk: 0,
            current_offset: 0,
            chunk_size,
        }
    }

    pub fn allocate<T>(&mut self, value: T) -> &mut T {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        // Align the offset
        let aligned_offset = (self.current_offset + align - 1) & !(align - 1);

        // Check if we need a new chunk
        if aligned_offset + size > self.chunk_size {
            self.chunks.push(Vec::with_capacity(self.chunk_size));
            self.current_chunk += 1;
            self.current_offset = 0;
            return self.allocate(value);
        }

        // Allocate in current chunk
        let chunk = &mut self.chunks[self.current_chunk];
        chunk.resize(aligned_offset + size, 0);

        let ptr = chunk.as_mut_ptr().wrapping_add(aligned_offset) as *mut T;
        unsafe {
            ptr.write(value);
            self.current_offset = aligned_offset + size;
            &mut *ptr
        }
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
        self.chunks.push(Vec::with_capacity(self.chunk_size));
        self.current_chunk = 0;
        self.current_offset = 0;
    }

    pub fn memory_usage(&self) -> usize {
        self.chunks.iter().map(|chunk| chunk.capacity()).sum()
    }
}