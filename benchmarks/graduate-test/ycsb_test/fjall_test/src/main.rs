use anyhow::Result;
use clap::Parser;
use fjall::{Config, Keyspace, PartitionCreateOptions, PartitionHandle};
use std::sync::Arc;
use ycsb_common::{KVEngine, YcsbConfig, load_data, run_workload};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "a")]
    workload: String,

    #[arg(short, long, default_value_t = 10000)]
    records: usize,

    #[arg(short, long, default_value_t = 10000)]
    ops: usize,
    
    #[arg(short, long, default_value_t = 1024)]
    value_size: usize,
    
    #[arg(long)]
    path: Option<String>,

    #[arg(long, default_value = "128")]
    write_buffer_size_mb: usize,
}

struct FjallEngine {
    keyspace: Keyspace,
    partition: PartitionHandle,
}

impl FjallEngine {
    fn new(path: &str, write_buffer_size_mb: usize) -> Result<Self> {
        let config = Config::new(path).max_write_buffer_size(write_buffer_size_mb as u64 * 1024 * 1024);
        let keyspace = Keyspace::open(config)?;
        let partition = keyspace.open_partition("default", PartitionCreateOptions::default())?;
        Ok(Self { keyspace, partition })
    }
}

impl KVEngine for FjallEngine {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.partition.insert(key, value)?;
        Ok(())
    }

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let val = self.partition.get(key)?;
        Ok(val.map(|v| v.to_vec()))
    }

    fn scan(&self, start_key: &[u8], count: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        // fjall range scan
        // partition.range(start_key..).take(count)
        let mut res = Vec::with_capacity(count);
        for item in self.partition.range(start_key..).take(count) {
            let (k, v) = item?;
            res.push((k.to_vec(), v.to_vec()));
        }
        Ok(res)
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    
    let db_path = args.path.unwrap_or_else(|| "/tmp/fjall_bench_db".to_string());
    
    // Clean up previous run
    let _ = std::fs::remove_dir_all(&db_path);

    println!("Initializing Fjall at {}", db_path);
    let engine = FjallEngine::new(&db_path, args.write_buffer_size_mb)?;

    // Phase 1: Load
    load_data(&engine, args.records, args.value_size)?;
    
    println!("Persisting data to disk...");
    engine.keyspace.persist(fjall::PersistMode::SyncAll)?;
    
    // Phase 2: Run
    let config = YcsbConfig {
        workload: args.workload,
        record_count: args.records,
        operation_count: args.ops,
        threads: 1, // Single threaded for now as per plan
        value_size: args.value_size,
    };
    
    println!("Running Workload {}...", config.workload);
    let results = run_workload(&engine, &config)?;
    
    println!("Throughput: {:.2} ops/sec", results.throughput);
    println!("Latency (us): p50={}, p99={}, max={}", 
        results.latency.value_at_quantile(0.5),
        results.latency.value_at_quantile(0.99),
        results.latency.max()
    );

    Ok(())
}
