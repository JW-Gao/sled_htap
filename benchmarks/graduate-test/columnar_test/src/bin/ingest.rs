use sled::{Db, Config};
use sled::schema::{TableSchema, DataType};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    // Path to TBL
    let tbl_path = "../测试用的sql语句/lineitem.tbl";
    let db_path = "lineitem_db";
    
    // Clean up old DB
    let _ = std::fs::remove_dir_all(db_path);
    
    let config = Config::new()
        .path(db_path)
        .cache_capacity(1024 * 1024 * 1024) // 1GB cache
        .mode(sled::Mode::HighThroughput);
        
    let db = config.open()?;
    
    println!("Ingesting from {}", tbl_path);
    
    // Define Schema for Parsing
    // Define Schema for Parsing (Full LineItem Schema)
    let schema = TableSchema::new(
        "lineitem",
        vec![
            ("orderkey", DataType::I32), // 0
            ("partkey", DataType::I32),
            ("suppkey", DataType::I32),
            ("linenumber", DataType::I32), // 3
            ("quantity", DataType::F32), // 4
            ("extendedprice", DataType::F32), // 5
            ("discount", DataType::F32), // 6
            ("tax", DataType::F32),
            ("returnflag", DataType::Bytes(1)), // 8
            ("linestatus", DataType::Bytes(1)), // 9
            ("shipdate", DataType::Date), // 10
            ("commitdate", DataType::Date),
            ("receiptdate", DataType::Date),
            ("shipinstruct", DataType::Bytes(25)), // 13
            ("shipmode", DataType::Bytes(10)),
            ("comment", DataType::Bytes(44)), // 15
        ],
        vec![4, 5, 6, 10], // Columnar cols (Only relevant ones need to be marked columnar for transposition optimization, technically L2 Merger checks this list to pick what to columnarize? Actually L2 merger re-uses this schema instance? L2 merger creates its own schema instance in code currently. This list is for identification.)
        // Ingest Projection: Put RELEVANT cols FIRST (0-3), then JUNK cols (4-15).
        // Original Indices: 4, 5, 6, 10 are Q6 Relevant.
        // Others: 0, 1, 2, 3, 7, 8, 9, 11, 12, 13, 14, 15.
        vec![
            4, 5, 6, 10,                 // Relevant (Indices 0, 1, 2, 3 in stored row)
            0, 1, 2, 3, 7, 8, 9, 11, 12, 13, 14, 15 // Junk
        ], 
    );
    // Note: TableSchema.new args: name, columns, columnar_cols, ingest_projection.
    
    // Open File
    let file = File::open(tbl_path)?;
    let reader = BufReader::new(file);
    
    let start = Instant::now();
    let mut count = 0;
    
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        
        // Parse Value
        let value_bytes = schema.parse_row(&line);
        
        // Parse Key: OrderKey(0) + LineNumber(3).
        // Manual parse because schema.parse_row only returns Value Projected.
        let parts: Vec<&str> = line.split('|').collect();
        let orderkey: i32 = parts[0].parse().unwrap_or(0);
        let linenumber: i32 = parts[3].parse().unwrap_or(0);
        
        // Composite Key: [orderkey as BE bytes] + [linenumber as BE bytes]
        // BE to preserve ordering.
        let mut key = Vec::with_capacity(8);
        key.extend_from_slice(&orderkey.to_be_bytes());
        key.extend_from_slice(&linenumber.to_be_bytes());
        
        db.insert(&key, value_bytes)?;
        
        count += 1;
        if count % 10000 == 0 {
            print!("\rIngested {} rows...", count);
        }
    }
    
    println!("\nIngested {} rows in {:.2?} s", count, start.elapsed());
    
    // Verify
    let ScanCount = db.iter().count();
    println!("DB verified count: {}", ScanCount);
    
    // Explicitly flush?
    db.flush()?;
    
    // Keep DB open for a bit to allow background merges?
    // Or close it.
    // L2 Merges trigger on access or background thread.
    // If we want to test Columnar Read, we need data in Base Log (L2).
    // Newly inserted data is in Memtable or L0.
    // We need to force compaction or simulate it.
    // Sled doesn't expose `compact` easily.
    // BUT we can just run the test. If it merges, great.
    // If not, we are testing Row Fallback (which works!).
    // To PROVE Columnar works, we need to inspect logs or metrics?
    // Or benchmarks speed.
    // For "Verification Plan", just running Q6 correctly is step 1.
    
    // ----------------------------------------------------------------
    // Performance Comparison: Row-Scan vs. Column-Scan (Q6)
    // ----------------------------------------------------------------
    
    println!("\n=== 执行延迟对比 (Latency Comparison) ===");
    
    let date_min = 19940101.0;
    let date_max = 19950101.0;
    let disc_min = 0.05;
    let disc_max = 0.07;
    let qty_max = 24.0;

    // 1. Baseline: Row-based Scan (simulate traditional iterator)
    // iterate db.iter(), parsing all 4 columns for every row.
    let start_row = Instant::now();
    let mut revenue_row = 0.0;
    let mut count_row = 0;
    
    for item in db.iter() {
        if let Ok((_k, v)) = item {
            // Value is [qty(4), price(4), disc(4), date(4)] = 16 bytes
            if v.len() >= 16 {
                // Manual parse using slice
                let val_slice = &v;
                let q_bytes: [u8;4] = val_slice[0..4].try_into().unwrap();
                let p_bytes: [u8;4] = val_slice[4..8].try_into().unwrap();
                let d_bytes: [u8;4] = val_slice[8..12].try_into().unwrap();
                let date_bytes: [u8;4] = val_slice[12..16].try_into().unwrap();
                
                let qty = f32::from_le_bytes(q_bytes);
                let price = f32::from_le_bytes(p_bytes);
                let disc = f32::from_le_bytes(d_bytes);
                let date = f32::from_le_bytes(date_bytes);
                
                if date >= date_min && date < date_max 
                   && disc >= disc_min && disc <= disc_max 
                   && qty < qty_max {
                       revenue_row += price * disc;
                }
                count_row += 1;
            }
        }
    }
    let dur_row = start_row.elapsed();
    println!("1. 行式扫描 (Row Scan) - Full Q6:");
    println!("   Revenue: {:.2}", revenue_row);
    println!("   Rows: {}", count_row);
    println!("   Time: {:.2?}", dur_row);


    // 2. New: Column-based Scan (Zip 4 iterators)
    // Note: If data is not transposed to L2, this uses 'Row Fallback' inside scan_column.
    // Overhead of 4x iterators might make it slower if not Columnar.
    let start_col = Instant::now();
    let mut revenue_col = 0.0;
    let mut count_col = 0;

    let iter_qty = db.scan_column(0);
    let iter_price = db.scan_column(1);
    let iter_disc = db.scan_column(2);
    let iter_date = db.scan_column(3);
    
    for (((qty, price), disc), date) in iter_qty.zip(iter_price).zip(iter_disc).zip(iter_date) {
         if date >= date_min && date < date_max 
            && disc >= disc_min && disc <= disc_max 
            && qty < qty_max {
                revenue_col += price * disc;
         }
         count_col += 1;
    }
    let dur_col = start_col.elapsed();
    println!("2. 列式扫描 (Column Scan) - Full Q6:");
    println!("   Revenue: {:.2}", revenue_col);
    println!("   Rows: {}", count_col);
    println!("   Time: {:.2?}", dur_col);

    // 3. Single Column Filter (Where Columnar shines most)
    // Scan ONLY Date column.
    let start_single = Instant::now();
    let mut count_single = 0;
    for date in db.scan_column(3) {
        if date >= date_min && date < date_max {
            count_single += 1;
        }
    }
    let dur_single = start_single.elapsed();
    println!("3. 单列扫描 (Single Column Scan) - Filter Date:");
    println!("   Matches: {}", count_single);
    println!("   Time: {:.2?}", dur_single);
    
    // Comparison
    println!("\n=== 对比结果 (Comparison) ===");
    println!("Row Scan Time: {:.2?}", dur_row);
    println!("Col Scan Time: {:.2?}", dur_col);
    if dur_col < dur_row {
        println!("Outcome: Column Scan is {:.2}x faster!", dur_row.as_secs_f64() / dur_col.as_secs_f64());
    } else {
        println!("Outcome: Column Scan is slower (likely due to iterator overhead on Row Data).");
        println!("Note: Columnar benefits require L2 Transposition (Flush/Compact).");
        println!("      Single Column Scan shows potential: {:.2?}", dur_single);
    }
    
    Ok(())
}
