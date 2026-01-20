use sled::{Config, Db};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::env;

const ITEM_COUNT: usize = 1_000_000;
const DURATION_SECS: u64 = 10;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut thread_count = 1;
    let mut mode = "lockfree".to_string();
    let mut read_ratio = 0; // 0-100

    // Simple arg parsing
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--threads" => {
                thread_count = args[i+1].parse().unwrap();
                i += 1;
            }
            "--mode" => {
                mode = args[i+1].clone();
                i += 1;
            }
            "--read-ratio" => {
                read_ratio = args[i+1].parse().unwrap();
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    println!("Starting Scalability Test: Threads={}, Mode={}, ReadRatio={}%", thread_count, mode, read_ratio);

    let path = format!("bench_scalability_{}_{}.db", mode, thread_count); // Unique DB per run to avoid lock issues
    let _ = std::fs::remove_dir_all(&path);

    let config = Config::default()
        .path(&path)
        .cache_capacity(2 * 1024 * 1024 * 1024) // 2GB Cache (Memory Bound)
        .flush_every_ms(None); // Disable background flush interference

    let db = config.open().unwrap();

    // Pre-fill
    println!("Pre-filling {} items...", ITEM_COUNT);
    for i in 0..ITEM_COUNT {
        if i % 200_000 == 0 { println!("Filled {}", i); }
        let k = format!("{:016}", i);
        let v = format!("val_{:08}", i);
        db.insert(k.as_bytes(), v.as_bytes()).unwrap();
    }
    // No flush here, keep in memory if possible? actually flush ensures structure is stable, but we want memory resident.
    // With 2GB cache, it stays on dirty pages in memory mostly.
    
    // Prepare for Threading
    let stop_signal = Arc::new(AtomicBool::new(false));
    let total_ops = Arc::new(AtomicUsize::new(0));

    let mut handles = vec![];

    // Shared State Wrapper
    // For LockFree: Arc<Db>
    // For Mutex: Arc<Mutex<Db>>
    // Since types are different, we handle logic inside thread closure conditionally or via enum? 
    // Simpler: Just always wrap in a struct that might have a mutex.
    
    // Actually, cloning Db is cheap.
    // Mutex Mode: We need a SINGLE Mutex guarding the Db.
    let mutex_db = if mode == "mutex" {
        Some(Arc::new(Mutex::new(db.clone())))
    } else {
        None
    };
    
    // For LockFree, we clone the raw db
    let lockfree_db = if mode == "lockfree" {
        Some(db.clone())
    } else {
        None
    };

    let start_time = Instant::now();

    for t_idx in 0..thread_count {
        let stop = stop_signal.clone();
        let ops = total_ops.clone();
        let my_mutex = mutex_db.clone();
        let my_lockfree = lockfree_db.clone();
        let my_mode = mode.clone();
        
        handles.push(thread::spawn(move || {
            let mut rng = StdRng::seed_from_u64((t_idx as u64) + 100);
            
            while !stop.load(Ordering::Relaxed) {
                let key_int = rng.gen_range(0, ITEM_COUNT);
                let key = format!("{:016}", key_int);
                let is_read = rng.gen_range(0, 100) < read_ratio;

                if my_mode == "mutex" {
                    let guard = my_mutex.as_ref().unwrap().lock().unwrap();
                    if is_read {
                        let _ = guard.get(key.as_bytes()).unwrap();
                    } else {
                        let val = format!("val_{}_{}", key_int, rng.gen::<u32>());
                        let _ = guard.insert(key.as_bytes(), val.as_bytes()).unwrap();
                    }
                    // lock released here
                } else {
                    // Lock-Free
                    let db_ref = my_lockfree.as_ref().unwrap();
                    if is_read {
                        let _ = db_ref.get(key.as_bytes()).unwrap();
                    } else {
                        let val = format!("val_{}_{}", key_int, rng.gen::<u32>());
                        let _ = db_ref.insert(key.as_bytes(), val.as_bytes()).unwrap();
                    }
                }
                
                ops.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    thread::sleep(Duration::from_secs(DURATION_SECS));
    stop_signal.store(true, Ordering::Relaxed);

    for h in handles {
        h.join().unwrap();
    }

    let ops_count = total_ops.load(Ordering::Relaxed);
    let qps = ops_count as f64 / DURATION_SECS as f64;

    println!("Scalability Test Complete.");
    println!("Total OPS: {}", ops_count);
    println!("QPS: {:.2}", qps);
    
    // Output strictly for parser:
    println!("RESULT: {},{},{},{:.2}", mode, thread_count, read_ratio, qps);
    
    let _ = std::fs::remove_dir_all(&path); // Cleanup
}
