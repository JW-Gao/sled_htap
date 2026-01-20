use sled::{Config, Db};
use std::time::{Instant, Duration};
use std::path::Path;
use std::fs;
use serde::Serialize;

#[derive(Serialize)]
struct OpRecord {
    op_index: usize,
    latency_us: u128,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "impact_db";
    let threshold = 10;
    
    // Cleanup
    if Path::new(path).exists() {
        fs::remove_dir_all(path)?;
    }

    let config = Config::new()
        .path(path)
        .page_consolidation_threshold(threshold)
        .cache_capacity(1024 * 1024 * 1024);
        
    let db = config.open()?;

    let mut wtr = csv::Writer::from_path("merge_impact_results.csv")?;
    
    // Key and Value
    let k = 1u32.to_be_bytes();
    
    // Warmup? maybe not needed, capturing the first merge is interesting.
    
    println!("Starting Merge Impact Benchmark (Threshold={})...", threshold);
    
    let num_ops = 200; // Enough to see multiple spikes (should triggers ~20 merges)

    for i in 0..num_ops {
        let mut val = Vec::with_capacity(18);
        val.extend_from_slice(&k);
        val.extend_from_slice(&(i as f32).to_le_bytes()); // varying f helps ensure logic runs
        val.extend_from_slice(b"impacttest");

        let start = Instant::now();
        db.insert(&k, &*val)?;
        // We explicitly flush? No, let pagecache handle it naturally to see when it triggers.
        // Actually, `insert` might just append to log. 
        // Force flush might ensure durability but might mask the internal merge trigger if we are not careful.
        // The consolidation check likely happens on `link` (which insert calls).
        // So just measuring insert duration is correct.
        let duration = start.elapsed();
        
        wtr.serialize(OpRecord {
            op_index: i,
            latency_us: duration.as_micros(),
        })?;
    }
    
    wtr.flush()?;
    println!("Done. Results saved to merge_impact_results.csv");

    Ok(())
}
