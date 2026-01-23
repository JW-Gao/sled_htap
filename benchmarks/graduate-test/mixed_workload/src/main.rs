use clap::{Parser, ValueEnum};
use sled::{Config, Db};
// use std::path::Path;
// use std::time::Duration;
use crate::workload::{run_benchmark, WorkloadConfig, WorkloadMode, WorkloadRatio};

mod schema;
mod workload;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of columns (30 or 70)
    #[arg(long, default_value_t = 30)]
    columns: usize,

    /// Workload Ratio
    #[arg(long, value_enum, default_value_t = Ratio::Balance)]
    ratio: Ratio,

    /// Scan Mode
    #[arg(long, value_enum, default_value_t = Mode::Row)]
    mode: Mode,

    /// Data Selectivity (0.0 - 1.0)
    #[arg(long, default_value_t = 0.1)]
    selectivity: f32,

    /// Total Ops to perform
    #[arg(long, default_value_t = 500_000)]
    total_ops: usize,
    
    /// Pre-populate rows (Warmup)
    #[arg(long, default_value_t = 1_000_000)]
    warmup_rows: usize,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Ratio {
    Read,
    Balance,
    Write,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Mode {
    Row,
    Column,
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    println!("=== Mixed Workload Benchmark ===");
    println!("Columns: {}", args.columns);
    println!("Ratio: {:?}", args.ratio);
    println!("Mode: {:?}", args.mode);
    println!("Selectivity: {}", args.selectivity);
    println!("Total Ops: {}", args.total_ops);
    println!("Warmup Rows: {}", args.warmup_rows);

    let path = format!("mixed_workload_db_{}_{}_{:?}_{:?}", args.columns, args.total_ops, args.ratio, args.mode);
    let _ = std::fs::remove_dir_all(&path);
    
    let config = Config::new()
        .path(&path)
        .cache_capacity(1024 * 1024 * 1024) // 1GB Cache
        .mode(sled::Mode::HighThroughput);
        
    let db: Db = config.open().expect("failed to open dictioanry");

    let wl_config = WorkloadConfig {
        num_columns: args.columns,
        ratio: match args.ratio {
            Ratio::Read => WorkloadRatio::ReadHeavy,
            Ratio::Balance => WorkloadRatio::Balance,
            Ratio::Write => WorkloadRatio::WriteHeavy,
        },
        mode: match args.mode {
            Mode::Row => WorkloadMode::Row,
            Mode::Column => WorkloadMode::Column,
        },
        selectivity: args.selectivity,
        total_ops: args.total_ops,
        warmup_rows: args.warmup_rows,
    };

    let duration = run_benchmark(&db, wl_config);
    
    println!("{{");
    println!("  \"scenario\": \"Cols{}-{:?}-Sel{:.1}\",", args.columns, args.ratio, args.selectivity);
    println!("  \"method\": \"{:?}\",", args.mode);
    println!("  \"total_ops\": {},", args.total_ops);
    println!("  \"duration_sec\": {:.4}", duration.as_secs_f64());
    println!("}}");
    
    // Cleanup
    let _ = std::fs::remove_dir_all(&path);
}
