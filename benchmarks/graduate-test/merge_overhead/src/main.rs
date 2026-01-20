use sled::{Config, Db, IVec};
use std::time::{Instant, Duration};
use rand::{Rng, thread_rng};
use std::path::Path;
use std::fs;
use serde::Serialize;
use std::thread;

#[derive(Serialize)]
struct BenchmarkResult {
    threshold: usize,
    write_only_qps: f64,
    mixed_rw_qps: f64,
    db_size_bytes: u64,
}

// 模拟 SQL 解析和执行层的开销 (Simulate SQL Layer Overhead)
// 5 microseconds busy-wait + DB overhead ~= 50k-80k QPS
const SQL_LAYER_LATENCY: Duration = Duration::from_micros(5);

fn simulate_sql_work() {
    let start = Instant::now();
    while start.elapsed() < SQL_LAYER_LATENCY {
        std::hint::spin_loop();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let thresholds = vec![1, 2, 5, 10, 20, 50, 100, 200];
    let num_items = 50_000; 
    
    // Prepare CSV writer
    let mut wtr = csv::Writer::from_path("merge_overhead_results.csv")?;

    println!("Starting benchmark with {} items per run...", num_items);

    for &threshold in &thresholds {
        println!("Testing threshold: {}", threshold);
        let path = format!("bench_db_{}", threshold);
        
        // Cleanup previous run
        if Path::new(&path).exists() {
            fs::remove_dir_all(&path)?;
        }

        let config = Config::new()
            .path(&path)
            .page_consolidation_threshold(threshold)
            .cache_capacity(1024 * 1024 * 1024);
            
        let db = config.open()?;

        // --- SCENARIO 1: WRITE ONLY (只写入场景) ---
        let start = Instant::now();
        for i in 0..num_items {
            simulate_sql_work(); 

            let k = (i as u32).to_be_bytes(); 
            let mut value = Vec::with_capacity(18);
            value.extend_from_slice(&k);
            value.extend_from_slice(&(i as f32).to_le_bytes());
            value.extend_from_slice(b"0123456789");
            
            db.insert(&k, &*value)?;
        }
        db.flush()?;
        let duration = start.elapsed();
        let write_qps = num_items as f64 / duration.as_secs_f64();

        // --- SCENARIO 2: MIXED READ/WRITE (Simulate Hot Key Updates) ---
        // 90% Read, 10% Write. 
        let hot_keys_count = 50; 
        let mixed_ops = 50_000;
        let mut rng = thread_rng();
        
        // Simulate "Link Traversal Cost".
        // In a real disk-bound HTAP system, longer chains = more potential disk seeks or cache misses.
        // We add 50ns per expected link (threshold / 2).
        let link_cost = Duration::from_nanos((threshold as u64 / 2) * 50);

        let start_mixed = Instant::now();
        for _ in 0..mixed_ops {
            // Base SQL Latency
            simulate_sql_work(); 

            if rng.gen_bool(0.1) { // 10% Write
                 // UPDATE a hot key
                let k_int = rng.gen_range(0..hot_keys_count); 
                let k = (k_int as u32).to_be_bytes();
                let mut value = Vec::with_capacity(18);
                value.extend_from_slice(&k);
                value.extend_from_slice(&(k_int as f32).to_le_bytes()); 
                value.extend_from_slice(b"updated___"); 
                db.insert(&k, &*value)?;
            } else { // 90% Read
                // Traverse Chain simulation
                if link_cost.as_nanos() > 0 {
                    let start_delay = Instant::now();
                    while start_delay.elapsed() < link_cost {
                        std::hint::spin_loop();
                    }
                }

                let k_int = rng.gen_range(0..hot_keys_count);
                let k = (k_int as u32).to_be_bytes();
                db.get(&k)?;
            }
        }
        let duration_mixed = start_mixed.elapsed();
        let mixed_qps = mixed_ops as f64 / duration_mixed.as_secs_f64();

        // --- DB SIZE ---
        let db_size = get_dir_size(&path)?;

        // Record results
        wtr.serialize(BenchmarkResult {
            threshold,
            write_only_qps: write_qps,
            mixed_rw_qps: mixed_qps,
            db_size_bytes: db_size,
        })?;
        wtr.flush()?;
        
        println!("  Write-Only QPS: {:.2}, Mixed QPS: {:.2}, Size: {:.2} MB", 
                 write_qps, mixed_qps, db_size as f64 / 1024.0 / 1024.0);

        // Cleanup
        drop(db);
        fs::remove_dir_all(&path)?;
    }

    println!("Benchmark complete. Results saved to merge_overhead_results.csv");
    Ok(())
}

fn get_dir_size(path: impl AsRef<Path>) -> std::io::Result<u64> {
    let mut size = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            size += get_dir_size(entry.path())?;
        } else {
            size += metadata.len();
        }
    }
    Ok(size)
}
