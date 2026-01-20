use sled::{Config, Db};
use std::time::{Instant, Duration};
use hdrhistogram::Histogram;
use std::path::Path;
use std::fs;

// 实验配置 (Experiment Config)
const BASE_ROW_COUNT: usize = 100_000; // 基准数据行数 (模拟大节点)
// const UPDATE_COUNT: usize = 30_000; // Moved to runtime config
const THRESHOLD: usize = 10;           // 触发合并的阈值

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    // 从环境变量读取模式: "BASELINE" or "OPTIMIZED"
    let update_count: usize = std::env::var("BENCH_OPS")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .expect("BENCH_OPS must be a number");
        
    let mode = std::env::var("TIERED_MODE").unwrap_or_else(|_| "BASELINE".to_string());
    println!("=== 开始 Tiered Merge 验证实验 (Mode: {}, Ops: {}) ===", mode, update_count);
    
    let path = "tiered_validation_db";
    // 清理旧数据
    if Path::new(path).exists() {
        fs::remove_dir_all(path)?;
    }

    let config = Config::new()
        .path(path)
        .page_consolidation_threshold(THRESHOLD)
        .cache_capacity(1024 * 1024 * 1024); // 足够大的缓存，排除Cache Miss干扰
        
    let db = config.open()?;

    println!("1. 正在构建基准数据 (Base Node, {} rows)...", BASE_ROW_COUNT);
    // 我们构建一个大的 Node，让它的物理大小达到 MB 级别
    // 通过批量写入 + Flush 尽可能让它们落在一个 Page 里 (取决于 Split 逻辑)
    // Sled 默认 Split 是 16KB ~ 32KB。
    // 为了模拟 "大节点合并"，我们需要让 Sled 的 Split 阈值很大，或者我们在逻辑模拟 "重写成本"。
    // *注意*: 在真实 Sled 中，Node 大小受限于 Segment Size。
    // 这里我们主要验证的是 "算法逻辑"：即 merge_overlay 是否重写了数据。
    
    let key = 1u32.to_be_bytes(); // 只要更新这一个 Key 就能触发该 Key 所在 Node 的合并
    
    // 填充数据
    for i in 0..BASE_ROW_COUNT {
         let k = (i as u32).to_be_bytes();
         let v = vec![0u8; 14]; // 保证 Schema 18 bytes (4+14? No, 4+4+10=18)
         // 其实只要 Value 长度对就行，内容随意
         db.insert(&k, &*v)?;
    }
    // 确保落盘并形成 Columnar Data
    // 提示: 这里依赖于 Sled 内部的 Split/Merge 机制是否已经把它们变成了 Columnar。
    // 我们之前的修改是 Inner::new 会尝试变为 Columnar。
    db.flush()?; 
    
    println!("Base Data构建完成。开始热点更新测试...");
    println!("目标: 更新 Key=1, 重复 {} 次", update_count);

    // Open CSV writer in append mode
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("tiered_results.csv")?;
    
    // Write header only if file is empty
    if file.metadata()?.len() == 0 {
        let mut wtr = csv::WriterBuilder::new().has_headers(false).from_writer(&file);
        wtr.write_record(&["op_index", "latency_us", "mode"])?;
        wtr.flush()?;
    }
    
    let mut wtr = csv::WriterBuilder::new().has_headers(false).from_writer(file);

    let mut hist = Histogram::<u64>::new_with_bounds(1, 1000_000, 3).unwrap();
    let bench_start = Instant::now();
    let mut max_latency = Duration::from_micros(0);

    for i in 0..update_count {
        let val = vec![0u8; 14]; // New value

        let op_start = Instant::now();
        db.insert(&key, &*val)?;
        let latency = op_start.elapsed();
        
        // Record to Histogram
        hist.record(latency.as_micros() as u64).unwrap();
        if latency > max_latency {
            max_latency = latency;
        }

        // Write to CSV (Sample every op? Or just keep it all? 50k is small enough)
        wtr.write_record(&[
            i.to_string(),
            latency.as_micros().to_string(),
            mode.clone()
        ])?;
        
        if i % 1000 == 0 {
             print!(".");
             use std::io::Write;
             std::io::stdout().flush()?;
        }
    }
    println!("\n");
    
    let total_duration = bench_start.elapsed();
    
    println!("=== 实验结果 ({}) ===", mode);
    println!("Total Ops: {}", update_count);
    println!("Total Time: {:?}", total_duration);
    println!("Avg Latency: {:.2} µs", hist.mean());
    println!("P99 Latency: {} µs", hist.value_at_quantile(0.99));
    println!("Max Latency: {:?} (重点关注指标)", max_latency);
    println!("IO Written: (请参考 OS 监控或 Sled Metrics)");

    Ok(())
}
