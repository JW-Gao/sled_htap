use std::sync::atomic::{AtomicUsize, Ordering};

/// Metrics associated with a Logical Page to support L2 Merge Priority calculation.
#[derive(Debug, Default)]
pub struct PageMetrics {
    /// Number of read operations (Get) on this page.
    pub read_count: AtomicUsize,
    /// Number of column scan operations on this page.
    pub scan_count: AtomicUsize,
    /// Number of updates (deltas) accumulated since last merge.
    /// This roughly maps to Delta Chain Length.
    pub delta_count: AtomicUsize,
    /// Estimated size of deltas in bytes.
    pub delta_size: AtomicUsize,
    /// Timestamp of the last merge (in system ticks or similar monotonic counter).
    /// Used to calculate WaitTime.
    pub last_merge_ts: AtomicUsize,
}

impl PageMetrics {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn mark_read(&self) {
        self.read_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn mark_scan(&self) {
        self.scan_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn mark_update(&self, size: usize) {
        self.delta_count.fetch_add(1, Ordering::Relaxed);
        self.delta_size.fetch_add(size, Ordering::Relaxed);
    }
    
    pub fn reset_after_merge(&self, current_ts: usize) {
        // Reset counters but maybe keep some history (decayed)? 
        // For now, simple reset to avoid infinite accumulation.
        self.read_count.store(0, Ordering::Relaxed);
        self.scan_count.store(0, Ordering::Relaxed);
        self.delta_count.store(0, Ordering::Relaxed);
        self.delta_size.store(0, Ordering::Relaxed);
        self.last_merge_ts.store(current_ts, Ordering::Relaxed);
    }
}

/// Calculate the merge priority score for a page.
/// 
/// Formula: Priority(p) = [R(p) * S(p) * (1 + U(p))] / C(p)
/// 
/// R(p): Read Benefit = ReadFreq * ColAccessRatio
/// S(p): Space Benefit = DeltaSize * (1 - 1/Compression)
/// U(p): Urgency = w1 * DeltaRatio + w2 * WaitTime
/// C(p): Cost = PageSize (Base + Delta)
pub fn calculate_priority(
    metrics: &PageMetrics,
    base_size: usize,
    check_ts: usize,
    w1: f64,
    w2: f64
) -> f64 {
    let read_count = metrics.read_count.load(Ordering::Relaxed) as f64;
    let scan_count = metrics.scan_count.load(Ordering::Relaxed) as f64;
    let delta_size = metrics.delta_size.load(Ordering::Relaxed) as f64;
    let delta_count = metrics.delta_count.load(Ordering::Relaxed) as f64;
    let last_merge = metrics.last_merge_ts.load(Ordering::Relaxed) as f64;
    
    // 1. R(p) - Read Benefit
    let total_reads = read_count + scan_count;
    if total_reads < 1.0 {
        // Minimal activity, prioritize low.
        return 0.0;
    }
    let col_ratio = if total_reads > 0.0 { scan_count / total_reads } else { 0.0 };
    // Weight column scans higher? Or just use the ratio. 
    // Let's assume raw frequency * ratio.
    let r_p = total_reads * (0.5 + 0.5 * col_ratio); 

    // 2. S(p) - Space Benefit
    // If no deltas, no benefit.
    if delta_count < 1.0 {
        return 0.0;
    }
    let compression_ratio = 2.0; // Default estimate
    let s_p = delta_size * (1.0 - 1.0 / compression_ratio);

    // 3. U(p) - Urgency
    let base_bytes = base_size as f64;
    // Protect against div/0
    let safe_base = if base_bytes < 1.0 { 4096.0 } else { base_bytes };
    let delta_ratio = delta_size / safe_base;
    let wait_time = (check_ts as f64).max(last_merge) - last_merge;
    
    let u_p = w1 * delta_ratio + w2 * wait_time;

    // 4. C(p) - Cost
    let total_page_size = safe_base + delta_size;
    let c_p = total_page_size;

    // Final Calculation
    let priority = (r_p * s_p * (1.0 + u_p)) / c_p;
    
    priority
}
