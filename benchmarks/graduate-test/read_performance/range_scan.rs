use sled::{Config, Db};
use std::thread;
use std::time::{Duration, Instant};
use std::fs::OpenOptions;
use std::io::Write;

const START_KEY: usize = 500_000;
const END_KEY: usize = 501_000; // 1000 keys
const UPDATE_ITERS: usize = 20;

fn main() {
    let path = "bench_range_scan.db";
    let _ = std::fs::remove_dir_all(path);

    let config = Config::default()
        .path(path)
        .cache_capacity(1024 * 1024 * 512);

    let db = config.open().unwrap();

    // 1. Initial Fill
    println!("Filling initial data (1M items)...");
    for i in 0..1_000_000 {
        if i % 10000 == 0 { if i % 100000 == 0 { println!("Filled {}", i); } }
        let k = format!("{:016}", i);
        let v = format!("val_{:08}", i);
        db.insert(k.as_bytes(), v.as_bytes()).unwrap();
    }
    db.flush().unwrap();

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("range_scan_results.txt")
        .unwrap();
    
    writeln!(file, "Iteration,WriteLatency_ms,ScanLatency_ms").unwrap();

    println!("Starting Fragmentation + Scan Loop...");
    
    let start_key = format!("{:016}", START_KEY);
    let end_key = format!("{:016}", END_KEY);

    for iter in 0..100 {
        // Step A: Fragment the range (Write Heavy)
        let t0 = Instant::now();
        for _ in 0..UPDATE_ITERS {
            for i in START_KEY..END_KEY {
                let k = format!("{:016}", i);
                let v = format!("updated_{}_{}", iter, i);
                db.insert(k.as_bytes(), v.as_bytes()).unwrap();
            }
        }
        let write_lat = t0.elapsed().as_millis();

        // Step B: Scan the range (Read)
        let t1 = Instant::now();
        let mut count = 0;
        let mut iter_scan = db.range(start_key.as_bytes()..end_key.as_bytes());
        while let Some(res) = iter_scan.next() {
            let _ = res.unwrap();
            count += 1;
        }
        let scan_lat = t1.elapsed().as_millis();

        println!("Iter {}: Writes took {}ms, Scan of {} items took {}ms", iter, write_lat, count, scan_lat);
        writeln!(file, "{},{},{}", iter, write_lat, scan_lat).unwrap();
        
        // Sleep to let background threads work slightly?
        thread::sleep(Duration::from_millis(100));
    }

    println!("Range Scan Benchmark Complete. Results saved to range_scan_results.txt");
}
