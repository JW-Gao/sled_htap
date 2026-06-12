use anyhow::Result;
use clap::Parser;
use sled::Db;
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
    
    #[arg(long)]
    path: Option<String>,
}

struct SledEngine {
    db: Db,
}

impl SledEngine {
    fn new(path: &str) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }
}

impl KVEngine for SledEngine {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.db.insert(key, value)?;
        Ok(())
    }

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let val = self.db.get(key)?;
        Ok(val.map(|v| v.to_vec()))
    }

    fn scan(&self, start_key: &[u8], count: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut res = Vec::with_capacity(count);
        for item in self.db.range(start_key..).take(count) {
             let (k, v) = item?;
             res.push((k.to_vec(), v.to_vec()));
        }
        Ok(res)
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    
    let db_path = args.path.unwrap_or_else(|| "/tmp/blp_bench_db".to_string());
    let _ = std::fs::remove_dir_all(&db_path);

    println!("Initializing blp-tree (sled) at {}", db_path);
    let engine = SledEngine::new(&db_path)?;

    // Phase 1: Load
    load_data(&engine, args.records, 100)?;
    
    // Phase 2: Run
    let config = YcsbConfig {
        workload: args.workload,
        record_count: args.records,
        operation_count: args.ops,
        threads: 1, 
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
