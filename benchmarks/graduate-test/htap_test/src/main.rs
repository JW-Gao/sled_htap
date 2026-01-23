mod schema;
mod olap_queries;
mod workload;

use clap::Parser;
use schema::TableSchema;
use workload::{WorkloadGenerator, Operation};
use olap_queries::execute_query;
use std::time::Instant;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "htap_test")]
#[command(about = "HTAP Mixed Workload Benchmark", long_about = None)]
struct Args {
    /// Number of columns in the table (30 for narrow, 70 for wide)
    #[arg(long, default_value_t = 30)]
    num_columns: usize,

    /// OLAP operation ratio (0.0 to 1.0)
    #[arg(long, default_value_t = 0.5)]
    olap_ratio: f64,

    /// OLTP operation ratio (0.0 to 1.0)
    #[arg(long, default_value_t = 0.5)]
    oltp_ratio: f64,

    /// Data access ratio for AP queries (0.0 to 1.0)
    #[arg(long, default_value_t = 0.5)]
    data_access_ratio: f64,

    /// Total number of operations to execute
    #[arg(long, default_value_t = 50000)]
    total_ops: usize,

    /// Number of rows to pre-populate
    #[arg(long, default_value_t = 100000)]
    prepopulate_rows: u64,

    /// Database mode: baseline or optimized
    #[arg(long, default_value = "baseline")]
    mode: String,

    /// Database path (optional, will use temp if not specified)
    #[arg(long)]
    db_path: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let separator = "=".repeat(80);
    println!("{}", separator);
    println!("HTAP Mixed Workload Benchmark");
    println!("{}", separator);
    println!("Configuration:");
    println!("  Columns: {}", args.num_columns);
    println!("  OLAP Ratio: {:.1}%", args.olap_ratio * 100.0);
    println!("  OLTP Ratio: {:.1}%", args.oltp_ratio * 100.0);
    println!("  Data Access Ratio: {:.1}%", args.data_access_ratio * 100.0);
    println!("  Total Operations: {}", args.total_ops);
    println!("  Pre-populate Rows: {}", args.prepopulate_rows);
    println!("  Mode: {}", args.mode);
    println!("{}", separator);

    // Create table schema
    let schema = TableSchema::new(args.num_columns);

    // Determine database path
    let db_path = if let Some(path) = args.db_path {
        PathBuf::from(path)
    } else {
        let table_type = if args.num_columns == 30 { "narrow" } else { "wide" };
        PathBuf::from(format!(
            "htap_test_db_{}_{}",
            table_type,
            args.mode
        ))
    };

    println!("Opening database: {:?}", db_path);
    
    // Open database
    let db = sled::open(&db_path)?;

    // Pre-populate database
    println!("Pre-populating {} rows...", args.prepopulate_rows);
    let prepopulate_start = Instant::now();
    
    for i in 0..args.prepopulate_rows {
        let row = schema.generate_row(i);
        let key = i.to_be_bytes(); // Use big-endian for sortable keys
        db.insert(key, row)?;
        
        if (i + 1) % 10000 == 0 {
            print!("\r  Progress: {}/{}", i + 1, args.prepopulate_rows);
            use std::io::Write;
            std::io::stdout().flush()?;
        }
    }
    
    db.flush()?;
    let prepopulate_duration = prepopulate_start.elapsed();
    println!("\r  Pre-population completed in {:.2}s", prepopulate_duration.as_secs_f64());

    // Generate workload
    println!("Generating workload...");
    let workload = WorkloadGenerator::new(
        args.total_ops,
        args.olap_ratio,
        args.oltp_ratio,
        args.data_access_ratio,
        args.prepopulate_rows,
    );
    println!("  Generated {} operations", workload.len());

    // Execute workload
    println!("Executing workload...");
    let execution_start = Instant::now();
    
    let mut insert_count = 0;
    let mut query_count = 0;
    
    for (idx, operation) in workload.operations.iter().enumerate() {
        match operation {
            Operation::Insert(pk) => {
                let row = schema.generate_row(*pk);
                let key = pk.to_be_bytes();
                db.insert(key, row)?;
                insert_count += 1;
            }
            Operation::Query(query_type, theta) => {
                // Use column 0 as filter column for Q2 and Q3
                execute_query(&db, &schema, *query_type, *theta, 0);
                query_count += 1;
            }
        }
        
        if (idx + 1) % 5000 == 0 {
            print!("\r  Progress: {}/{}", idx + 1, workload.len());
            use std::io::Write;
            std::io::stdout().flush()?;
        }
    }
    
    db.flush()?;
    let execution_duration = execution_start.elapsed();
    
    println!("\r  Execution completed!");
    println!();
    println!("{}", separator);
    println!("Results:");
    println!("  Total Operations: {}", workload.len());
    println!("  OLTP Inserts: {}", insert_count);
    println!("  OLAP Queries: {}", query_count);
    println!("  Execution Time: {:.3} seconds", execution_duration.as_secs_f64());
    println!("  Throughput: {:.2} ops/sec", workload.len() as f64 / execution_duration.as_secs_f64());
    println!("{}", separator);

    // Clean up (optional - comment out if you want to keep the database)
    // drop(db);
    // std::fs::remove_dir_all(&db_path)?;

    Ok(())
}
