use crate::generator::KeyGenerator;
use crate::KVEngine;
use rand::{rngs::StdRng, SeedableRng, Rng};
use std::time::Instant;
use hdrhistogram::Histogram;

pub enum Operation {
    Read(u64),
    Update(u64, Vec<u8>),
    Insert(u64, Vec<u8>),
    Scan(u64, usize),
}

pub struct YcsbConfig {
    pub workload: String, // "a", "b", "e"
    pub record_count: usize,
    pub operation_count: usize,
    pub threads: usize,
    pub value_size: usize,
}

pub struct Results {
    pub throughput: f64,
    pub latency: Histogram<u64>,
}

pub fn load_data(engine: &impl KVEngine, record_count: usize, value_size: usize) -> anyhow::Result<()> {
    println!("Loading {} records with value size {} bytes...", record_count, value_size);
    for i in 0..record_count {
        let key_bytes = format!("user{:019}", i).into_bytes();
        let value_bytes = vec![0u8; value_size];
        engine.put(&key_bytes, &value_bytes)?;
        if i % 100000 == 0 && i > 0 {
             println!("Loaded {} records", i);
        }
    }
    println!("Load complete.");
    Ok(())
}

pub fn run_workload(engine: &impl KVEngine, config: &YcsbConfig) -> anyhow::Result<Results> {
    let mut rng = StdRng::seed_from_u64(42);
    let mut key_gen = match config.workload.as_str() {
        "a" | "b" => KeyGenerator::new_zipf(config.record_count, 0.99)?, // standard YCSB skew
        "e" => KeyGenerator::new_zipf(config.record_count, 0.99)?, 
        _ => return Err(anyhow::anyhow!("Unknown workload")),
    };
    
    // For inserts in E, we need sequential or similar. 
    // YCSB "insert order" property. Assuming standard YCSB Core logic which picks keys.
    // Simplifying: 
    // A: 50/50 R/U
    // B: 95/5 R/U
    // E: 95/5 Scan/Insert
    
    let mut latencies = Histogram::<u64>::new(3).unwrap();
    let start = Instant::now();

    for _ in 0..config.operation_count {
        let op_choice: f64 = rng.gen();
        let key_id = key_gen.next(&mut rng) as u64;
        let key = format!("user{:019}", key_id).into_bytes();
        
        let op_start = Instant::now();
        match config.workload.as_str() {
            "a" => {
                if op_choice < 0.5 {
                    engine.get(&key)?;
                } else {
                     engine.put(&key, &vec![0u8; config.value_size])?; // Update
                }
            },
            "b" => {
                if op_choice < 0.95 {
                    engine.get(&key)?;
                } else {
                    engine.put(&key, &vec![0u8; config.value_size])?; // Update
                }
            },
            "e" => {
                if op_choice < 0.95 {
                     // Scan
                     // Range length uniform [1, 100]
                     let len_dist = rand::distributions::Uniform::new_inclusive(1, 100);
                     let len = crate::generator::KeyGenerator::Uniform(len_dist).next(&mut rng);
                     engine.scan(&key, len)?;
                } else {
                    // Insert
                    let new_key_id = config.record_count + rng.gen::<usize>() % 1000000;
                     let new_key = format!("user{:019}", new_key_id).into_bytes();
                    engine.put(&new_key, &vec![0u8; config.value_size])?;
                }
            },
            _ => unreachable!(),
        }

        let duration = op_start.elapsed().as_micros() as u64;
        latencies.record(duration)?;
    }
    
    let total_time = start.elapsed();
    let throughput = config.operation_count as f64 / total_time.as_secs_f64();
    
    Ok(Results {
        throughput,
        latency: latencies,
    })
}
