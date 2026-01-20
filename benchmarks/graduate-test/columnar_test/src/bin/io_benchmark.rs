use std::fs::File;
use std::io::{Write, Read, BufReader, Seek, SeekFrom};
use std::time::Instant;
use std::hint::black_box;

// Simulation parameters
const ROW_COUNT: usize = 10_000_000; // 10 Million Rows (~1.2GB)
const ROW_SIZE: usize = 120; // 4 columns * 4 bytes + ~100 bytes junk
const COL_SIZE: usize = 4;   // We only care about 1 column (e.g., Date) roughly

fn main() -> std::io::Result<()> {
    println!("=== Sled Columnar IO Benchmark Simulation ===");
    println!("Generating dataset: {} rows...", ROW_COUNT);
    println!("Row Format Size: {} MB", (ROW_COUNT * ROW_SIZE) / 1024 / 1024);
    println!("Col Format Size: {} MB (Relevant Data)", (ROW_COUNT * COL_SIZE) / 1024 / 1024);

    let row_path = "/tmp/sled_bench_row.dat";
    let col_path = "/tmp/sled_bench_col.dat";

    // 1. Prepare Data
    // We fill random bytes, but keep the structure valid enough if needed.
    // Actually we just care about IO volume here.
    let row_data = vec![0u8; ROW_COUNT * ROW_SIZE];
    // Relevant column is just 4 bytes per row
    let col_data = vec![0u8; ROW_COUNT * COL_SIZE];

    // 2. Write Files (Simulate Storage)
    {
        print!("Writing Row Data to disk...");
        let mut f = File::create(row_path)?;
        f.write_all(&row_data)?;
        f.sync_all()?;
        println!(" Done.");

        print!("Writing Col Data to disk...");
        let mut f = File::create(col_path)?;
        f.write_all(&col_data)?;
        f.sync_all()?;
        println!(" Done.");
    }

    // 3. Drop Cache Hint (Best Effort)
    // We can't sudo, so this is "Warm Cache" or "OS Page Cache" speed.
    // However, memory copy of 120MB vs 4MB is still significant.
    println!("\n=== Benchmark Start (Simulating Cold/Warm Read) ===");

    // --- ROW SCAN ---
    // Simulate: Read Full Row, Extract 4 Bytes, Parse
    let start_row = Instant::now();
    {
        let mut file = File::open(row_path)?;
        let mut reader = BufReader::with_capacity(8 * 1024, file); // 8KB buffer
        let mut buffer = vec![0u8; ROW_SIZE];
        let mut sum = 0.0;
        
        for _ in 0..ROW_COUNT {
            reader.read_exact(&mut buffer)?; // Force IO of full row
            // Extract "Date" at offset 12 (fake offset)
            let val_bytes = &buffer[12..16];
            let val = f32::from_le_bytes(val_bytes.try_into().unwrap());
            sum += val;
        }
        black_box(sum);
    }
    let dur_row = start_row.elapsed();
    println!("Row Scan Time: {:.4}s", dur_row.as_secs_f64());
    println!("  Throughput: {:.2} MB/s", (ROW_COUNT * ROW_SIZE) as f64 / 1024.0 / 1024.0 / dur_row.as_secs_f64());

    // --- COL SCAN ---
    // Simulate: Read Chunk of Column, Parse
    // In Sled, we would read a Page of Column Data (e.g., 4KB chunk).
    // Here we read the continuous column file.
    let start_col = Instant::now();
    {
        let mut file = File::open(col_path)?;
        let mut reader = BufReader::with_capacity(8 * 1024, file);
        let mut buffer = vec![0u8; COL_SIZE]; // Read 4 bytes at a time? 
        // Real columnar engine reads vectors. Let's read 1024 values at a time (Vectorized IO).
        // 1024 * 4 = 4096 bytes (1 Page)
        let batch_size = 1024;
        let mut batch_buf = vec![0u8; batch_size * COL_SIZE];
        let mut sum = 0.0;

        let mut remain = ROW_COUNT;
        while remain > 0 {
            let current_batch = std::cmp::min(remain, batch_size);
            let bytes_to_read = current_batch * COL_SIZE;
            reader.read_exact(&mut batch_buf[..bytes_to_read])?;
            
            // Vectorized Processing
            for i in 0..current_batch {
                let start = i * 4;
                let val_bytes = &batch_buf[start..start+4];
                let val = f32::from_le_bytes(val_bytes.try_into().unwrap());
                sum += val;
            }
            remain -= current_batch;
        }
        black_box(sum);
    }
    let dur_col = start_col.elapsed();
    println!("Col Scan Time: {:.4}s", dur_col.as_secs_f64());
    println!("  Throughput: {:.2} MB/s (Effective Data)", (ROW_COUNT * ROW_SIZE) as f64 / 1024.0 / 1024.0 / dur_col.as_secs_f64());

    // --- REPORT ---
    println!("\n=== Results ===");
    println!("Speedup: {:.2}x", dur_row.as_secs_f64() / dur_col.as_secs_f64());
    println!("IO Vol Reduction: {:.2}%", 100.0 * (1.0 - (COL_SIZE as f64 / ROW_SIZE as f64)));
    
    // Cleanup
    let _ = std::fs::remove_file(row_path);
    let _ = std::fs::remove_file(col_path);
    
    Ok(())
}
