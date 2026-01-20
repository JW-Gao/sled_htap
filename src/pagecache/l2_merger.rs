use std::sync::{Arc, Weak};
use std::thread;
use std::time::Duration;
use crossbeam_channel::{bounded, Receiver, Sender};
use crate::pagecache::{PageCacheInner, PageId};
use crate::Result;
use crate::pin;
use std::sync::atomic::Ordering;

/// Task sent to the background merge worker.
/// We use a tuple of (Priority, PageId) to allow sorting.
#[derive(Debug, Clone, Copy)]
pub struct MergeTask {
    pub priority: f64,
    pub pid: PageId,
}

#[cfg(test)]
use std::sync::Mutex;

pub struct L2Merger {
    sender: Sender<MergeTask>,
    #[cfg(test)]
    pub processed: Arc<Mutex<Vec<PageId>>>,
}

impl L2Merger {
    pub fn start(
        weak_pc: Weak<PageCacheInner>, 
        scan_interval_ms: u64
    ) -> (Self, std::thread::JoinHandle<()>, Option<std::thread::JoinHandle<()>>) {
        let (tx, rx) = bounded(1024);
        
        #[cfg(test)]
        let processed = Arc::new(Mutex::new(Vec::new()));
        #[cfg(test)]
        let processed_clone = processed.clone();

        let weak_pc_worker = weak_pc.clone();
        let worker_handle = thread::spawn(move || {
            let thread = thread::current();
            let name = thread.name().unwrap_or("unknown");
            log::info!("L2 Merge Worker started on thread {}", name);
            
            #[cfg(test)]
            let res = merge_worker_loop(weak_pc_worker, rx, processed_clone);
            #[cfg(not(test))]
            let res = merge_worker_loop(weak_pc_worker, rx);

            if let Err(e) = res {
                log::error!("L2 Merge Worker encountered error: {:?}", e);
            }
        });

        let merger = Self { 
            sender: tx,
            #[cfg(test)]
            processed,
        };

        let mut scanner_handle = None;
        if scan_interval_ms > 0 {
            let merger_clone = merger.clone();
            let weak_pc_scanner = weak_pc.clone();
            scanner_handle = Some(thread::spawn(move || {
                background_scanner_loop(weak_pc_scanner, scan_interval_ms, merger_clone);
            }));
        }

        (merger, worker_handle, scanner_handle)
    }

    // Clone impl needed for passing to scanner
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            #[cfg(test)]
            processed: self.processed.clone(),
        }
    }

    pub fn submit(&self, pid: PageId, priority: f64) {
        if let Err(e) = self.sender.try_send(MergeTask { priority, pid }) {
            log::debug!("Failed to submit L2 merge task for pid {}: {:?}", pid, e);
        }
    }
}

fn background_scanner_loop(
    weak_pc: Weak<PageCacheInner>, 
    interval_ms: u64, 
    merger: L2Merger
) {
    let mut cursor: u64 = 0;
    let mut interval = Duration::from_millis(interval_ms);
    if interval_ms < 1 { interval = Duration::from_millis(1); }

    loop {
        thread::sleep(interval);
        
        let pc = match weak_pc.upgrade() {
            Some(pc) => pc,
            None => return,
        };

        let max_pid = pc.idgen.load(Ordering::Relaxed) as u64;
        if max_pid == 0 { continue; }

        let guard = pin();
        
        for _ in 0..100 {
            cursor = (cursor + 1) % max_pid;
            let pid = cursor;
            
            if let Ok(Some(node_view)) = pc.get(pid, &guard) {
                 // node_view is NodeView(PageView)
                 // Access PageView via .0
                 // metrics is in Page, via Deref from PageView
                 let priority = crate::pagecache::metrics::calculate_priority(
                     &node_view.0.metrics, 
                     1024, 
                     0, 
                     1.0, 
                     1.0
                 );
                 
                 if priority > 0.0 {
                     merger.submit(pid, priority);
                 }
            }
        }
    }
}


#[cfg(test)]
fn merge_worker_loop(
    weak_pc: Weak<PageCacheInner>, 
    rx: Receiver<MergeTask>,
    processed: Arc<Mutex<Vec<PageId>>>
) -> Result<()> {
    let mut buffer: Vec<MergeTask> = Vec::with_capacity(128);
    
    loop {
        if buffer.is_empty() {
            match rx.recv() {
                Ok(t) => buffer.push(t),
                Err(_) => return Ok(()), 
            }
        }
        
        if weak_pc.upgrade().is_none() {
            return Ok(());
        }
        
        while buffer.len() < 128 {
            if let Ok(task) = rx.try_recv() {
                buffer.push(task);
            } else {
                break;
            }
        }
        
        buffer.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
        
        if !buffer.is_empty() {
            let task = buffer.remove(0);
            
            if let Some(pc) = weak_pc.upgrade() {
                match merge_page_to_cold_log(&pc, task.pid) {
                    Ok(_) => {
                        processed.lock().unwrap().push(task.pid);
                    }
                    Err(e) => log::warn!("L2 merge failed for pid {}: {:?}", task.pid, e),
                }
            } else {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(not(test))]
fn merge_worker_loop(weak_pc: Weak<PageCacheInner>, rx: Receiver<MergeTask>) -> Result<()> {
    let mut buffer: Vec<MergeTask> = Vec::with_capacity(128);
    loop {
        if buffer.is_empty() {
            match rx.recv() {
                Ok(t) => buffer.push(t),
                Err(_) => return Ok(()), 
            }
        }
        if weak_pc.upgrade().is_none() {
            return Ok(());
        }
        while buffer.len() < 128 {
            if let Ok(task) = rx.try_recv() {
                buffer.push(task);
            } else {
                break;
            }
        }
        buffer.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
        if !buffer.is_empty() {
            let task = buffer.remove(0);
            if let Some(pc) = weak_pc.upgrade() {
                if let Err(e) = merge_page_to_cold_log(&pc, task.pid) {
                    log::warn!("L2 merge failed for pid {}: {:?}", task.pid, e);
                }
            } else {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

use crate::pagecache::Update;
use crate::pagecache::PageView;


use crate::schema::{TableSchema, DataType};
use crate::node::{Node, KeyRef};
use std::borrow::Cow;
use crate::{IVec}; // Ensure IVec is available if needed

fn merge_page_to_cold_log(pc: &Arc<PageCacheInner>, pid: PageId) -> Result<()> {
    let guard = pin();
    let view_opt = pc.get(pid, &guard)?;
    
    if let Some(view) = view_opt {
        // If already cold, skip
        if let Some(info) = view.0.cache_infos.first() {
            if info.pointer.is_cold() {
                return Ok(());
            }
        }
        
        // Check if it's a Node update
        let old_node = match &view.0.update {
            Some(Update::Node(node)) => node,
            _ => return Ok(()),
        };

        // --- Schema-Aware Transposition Logic ---
        
        // Define Lineitem Q6 Schema (Hardcoded for this experiment)
        // Col1: quantity (f32), Col2: extendedprice (f32), Col3: discount (f32), Col4: shipdate (f32)
        // Ingestion stored them as [f32; 4] = 16 bytes?
        // Wait, Ingestion script "Rows are stripped of values".
        // If Ingestion script sends 4 * f32, then value len is 16.
        // Let's assume input values are 16 bytes + maybe others?
        // User said: "Ingestion Script reads TBL... application only keeps 4 cols... db.insert".
        // So Value is exactly 16 bytes (4 floats).
        // Let's verify compatibility.
        
        let schema = TableSchema::new(
            "lineitem_q6",
            vec![
                ("quantity", DataType::F32),
                ("extendedprice", DataType::F32),
                ("discount", DataType::F32),
                ("shipdate", DataType::F32), // Ingested as f32
            ],
            vec![0, 1, 2, 3], // All 4 are columnar
            vec![], // Projection handled at ingest? Or here? 
                    // "Filter-at-Ingest": Data already projected.
        );

        let mut transposed_node = None;
        
        // Only try to transpose Leaf Nodes (is_index=false)
        if !old_node.is_index {
             // We need to collect items to pass to new_columnar
             // Iterating old_node to check if it matches schema
             let items: Vec<(KeyRef, Cow<[u8]>)> = old_node.iter().collect();
             
             // Check if data is consistent with Schema (16 bytes per value)
             let is_lineitem_chunk = !items.is_empty() && items.iter().all(|(_, v)| v.len() == 16);
             
             if is_lineitem_chunk {
                 // Transpose!
                 let count = items.len();
                 let mut col1 = Vec::with_capacity(count * 4);
                 let mut col2 = Vec::with_capacity(count * 4);
                 let mut col3 = Vec::with_capacity(count * 4);
                 let mut col4 = Vec::with_capacity(count * 4);
                 
                 for (_, val) in &items {
                     // Split 16 bytes into 4 chunks
                     // Val is [f32, f32, f32, f32] (LE)
                     col1.extend_from_slice(&val[0..4]);
                     col2.extend_from_slice(&val[4..8]);
                     col3.extend_from_slice(&val[8..12]);
                     col4.extend_from_slice(&val[12..16]);
                 }
                 
                 // Concat all columns
                 let mut blob = Vec::with_capacity(count * 16);
                 let off1 = 0u32;
                 blob.extend_from_slice(&col1);
                 
                 let off2 = blob.len() as u32;
                 blob.extend_from_slice(&col2);
                 
                 let off3 = blob.len() as u32;
                 blob.extend_from_slice(&col3);
                 
                 let off4 = blob.len() as u32;
                 blob.extend_from_slice(&col4);
                 
                 let offsets = vec![off1, off2, off3, off4];
                 
                 // Create Columnar Node
                 // We pass `items` as well so Keys are preserved in the Node body! (Critical for find/scan boundaries)
                 // Node::new_columnar will write keys.
                 // Values in `items` are ignored/cleared by new_columnar implementation?
                 // Wait, my `new_columnar` impl iterates `items`. It writes `k` but for value?
                 // My `new_columnar` implementation IGNORES `v` in the loop!
                 // "for (k, _) in items". YES.
                 // So we don't store 16-byte vals in Node body. Good.
                 
                 let new_node = Node::new_columnar(
                     old_node.lo(),
                     old_node.hi(),
                     old_node.inner.prefix_len,
                     old_node.inner.next, 
                     &items,
                     &blob,
                     &offsets,
                 );
                 transposed_node = Some(new_node);
             }
        }
        
        let update_node = transposed_node.unwrap_or_else(|| old_node.clone());

        // Construct PageView.
        let old_view = PageView {
            read: view.0.read,
            entry: view.0.entry,
        };

        // CAS logic
        // If we transposed, we replaced the Node.
        // Code: Update::Node(update_node)
        match pc.cas_page(pid, old_view, Update::Node(update_node), true, &guard) {
            Ok(Ok(_)) => {
                view.0.metrics.reset_after_merge(0); 
            }
            Ok(Err(_)) => {
                log::trace!("L2 merge CAS failed for pid {}", pid);
            }
            Err(e) => return Err(e),
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    #[test]
    fn test_l2_merger_priority_queue() {
        let config = Config::new().temporary(true);
        let db = config.open().unwrap();
        
        let l2_merger_guard = db.context.pagecache.0.l2_merger.lock();
        let l2_merger = match &*l2_merger_guard {
            Some(l2) => l2.clone(),
            None => panic!("L2Merger should be initialized on DB start"),
        };
            
        db.insert(b"k", b"v").unwrap();
        let root_pid = db.default.root.load(std::sync::atomic::Ordering::Relaxed);

        l2_merger.submit(root_pid, 100.0);
        
        thread::sleep(Duration::from_millis(500));
        
        let processed = l2_merger.processed.lock().unwrap();
        assert!(processed.contains(&root_pid), "Root PID should be processed");
        
        let guard = pin();
        let view = db.context.pagecache.get(root_pid, &guard).unwrap().expect("Root page should exist");
        if let Some(info) = view.0.cache_infos.first() {
             assert!(info.pointer.is_cold(), "Root page should be in Cold Log after merge");
        } else {
             panic!("Root page should have cache info");
        }
    }
}
