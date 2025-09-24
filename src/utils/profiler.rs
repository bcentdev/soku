use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

/// Performance profiler for identifying bottlenecks
pub struct UltraProfiler {
    timings: Arc<Mutex<HashMap<String, Vec<Duration>>>>,
    active_timers: Arc<Mutex<HashMap<String, Instant>>>,
}

impl UltraProfiler {
    pub fn new() -> Self {
        Self {
            timings: Arc::new(Mutex::new(HashMap::new())),
            active_timers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start_timer(&self, name: &str) {
        let mut timers = self.active_timers.lock().unwrap();
        timers.insert(name.to_string(), Instant::now());
    }

    pub fn end_timer(&self, name: &str) -> Duration {
        let mut timers = self.active_timers.lock().unwrap();
        let start = timers.remove(name).unwrap_or_else(Instant::now);
        let duration = start.elapsed();

        let mut timings = self.timings.lock().unwrap();
        timings.entry(name.to_string()).or_default().push(duration);

        duration
    }

    pub fn get_stats(&self) -> ProfilerStats {
        let timings = self.timings.lock().unwrap();
        let mut stats = ProfilerStats {
            total_time: Duration::new(0, 0),
            bottlenecks: Vec::new(),
        };

        for (name, durations) in timings.iter() {
            let total: Duration = durations.iter().sum();
            let avg = total / (durations.len() as u32);
            let max = *durations.iter().max().unwrap_or(&Duration::new(0, 0));

            stats.total_time += total;
            stats.bottlenecks.push(BottleneckInfo {
                name: name.clone(),
                total_time: total,
                average_time: avg,
                max_time: max,
                call_count: durations.len(),
            });
        }

        // Sort by total time (biggest bottlenecks first)
        stats.bottlenecks.sort_by(|a, b| b.total_time.cmp(&a.total_time));
        stats
    }

    pub fn report_bottlenecks(&self) {
        let stats = self.get_stats();

        println!("🔍 Performance Profile:");
        println!("   Total build time: {:?}", stats.total_time);
        println!("   Top bottlenecks:");

        for (i, bottleneck) in stats.bottlenecks.iter().take(5).enumerate() {
            let percentage = (bottleneck.total_time.as_millis() as f64 / stats.total_time.as_millis() as f64) * 100.0;
            println!("   {}. {} - {:?} ({:.1}% of total, {} calls)",
                i + 1,
                bottleneck.name,
                bottleneck.total_time,
                percentage,
                bottleneck.call_count
            );
        }
    }
}

impl Default for UltraProfiler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct ProfilerStats {
    pub total_time: Duration,
    pub bottlenecks: Vec<BottleneckInfo>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct BottleneckInfo {
    pub name: String,
    pub total_time: Duration,
    #[allow(dead_code)]
    pub average_time: Duration,
    #[allow(dead_code)]
    pub max_time: Duration,
    pub call_count: usize,
}

/// RAII timer for automatic timing
#[allow(dead_code)]
pub struct ScopedTimer<'a> {
    profiler: &'a UltraProfiler,
    name: String,
}

impl<'a> ScopedTimer<'a> {
    pub fn new(profiler: &'a UltraProfiler, name: &str) -> Self {
        profiler.start_timer(name);
        Self {
            profiler,
            name: name.to_string(),
        }
    }
}

impl<'a> Drop for ScopedTimer<'a> {
    fn drop(&mut self) {
        self.profiler.end_timer(&self.name);
    }
}

// Macro para timing fácil
#[macro_export]
macro_rules! profile {
    ($profiler:expr, $name:expr, $block:block) => {
        {
            let _timer = $crate::utils::profiler::ScopedTimer::new($profiler, $name);
            $block
        }
    };
}