use sled::{Config, Db};
use std::env;
use std::fs;
use std::time::Instant;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

fn get_process_write_bytes() -> u64 {
    let status = fs::read_to_string("/proc/self/io").expect("Failed to read /proc/self/io");
    for line in status.lines() {
        if line.starts_with("write_bytes:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse().unwrap_or(0);
            }
        }
    }
    0
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let mode = env::var("TIERED_MODE").unwrap_or_else(|_| "BASELINE".to_string());
    let update_count: usize = env::var("BENCH_OPS").unwrap_or("3000".to_string()).parse()?;
    
    println!("=== 开始 Write Amplification 验证实验 (Mode: {}, Ops: {}) ===", mode, update_count);

    let path = format!("wa_bench_data.sled");
    
    // Clean up previous run to ensure independence
    let _ = fs::remove_dir_all(&path);

    let config = Config::new()
        .path(&path)
        .cache_capacity(1024 * 1024 * 1024) 
        .mode(if mode == "OPTIMIZED" {
            sled::Mode::LowSpace
        } else {
            sled::Mode::HighThroughput
        })
        .page_consolidation_threshold(10); 

    let db = config.open()?;

    println!("--> Pre-filling 100,000 items with RANDOM keys...");
    let value = vec![0u8; 100]; // 100 bytes payload
    
    // Use fixed seed for reproducibility across runs
    let mut rng = StdRng::seed_from_u64(42); 
    
    // Generate random keys to ensure tree structure (not just appending)
    // We use a large unexpected range to scatter them
    for _ in 0..100_000 {
        let k_int: u64 = rng.gen(); 
        let k = k_int.to_be_bytes(); // Use big endian to distribute somewhat if we used monotonic, but here it's random
        db.insert(&k, value.as_slice())?;
    }
    db.flush()?;

    // Settle down
    std::thread::sleep(std::time::Duration::from_secs(2));

    let start_io = get_process_write_bytes();
    let start_time = Instant::now();

    println!("--> Starting {} updates...", update_count);

    // Pick a few hot keys to update repeatedly. 
    // We'll pick a subset of the random keys we inserted, or just new random keys.
    // "Hot key update" usually means updating SAME keys. 
    // Let's pick 100 specific random keys derived from a new seed to be our working set?
    // Or just re-generate the first 100 keys from the same seed?
    let mut rng_keys = StdRng::seed_from_u64(42);
    let mut hot_keys = Vec::new();
    for _ in 0..100 {
        let k_int: u64 = rng_keys.gen();
        hot_keys.push(k_int.to_be_bytes());
    }

    for i in 0..update_count {
        let key = &hot_keys[i % hot_keys.len()];
        let val = format!("val_{}", i); 
        db.insert(key, val.as_bytes())?;
        
        if i % 100 == 0 {
             let _ = db.flush();
        }
    }
    
    // Force final flush
    db.flush()?;

    let end_io = get_process_write_bytes();
    let duration = start_time.elapsed();
    let written = end_io - start_io;
    
    // CSV Output for Analysis
    let csv_file = "wa_results.csv";
    // Check if file is empty or new to write header
    let file_exists = fs::metadata(csv_file).is_ok();
    
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(csv_file)?;
        
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(!file_exists) // Only write header if file didn't exist
        .from_writer(file);

    if !file_exists {
        wtr.write_record(&["mode", "updates", "total_written_bytes"])?;
    }
    
    wtr.write_record(&[
        mode,
        update_count.to_string(),
        written.to_string(),
    ])?;
    wtr.flush()?;
    
    println!("Done. Written: {} bytes", written);
    
    Ok(())
}
