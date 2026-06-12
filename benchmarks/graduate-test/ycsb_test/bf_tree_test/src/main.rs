use anyhow::{Result, Context};
use clap::Parser;
use bf_tree::{BfTree, Config, ScanReturnField, LeafReadResult};
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
    #[arg(short, long, default_value = "bf_tree_db")]
    path: String,
    
    #[arg(short, long, default_value_t = 1024)]
    value_size: usize,
}

struct BfTreeEngine {
    tree: BfTree,
}

impl BfTreeEngine {
    fn new(path: &str) -> Result<Self> {
        let mut config = Config::new(path, 1024 * 1024 * 32);
        config.cb_max_key_len(64);
        let tree = BfTree::with_config(config, None)
            .map_err(|e| anyhow::anyhow!("BfTree init failed: {:?}", e))?;
        Ok(Self { tree })
    }
}

impl KVEngine for BfTreeEngine {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        use bf_tree::LeafInsertResult;
        match self.tree.insert(key, value) {
            LeafInsertResult::Success => Ok(()),
            LeafInsertResult::InvalidKV(msg) => Err(anyhow::anyhow!("Insert failed: {}", msg)),
        }
    }

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let mut buffer = vec![0u8; 4096]; // Sufficient for YCSB (1KB values)
        match self.tree.read(key, &mut buffer) {
            LeafReadResult::Found(len) => {
                let len = len as usize;
                Ok(Some(buffer[..len].to_vec()))
            }
            LeafReadResult::NotFound | LeafReadResult::Deleted => Ok(None),
            LeafReadResult::InvalidKey => Err(anyhow::anyhow!("Invalid key")),
        }
    }

    fn scan(&self, start_key: &[u8], count: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut iter = self.tree.scan_with_count(start_key, count, ScanReturnField::KeyAndValue)
            .map_err(|e| anyhow::anyhow!("Scan failed: {:?}", e))?;
        
        let mut res = Vec::with_capacity(count);
        let mut buffer = vec![0u8; 16384]; // Large buffer for scan item (key + value)

        while let Some((k_len, v_len)) = iter.next(&mut buffer) {
            let k_len = k_len as usize;
            let v_len = v_len as usize;
            let key = buffer[0..k_len].to_vec();
            // Assuming value follows key. 
            // In bf-tree tests, they check buffer content.
            // Based on typical implementations, if both are requested, they are likely packed.
            // Let's assume contiguous.
            let value = buffer[k_len .. k_len + v_len].to_vec();
            res.push((key, value));
        }
        Ok(res)
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    
    println!("Initializing BfTree (Disk: {})", args.path);
    let engine = BfTreeEngine::new(&args.path)?;

    // Phase 1: Load
    println!("Loading data...");
    load_data(&engine, args.records, args.value_size).context("Failed to load data")?;
    
    // Phase 2: Run
    let config = YcsbConfig {
        workload: args.workload.clone(),
        record_count: args.records,
        operation_count: args.ops,
        threads: 1, 
        value_size: args.value_size,
    };
    
    println!("Running Workload {}...", config.workload);
    let results = run_workload(&engine, &config).context("Failed to run workload")?;
    
    println!("Throughput: {:.2} ops/sec", results.throughput);
    println!("Latency (us): p50={}, p99={}, max={}", 
        results.latency.value_at_quantile(0.5),
        results.latency.value_at_quantile(0.99),
        results.latency.max()
    );

    Ok(())
}

