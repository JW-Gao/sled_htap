use crate::{pin, Batch, FastMap8, IVec, Result, Tree};
use crate::ebr::{Atomic, Owned};

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// A very simple two-level overlay for `Tree` that mimics a write-optimized
/// Level 1 (in-memory) over a Level 2 (the underlying sled `Tree`).
///
/// Goals:
/// - Keep it dead-simple and safe.
/// - Favor write-heavy or read-after-write workloads by serving from L1.
/// - Periodically flush L1 into L2 using batches.
///
/// This is intentionally minimal and not intended for production use. It is
/// useful for experiments and benchmarks that want to approximate an L1/L2
/// system with PUSH (flush) and a light PULL (optional promote on read).
#[derive(Clone)]
enum L1Value { Row(IVec), Tombstone }

type L1Map = FastMap8<IVec, L1Value>;
type AccessCountMap = FastMap8<IVec, Arc<AtomicU64>>;

pub struct OverlayTree {
    base: Tree,
    l1: Atomic<L1Map>,
    access_counts: Atomic<AccessCountMap>,
    flush_threshold: usize,
    flush_interval: Duration,
    promote_on_read: bool,
    promote_threshold: u64, // 访问次数阈值，达到此值才提升到 L1
    columns: usize,
    stop: Arc<AtomicBool>,
    flusher: Option<JoinHandle<()>>,
}

impl OverlayTree {
    /// Create a new overlay over an existing `Tree`.
    ///
    /// - `flush_threshold`: L1 size that triggers a batch flush.
    /// - `flush_interval`: time-based periodic flush.
    /// - `promote_on_read`: whether to copy hot keys from L2 into L1 on read hits.
    pub fn new(base: Tree, flush_threshold: usize, flush_interval: Duration, promote_on_read: bool) -> Self {
        Self::with_columns(base, flush_threshold, flush_interval, promote_on_read, 1)
    }

    /// Create with explicit column count for L2 columnar layout.
    pub fn with_columns(base: Tree, flush_threshold: usize, flush_interval: Duration, promote_on_read: bool, columns: usize) -> Self {
        Self::with_columns_and_promote_threshold(base, flush_threshold, flush_interval, promote_on_read, columns, 3)
    }

    /// Create with explicit column count and promote threshold.
    /// - `promote_threshold`: 访问次数达到此值才提升到 L1（默认 3）
    pub fn with_columns_and_promote_threshold(base: Tree, flush_threshold: usize, flush_interval: Duration, promote_on_read: bool, columns: usize, promote_threshold: u64) -> Self {
        let l1 = Atomic::new(L1Map::default());
        let access_counts = Atomic::new(AccessCountMap::default());
        let stop = Arc::new(AtomicBool::new(false));

        let l1_for_thread = l1.clone();
        let base_for_thread = base.clone();
        let stop_for_thread = Arc::clone(&stop);
        let columns_for_thread = columns;

        let flusher = Some(thread::spawn(move || {
            let mut last = Instant::now();
            while !stop_for_thread.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(5));

                let guard = pin();
                let cur = unsafe { l1_for_thread.load(Ordering::Acquire, &guard).deref() };
                let due_time = last.elapsed() >= flush_interval;
                let due_size = cur.len() >= flush_threshold;
                if !(due_time || due_size) { continue; }

                last = Instant::now();

                // Drain L1 using RCU swap, then flush drained.
                let guard = pin();
                let prev = l1_for_thread.swap(Owned::new(L1Map::default()), Ordering::AcqRel, &guard);
                let drained_ref = unsafe { prev.deref() };
                if drained_ref.is_empty() { continue; }

                let mut batch = Batch::default();
                for (k, v) in drained_ref.iter() {
                    match v {
                        // convert row in L1 to column entries in L2
                        L1Value::Row(val) => {
                            let cols = split_cols(val.clone());
                            for (i, c) in cols.iter().enumerate() { batch.insert(make_col_key(k.as_ref(), i), c.clone()); }
                        }
                        L1Value::Tombstone => {
                            for i in 0..columns_for_thread { batch.remove(make_col_key(k.as_ref(), i)); }
                        }
                    };
                }

                // Best-effort apply; ignore errors for simplicity in experiments.
                let _ = base_for_thread.apply_batch(batch);
            }
        }));

        Self { base, l1, access_counts, flush_threshold, flush_interval, promote_on_read, promote_threshold, columns, stop, flusher }
    }

    /// Insert into L1. Returns previous value if visible (from L1 or L2).
    pub fn insert<K: AsRef<[u8]>, V: Into<IVec>>(&self, key: K, value: V) -> Result<Option<IVec>> {
        // Read previous value via get which checks L1 then L2.
        let prev = self.get(key.as_ref())?;
        let key_vec = IVec::from(key.as_ref());
        self.rcu_insert(key_vec, L1Value::Row(value.into()));
        Ok(prev)
    }

    /// Remove in L1. Returns previous value.
    pub fn remove<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<IVec>> {
        let prev = self.get(key.as_ref())?;
        self.rcu_insert(IVec::from(key.as_ref()), L1Value::Tombstone);
        Ok(prev)
    }

    /// Get from L1 first, then fall back to L2. Optionally promote hot items to L1 based on access frequency.
    pub fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<IVec>> {
        let key_ref = key.as_ref();
        if let Some(v) = self.l1_get_value(key_ref) {
            return Ok(v);
        }

        // Reconstruct from L2 columns if possible
        let mut cols = Vec::with_capacity(self.columns);
        for i in 0..self.columns {
            match self.base.get(make_col_key(key_ref, i))? {
                Some(c) => cols.push(c),
                None => { return Ok(None); }
            }
        }
        let row = join_cols_vec(&cols);
        
        // 基于访问频率的热点提升策略
        if self.promote_on_read {
            let key_vec = IVec::from(key_ref);
            let access_count = self.increment_access_count(&key_vec);
            
            // 只有访问次数达到阈值才提升到 L1
            if access_count >= self.promote_threshold {
                let guard = pin();
                let cur_len = unsafe { self.l1.load(Ordering::Acquire, &guard).deref() }.len();
                if cur_len < self.flush_threshold {
                    self.rcu_insert(key_vec, L1Value::Row(row.clone()));
                }
            }
        }
        Ok(Some(row))
    }

    /// Get a single column by index. Returns None if out of range or not found.
    pub fn get_column<K: AsRef<[u8]>>(&self, key: K, column_index: usize) -> Result<Option<IVec>> {
        let key_ref = key.as_ref();
        if let Some(opt) = self.l1_get_column(key_ref, column_index) { return Ok(opt); }

        Ok(self.base.get(make_col_key(key_ref, column_index))?)
    }

    /// Force a synchronous flush of the current L1 state.
    pub fn flush_now(&self) -> Result<()> {
        let _ = self.flush_interval; // access to silence unused field warning in minimal build
        let guard = pin();
        let prev = self.l1.swap(Owned::new(L1Map::default()), Ordering::AcqRel, &guard);
        let drained = unsafe { prev.deref() };
        if drained.is_empty() { return Ok(()); }
        let mut batch = Batch::default();
        for (k, v) in drained.iter() {
            match v {
                L1Value::Row(val) => {
                    let cols = split_cols(val.clone());
                    for (i, c) in cols.iter().enumerate() { batch.insert(make_col_key(k.as_ref(), i), c.clone()); }
                }
                L1Value::Tombstone => {
                    for i in 0..self.columns { batch.remove(make_col_key(k.as_ref(), i)); }
                }
            };
        }
        let _ = self.base.apply_batch(batch)?;
        Ok(())
    }

    fn l1_get_value(&self, key: &[u8]) -> Option<Option<IVec>> {
        let guard = pin();
        let map = unsafe { self.l1.load(Ordering::Acquire, &guard).deref() };
        map.get(key).map(|v| match v { L1Value::Row(val) => Some(val.clone()), L1Value::Tombstone => None })
    }

    fn l1_get_column(&self, key: &[u8], idx: usize) -> Option<Option<IVec>> {
        let guard = pin();
        let map = unsafe { self.l1.load(Ordering::Acquire, &guard).deref() };
        map.get(key).map(|v| match v { L1Value::Row(val) => split_cols(val.clone()).get(idx).cloned(), L1Value::Tombstone => None })
    }

    fn increment_access_count(&self, key: &IVec) -> u64 {
        loop {
            let guard = pin();
            let cur_ptr = self.access_counts.load(Ordering::Acquire, &guard);
            let cur = unsafe { cur_ptr.deref() };
            
            // 如果已存在，直接递增
            if let Some(counter) = cur.get(key) {
                let new_val = counter.fetch_add(1, Ordering::Relaxed) + 1;
                return new_val;
            }
            
            // 不存在，需要添加新的计数器
            let mut next = cur.clone();
            next.insert(key.clone(), Arc::new(AtomicU64::new(1)));
            match self.access_counts.compare_and_set(cur_ptr, Owned::new(next), (Ordering::AcqRel, Ordering::Acquire), &guard) {
                Ok(_) => return 1,
                Err(_) => continue, // 重试
            }
        }
    }

    fn rcu_insert(&self, key: IVec, val: L1Value) {
        loop {
            let guard = pin();
            let cur_ptr = self.l1.load(Ordering::Acquire, &guard);
            let cur = unsafe { cur_ptr.deref() };
            let mut next = cur.clone();
            next.insert(key.clone(), val.clone());
            match self.l1.compare_and_set(cur_ptr, Owned::new(next), (Ordering::AcqRel, Ordering::Acquire), &guard) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    }
}

impl Drop for OverlayTree {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.flusher.take() { let _ = h.join(); }
        let _ = self.flush_now();
    }
}

fn split_cols(row: IVec) -> Vec<IVec> {
    const DELIM: u8 = b'|';
    let mut cols = Vec::new();
    let mut start = 0usize;
    let bytes = row.as_ref();
    for (i, b) in bytes.iter().enumerate() {
        if *b == DELIM { cols.push(IVec::from(&bytes[start..i])); start = i + 1; }
    }
    cols.push(IVec::from(&bytes[start..]));
    cols
}

fn join_cols_vec(cols: &Vec<IVec>) -> IVec {
    const DELIM: u8 = b'|';
    let total_len: usize = if cols.is_empty() { 0 } else { cols.iter().map(|c| c.len()).sum::<usize>() + (cols.len() - 1) };
    let mut buf = Vec::with_capacity(total_len);
    for (i, c) in cols.iter().enumerate() {
        if i > 0 { buf.push(DELIM); }
        buf.extend_from_slice(c.as_ref());
    }
    IVec::from(buf)
}

fn make_col_key(row_key: &[u8], col_idx: usize) -> IVec {
    // format: b"c|<u16_be>|" + row_key
    let mut out = Vec::with_capacity(2 + 2 + 1 + row_key.len());
    out.extend_from_slice(b"c|");
    let idx = col_idx as u16;
    out.extend_from_slice(&idx.to_be_bytes());
    out.push(b'|');
    out.extend_from_slice(row_key);
    IVec::from(out)
}

impl OverlayTree {
    /// Aggregate: sum of a numeric column encoded as BE u64 in L2 column entries
    pub fn sum_column(&self, column_index: usize) -> Result<u128> {
        let mut total: u128 = 0;
        let prefix = make_col_key(&[], column_index);
        let mut it = self.base.range(prefix.as_ref()..);
        while let Some(Ok((k, v))) = it.next() {
            if !k.starts_with(prefix.as_ref()) { break; }
            if v.len() == 8 {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(v.as_ref());
                total += u64::from_be_bytes(arr) as u128;
            }
        }
        Ok(total)
    }

    pub fn avg_column(&self, column_index: usize) -> Result<Option<f64>> {
        let mut total: u128 = 0;
        let mut cnt: u128 = 0;
        let prefix = make_col_key(&[], column_index);
        let mut it = self.base.range(prefix.as_ref()..);
        while let Some(Ok((k, v))) = it.next() {
            if !k.starts_with(prefix.as_ref()) { break; }
            if v.len() == 8 {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(v.as_ref());
                total += u64::from_be_bytes(arr) as u128;
                cnt += 1;
            }
        }
        if cnt == 0 { return Ok(None); }
        Ok(Some(total as f64 / cnt as f64))
    }

    /// Range aggregation: sum of a column within key range [k_start, k_end)
    pub fn sum_column_range(&self, column_index: usize, k_start: &[u8], k_end: &[u8]) -> Result<u128> {
        let mut total: u128 = 0;
        let col_prefix = make_col_key(&[], column_index);
        let start_key = make_col_key(k_start, column_index);
        let end_key = make_col_key(k_end, column_index);
        let mut it = self.base.range(start_key.as_ref()..end_key.as_ref());
        while let Some(Ok((k, v))) = it.next() {
            if !k.starts_with(col_prefix.as_ref()) { break; }
            if v.len() == 8 {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(v.as_ref());
                total += u64::from_be_bytes(arr) as u128;
            }
        }
        Ok(total)
    }

    /// Range aggregation: average of a column within key range [k_start, k_end)
    pub fn avg_column_range(&self, column_index: usize, k_start: &[u8], k_end: &[u8]) -> Result<Option<f64>> {
        let mut total: u128 = 0;
        let mut cnt: u128 = 0;
        let col_prefix = make_col_key(&[], column_index);
        let start_key = make_col_key(k_start, column_index);
        let end_key = make_col_key(k_end, column_index);
        let mut it = self.base.range(start_key.as_ref()..end_key.as_ref());
        while let Some(Ok((k, v))) = it.next() {
            if !k.starts_with(col_prefix.as_ref()) { break; }
            if v.len() == 8 {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(v.as_ref());
                total += u64::from_be_bytes(arr) as u128;
                cnt += 1;
            }
        }
        if cnt == 0 { return Ok(None); }
        Ok(Some(total as f64 / cnt as f64))
    }

    /// Range aggregation statistics: returns (sum, sum of squares, count)
    pub fn column_range_stats(
        &self,
        column_index: usize,
        k_start: &[u8],
        k_end: &[u8],
    ) -> Result<(u128, u128, u64)> {
        let mut sum: u128 = 0;
        let mut sum_sq: u128 = 0;
        let mut count: u64 = 0;

        let col_prefix = make_col_key(&[], column_index);
        let start_key = make_col_key(k_start, column_index);
        let end_key = make_col_key(k_end, column_index);
        let mut it = self.base.range(start_key.as_ref()..end_key.as_ref());
        while let Some(Ok((k, v))) = it.next() {
            if !k.starts_with(col_prefix.as_ref()) { break; }
            if v.len() == 8 {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(v.as_ref());
                let val = u64::from_be_bytes(arr) as u128;
                sum += val;
                sum_sq += val * val;
                count += 1;
            }
        }
        Ok((sum, sum_sq, count))
    }
}

