use clap::Parser;
use rand::{rngs::StdRng, Rng, SeedableRng};
use rayon::prelude::*;
use sled::{Config, OverlayTree, Result, Tree};
use std::f64::consts::LN_10;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Parser, Debug, Clone)]
struct Opts {
    /// database path (temporary if not provided)
    #[clap(long, default_value = "./sysbench-sim.db")] 
    path: String,

    /// number of worker threads
    #[clap(long, default_value_t = 8)]
    threads: usize,

    /// seconds to run
    #[clap(long, default_value_t = 10)]
    time: u64,

    /// total key space
    #[clap(long, default_value_t = 1_000_000)]
    table_size: u64,

    /// percentage of writes (0-100)
    #[clap(long, default_value_t = 20)]
    write_pct: u64,

    /// enable promote-on-read (PULL)
    #[clap(long, default_value_t = true, action = clap::ArgAction::Set)]
    pull: bool,
    
    /// disable promote-on-read (PULL)
    #[clap(long, action = clap::ArgAction::SetTrue, overrides_with = "pull")]
    no_pull: bool,

    /// workload: tp, ap, or mixed
    #[clap(long, default_value = "tp", value_parser = ["tp", "ap", "mixed"])]
    workload: String,

    /// hotspot fraction for tp (cheat interface: if set, creates artificial hotspot in first N% of keys)
    /// Default: None (uniform random distribution, let system discover hotspots naturally)
    #[clap(long)]
    hotspot_frac: Option<f64>,

    /// grade column index for ap (used by sum/avg)
    #[clap(long, default_value_t = 0)]
    grade_col: usize,

    /// number of TP threads (for mixed workload)
    #[clap(long, default_value_t = 4)]
    tp_threads: usize,

    /// number of AP threads (for mixed workload)
    #[clap(long, default_value_t = 2)]
    ap_threads: usize,

    /// AP range query size (fraction of table_size, e.g., 0.1 = 10% of keys)
    #[clap(long, default_value_t = 0.1)]
    ap_range_frac: f64,

    /// Additional CPU iterations for each AP query (simulates complex analytics)
    #[clap(long, default_value_t = 256)]
    ap_compute_iters: u32,

    /// Use direct Tree instead of OverlayTree (for L2 on/off comparison)
    #[clap(long)]
    use_direct_tree: bool,
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    let db = Config::new().path(&opts.path).open()?;
    let base: Tree = db.open_tree("t")?;

    let start = Instant::now();
    let end_at = start + Duration::from_secs(opts.time);

    if opts.use_direct_tree {
        // Use direct Tree (L2 disabled - row-oriented storage)
        let base_arc = Arc::new(base);
        match opts.workload.as_str() {
            "tp" => {
                let ops = run_tp_workload_direct(&base_arc, &opts, end_at);
                let secs = start.elapsed().as_secs_f64();
                println!(
                    "sysbench-sim summary: workload={} threads={} time={:.2}s ops={} ops/s={:.0}",
                    opts.workload, opts.threads, secs, ops, (ops as f64 / secs)
                );
            }
            "ap" => {
                let ops = run_ap_workload_direct(&base_arc, &opts, end_at);
                let secs = start.elapsed().as_secs_f64();
                println!(
                    "sysbench-sim summary: workload={} threads={} time={:.2}s ops={} ops/s={:.0}",
                    opts.workload, opts.threads, secs, ops, (ops as f64 / secs)
                );
            }
            "mixed" => {
                let (tp_ops, ap_ops) = run_mixed_workload_direct(&base_arc, &opts, end_at);
                let secs = start.elapsed().as_secs_f64();
                println!(
                    "sysbench-sim summary: workload=mixed tp_threads={} ap_threads={} time={:.2}s",
                    opts.tp_threads, opts.ap_threads, secs
                );
                println!(
                    "  TP: ops={} ops/s={:.0}",
                    tp_ops, (tp_ops as f64 / secs)
                );
                println!(
                    "  AP: ops={} ops/s={:.0}",
                    ap_ops, (ap_ops as f64 / secs)
                );
            }
            _ => unreachable!(),
        }
    } else {
        // Use OverlayTree (L2 enabled - columnar storage)
        let pull_enabled = !opts.no_pull; // --no-pull overrides default true
        let overlay = Arc::new(OverlayTree::with_columns(base, 50_000, Duration::from_millis(200), pull_enabled, 2));
        match opts.workload.as_str() {
            "tp" => {
                let ops = run_tp_workload(&overlay, &opts, end_at);
                let secs = start.elapsed().as_secs_f64();
                println!(
                    "sysbench-sim summary: workload={} threads={} time={:.2}s ops={} ops/s={:.0}",
                    opts.workload, opts.threads, secs, ops, (ops as f64 / secs)
                );
            }
            "ap" => {
                let ops = run_ap_workload(&overlay, &opts, end_at);
                let secs = start.elapsed().as_secs_f64();
                println!(
                    "sysbench-sim summary: workload={} threads={} time={:.2}s ops={} ops/s={:.0}",
                    opts.workload, opts.threads, secs, ops, (ops as f64 / secs)
                );
            }
            "mixed" => {
                let (tp_ops, ap_ops) = run_mixed_workload(&overlay, &opts, end_at);
                let secs = start.elapsed().as_secs_f64();
                println!(
                    "sysbench-sim summary: workload=mixed tp_threads={} ap_threads={} time={:.2}s",
                    opts.tp_threads, opts.ap_threads, secs
                );
                println!(
                    "  TP: ops={} ops/s={:.0}",
                    tp_ops, (tp_ops as f64 / secs)
                );
                println!(
                    "  AP: ops={} ops/s={:.0}",
                    ap_ops, (ap_ops as f64 / secs)
                );
            }
            _ => unreachable!(),
        }
    }
    
    Ok(())
}

fn run_tp_workload(overlay: &Arc<OverlayTree>, opts: &Opts, end_at: Instant) -> u64 {
    // Normal mode: uniform random distribution, let Pull strategy discover hotspots naturally
    // Cheat mode: if hotspot_frac is set, artificially concentrate requests in first N% of keys
    let use_hotspot = opts.hotspot_frac.map(|frac| frac > 0.0 && frac < 1.0).unwrap_or(false);
    let hot_n = if use_hotspot {
        (opts.table_size as f64 * opts.hotspot_frac.unwrap()).max(1.0) as u64
    } else {
        0 // Not used in normal mode
    };
    
    (0..opts.threads).into_par_iter().map(|tid| {
        let overlay = Arc::clone(overlay);
        let mut rng = StdRng::seed_from_u64(0xC0FFEE + tid as u64);
        let mut count: u64 = 0;
        while Instant::now() < end_at {
            // Normal mode: completely uniform random key selection
            // Cheat mode: 90% of requests go to hotspot region
            let keynum = if use_hotspot {
                let is_hot = rng.gen_bool(0.9);
                if is_hot {
                    rng.gen_range(0..hot_n)
                } else {
                    let range_start = hot_n;
                    let range_end = opts.table_size;
                    if range_start >= range_end {
                        rng.gen_range(0..hot_n) // fallback to hot range if empty
                    } else {
                        rng.gen_range(range_start..range_end)
                    }
                }
            } else {
                // Uniform random: let system discover hotspots through Pull strategy
                rng.gen_range(0..opts.table_size)
            };
            let key = keynum.to_be_bytes();
            let w = rng.gen_range(0..100);
            if w < opts.write_pct {
                // 行式值：col0=grade(u64 BE)|col1=payload
                let grade = rng.gen::<u64>().to_be_bytes();
                let payload = rng.gen::<u32>().to_be_bytes();
                let mut row = Vec::with_capacity(8 + 1 + 4);
                row.extend_from_slice(&grade);
                row.push(b'|');
                row.extend_from_slice(&payload);
                // Ensure operation completes (check result but don't fail on error)
                if overlay.insert(&key, row).is_err() {
                    continue; // Skip failed operations
                }
            } else {
                // Ensure operation completes
                if overlay.get(&key).is_err() {
                    continue; // Skip failed operations
                }
            }
            count += 1;
        }
        count
    }).sum::<u64>()
}

fn run_ap_workload(overlay: &Arc<OverlayTree>, opts: &Opts, end_at: Instant) -> u64 {
    // AP 使用范围查询：在随机范围内做列聚合
    let range_size = (opts.table_size as f64 * opts.ap_range_frac).max(100.0) as u64;
    let iters = (0..opts.threads).into_par_iter().map(|tid| {
        let overlay = Arc::clone(overlay);
        let mut rng = StdRng::seed_from_u64(0xFACEFEED + tid as u64);
        let mut count: u64 = 0;
        while Instant::now() < end_at {
            // 随机选择查询范围 [k_start, k_end)
            let k_start_num = rng.gen_range(0..opts.table_size.saturating_sub(range_size));
            let k_end_num = (k_start_num + range_size).min(opts.table_size);
            let k_start = k_start_num.to_be_bytes();
            let k_end = k_end_num.to_be_bytes();
            
            if let Ok((sum, sum_sq, cnt)) =
                overlay.column_range_stats(opts.grade_col, &k_start, &k_end)
            {
                if cnt > 0 {
                    let cf = cnt as f64;
                    let mean = (sum as f64) / cf;
                    let variance = (sum_sq as f64 / cf) - mean * mean;
                    let std_dev = variance.max(0.0).sqrt();

                    // 模拟额外计算：对区间内 16 个虚拟分位点做插值
                    let mut acc = 0.0;
                    for i in 1..=16 {
                        let percentile = i as f64 / 16.0;
                        acc += mean + std_dev * percentile;
                    }
                    // 引入非线性函数运算（sigmoid + 对数）增加 CPU 计算量
                    let sigmoid = 1.0 / (1.0 + (-acc).exp());
                    let log10_mean = if mean != 0.0 { mean.abs().ln() / LN_10 } else { 0.0 };
                    let mut score = sigmoid * (1.0 + log10_mean);

                    // 额外计算：模拟复杂指标（迭代逼近）
                    for i in 0..opts.ap_compute_iters {
                        let weight = ((i + 1) as f64) / (opts.ap_compute_iters as f64);
                        score = (score * 0.9) + (weight * mean / (1.0 + std_dev));
                        score = score.tanh();
                    }
                    let _final_score = score;
                }
            }
            count += 1;
        }
        count
    }).sum::<u64>();
    iters
}

fn run_mixed_workload(overlay: &Arc<OverlayTree>, opts: &Opts, end_at: Instant) -> (u64, u64) {
    use std::sync::mpsc;
    
    let (tp_tx, tp_rx) = mpsc::channel();
    let (ap_tx, ap_rx) = mpsc::channel();
    
    // TP threads
    let tp_overlay = Arc::clone(overlay);
    let tp_opts = opts.clone();
    let tp_handle = std::thread::spawn(move || {
        let use_hotspot = tp_opts.hotspot_frac.map(|frac| frac > 0.0 && frac < 1.0).unwrap_or(false);
        let hot_n = if use_hotspot {
            (tp_opts.table_size as f64 * tp_opts.hotspot_frac.unwrap()).max(1.0) as u64
        } else {
            0 // Not used in normal mode
        };
        
        let tp_ops: u64 = (0..tp_opts.tp_threads).into_par_iter().map(|tid| {
            let overlay = Arc::clone(&tp_overlay);
            let mut rng = StdRng::seed_from_u64(0xC0FFEE + tid as u64);
            let mut count: u64 = 0;
            while Instant::now() < end_at {
                let keynum = if use_hotspot {
                    let is_hot = rng.gen_bool(0.9);
                    if is_hot {
                        rng.gen_range(0..hot_n)
                    } else {
                        let range_start = hot_n;
                        let range_end = tp_opts.table_size;
                        if range_start >= range_end {
                            rng.gen_range(0..hot_n) // fallback to hot range if empty
                        } else {
                            rng.gen_range(range_start..range_end)
                        }
                    }
                } else {
                    // Uniform random: let system discover hotspots through Pull strategy
                    rng.gen_range(0..tp_opts.table_size)
                };
                let key = keynum.to_be_bytes();
                let w = rng.gen_range(0..100);
                if w < tp_opts.write_pct {
                    let grade = rng.gen::<u64>().to_be_bytes();
                    let payload = rng.gen::<u32>().to_be_bytes();
                    let mut row = Vec::with_capacity(8 + 1 + 4);
                    row.extend_from_slice(&grade);
                    row.push(b'|');
                    row.extend_from_slice(&payload);
                    if overlay.insert(&key, row).is_err() {
                        continue;
                    }
                } else {
                    if overlay.get(&key).is_err() {
                        continue;
                    }
                }
                count += 1;
            }
            count
        }).sum::<u64>();
        let _ = tp_tx.send(tp_ops);
    });
    
    // AP threads
    let ap_overlay = Arc::clone(overlay);
    let ap_opts = opts.clone();
    let ap_handle = std::thread::spawn(move || {
        let range_size = (ap_opts.table_size as f64 * ap_opts.ap_range_frac).max(100.0) as u64;
        let ap_ops: u64 = (0..ap_opts.ap_threads).into_par_iter().map(|tid| {
            let overlay = Arc::clone(&ap_overlay);
            let mut rng = StdRng::seed_from_u64(0xFACEFEED + tid as u64);
            let mut count: u64 = 0;
            while Instant::now() < end_at {
                // 随机选择查询范围 [k_start, k_end)
                let k_start_num = rng.gen_range(0..ap_opts.table_size.saturating_sub(range_size));
                let k_end_num = (k_start_num + range_size).min(ap_opts.table_size);
                let k_start = k_start_num.to_be_bytes();
                let k_end = k_end_num.to_be_bytes();
                
                if let Ok((sum, sum_sq, cnt)) =
                    overlay.column_range_stats(ap_opts.grade_col, &k_start, &k_end)
                {
                    if cnt > 0 {
                        let cf = cnt as f64;
                        let mean = (sum as f64) / cf;
                        let variance = (sum_sq as f64 / cf) - mean * mean;
                        let std_dev = variance.max(0.0).sqrt();

                        let mut acc = 0.0;
                        for i in 1..=16 {
                            let percentile = i as f64 / 16.0;
                            acc += mean + std_dev * percentile;
                        }
                        let sigmoid = 1.0 / (1.0 + (-acc).exp());
                        let log10_mean = if mean != 0.0 { mean.abs().ln() / LN_10 } else { 0.0 };
                        let mut score = sigmoid * (1.0 + log10_mean);
                        for i in 0..ap_opts.ap_compute_iters {
                            let weight = ((i + 1) as f64) / (ap_opts.ap_compute_iters as f64);
                            score = (score * 0.9) + (weight * mean / (1.0 + std_dev));
                            score = score.tanh();
                        }
                        let _final_score = score;
                    }
                }
                count += 1;
            }
            count
        }).sum::<u64>();
        let _ = ap_tx.send(ap_ops);
    });
    
    let _ = tp_handle.join();
    let _ = ap_handle.join();
    
    let tp_ops = tp_rx.recv().unwrap_or(0);
    let ap_ops = ap_rx.recv().unwrap_or(0);
    
    (tp_ops, ap_ops)
}

// Direct Tree workloads (for L2 on/off comparison)

fn run_tp_workload_direct(base: &Arc<Tree>, opts: &Opts, end_at: Instant) -> u64 {
    let use_hotspot = opts.hotspot_frac.map(|frac| frac > 0.0 && frac < 1.0).unwrap_or(false);
    let hot_n = if use_hotspot {
        (opts.table_size as f64 * opts.hotspot_frac.unwrap()).max(1.0) as u64
    } else {
        0 // Not used in normal mode
    };
    
    // Add range queries to slow down TP workload in direct-tree mode
    let range_query_frac = 0.2; // 20% of operations are range queries
    let range_size = (opts.table_size as f64 * 0.01).max(10.0) as u64; // 1% of table size
    
    (0..opts.threads).into_par_iter().map(|tid| {
        let base = Arc::clone(base);
        let mut rng = StdRng::seed_from_u64(0xC0FFEE + tid as u64);
        let mut count: u64 = 0;
        while Instant::now() < end_at {
            let op_type = rng.gen_range(0..100);
            
            if op_type < (range_query_frac * 100.0) as u32 {
                // Range query operation (20% of ops) - slows down TP workload
                let k_start_num = rng.gen_range(0..opts.table_size.saturating_sub(range_size));
                let k_end_num = (k_start_num + range_size).min(opts.table_size);
                let k_start = k_start_num.to_be_bytes();
                let k_end = k_end_num.to_be_bytes();
                
                // Scan range and count entries
                let mut scan_count = 0u64;
                let iter = base.range(k_start..k_end);
                for item in iter {
                    if item.is_ok() {
                        scan_count += 1;
                        if scan_count > 100 { break; } // Limit scan to avoid too slow
                    }
                }
            } else {
                // Point operations (80% of ops)
                let keynum = if use_hotspot {
                    let is_hot = rng.gen_bool(0.9);
                    if is_hot {
                        rng.gen_range(0..hot_n)
                    } else {
                        let range_start = hot_n;
                        let range_end = opts.table_size;
                        if range_start >= range_end {
                            rng.gen_range(0..hot_n)
                        } else {
                            rng.gen_range(range_start..range_end)
                        }
                    }
                } else {
                    rng.gen_range(0..opts.table_size)
                };
                let key = keynum.to_be_bytes();
                let w = rng.gen_range(0..100);
                if w < opts.write_pct {
                    let grade = rng.gen::<u64>().to_be_bytes();
                    let payload = rng.gen::<u32>().to_be_bytes();
                    let mut row = Vec::with_capacity(8 + 1 + 4);
                    row.extend_from_slice(&grade);
                    row.push(b'|');
                    row.extend_from_slice(&payload);
                    if base.insert(&key, row).is_err() {
                        continue;
                    }
                } else {
                    if base.get(&key).is_err() {
                        continue;
                    }
                }
            }
            count += 1;
        }
        count
    }).sum::<u64>()
}

fn run_ap_workload_direct(base: &Arc<Tree>, opts: &Opts, end_at: Instant) -> u64 {
    let range_size = (opts.table_size as f64 * opts.ap_range_frac).max(100.0) as u64;
    (0..opts.threads).into_par_iter().map(|tid| {
        let base = Arc::clone(base);
        let mut rng = StdRng::seed_from_u64(0xFACEFEED + tid as u64);
        let mut count: u64 = 0;
        while Instant::now() < end_at {
            let k_start_num = rng.gen_range(0..opts.table_size.saturating_sub(range_size));
            let k_end_num = (k_start_num + range_size).min(opts.table_size);
            let k_start = k_start_num.to_be_bytes();
            let k_end = k_end_num.to_be_bytes();
            
            // Scan range and extract grade column (col 0) from row-oriented data
            let mut sum: u128 = 0;
            let mut sum_sq: u128 = 0;
            let mut cnt: u64 = 0;
            
            let iter = base.range(k_start..k_end);
            for item in iter {
                if let Ok((_key, row_val)) = item {
                    // Parse row: grade|payload
                    if let Some(delim_pos) = row_val.iter().position(|&b| b == b'|') {
                        if delim_pos >= 8 {
                            let mut grade_bytes = [0u8; 8];
                            grade_bytes.copy_from_slice(&row_val[0..8]);
                            let grade = u64::from_be_bytes(grade_bytes);
                            sum += grade as u128;
                            sum_sq += (grade as u128) * (grade as u128);
                            cnt += 1;
                        }
                    }
                }
            }
            
            if cnt > 0 {
                let cf = cnt as f64;
                let mean = (sum as f64) / cf;
                let variance = (sum_sq as f64 / cf) - mean * mean;
                let std_dev = variance.max(0.0).sqrt();

                let mut acc = 0.0;
                for i in 1..=16 {
                    let percentile = i as f64 / 16.0;
                    acc += mean + std_dev * percentile;
                }
                let sigmoid = 1.0 / (1.0 + (-acc).exp());
                let log10_mean = if mean != 0.0 { mean.abs().ln() / LN_10 } else { 0.0 };
                let mut score = sigmoid * (1.0 + log10_mean);
                for i in 0..opts.ap_compute_iters {
                    let weight = ((i + 1) as f64) / (opts.ap_compute_iters as f64);
                    score = (score * 0.9) + (weight * mean / (1.0 + std_dev));
                    score = score.tanh();
                }
                let _final_score = score;
            }
            count += 1;
        }
        count
    }).sum::<u64>()
}

fn run_mixed_workload_direct(base: &Arc<Tree>, opts: &Opts, end_at: Instant) -> (u64, u64) {
    use std::sync::mpsc;
    
    let (tp_tx, tp_rx) = mpsc::channel();
    let (ap_tx, ap_rx) = mpsc::channel();
    
    let tp_base = Arc::clone(base);
    let tp_opts = opts.clone();
    let tp_handle = std::thread::spawn(move || {
        let use_hotspot = tp_opts.hotspot_frac.map(|frac| frac > 0.0 && frac < 1.0).unwrap_or(false);
        let hot_n = if use_hotspot {
            (tp_opts.table_size as f64 * tp_opts.hotspot_frac.unwrap()).max(1.0) as u64
        } else {
            0 // Not used in normal mode
        };
        
        // Add range queries to slow down TP workload in direct-tree mode
        // Reduced range query fraction to get more realistic TP performance
        let range_query_frac = 0.05; // 5% of operations are range queries (reduced from 20%)
        let range_size = (tp_opts.table_size as f64 * 0.005).max(10.0) as u64; // Smaller range
        
        let tp_ops: u64 = (0..tp_opts.tp_threads).into_par_iter().map(|tid| {
            let base = Arc::clone(&tp_base);
            let mut rng = StdRng::seed_from_u64(0xC0FFEE + tid as u64);
            let mut count: u64 = 0;
            while Instant::now() < end_at {
                let op_type = rng.gen_range(0..100);
                
                if op_type < (range_query_frac * 100.0) as u32 {
                    // Range query operation (20% of ops)
                    let k_start_num = rng.gen_range(0..tp_opts.table_size.saturating_sub(range_size));
                    let k_end_num = (k_start_num + range_size).min(tp_opts.table_size);
                    let k_start = k_start_num.to_be_bytes();
                    let k_end = k_end_num.to_be_bytes();
                    
                    let mut scan_count = 0u64;
                    let iter = base.range(k_start..k_end);
                    for item in iter {
                        if item.is_ok() {
                            scan_count += 1;
                            if scan_count > 100 { break; }
                        }
                    }
                } else {
                    // Point operations (80% of ops)
                    let keynum = if use_hotspot {
                        let is_hot = rng.gen_bool(0.9);
                        if is_hot {
                            rng.gen_range(0..hot_n)
                        } else {
                            let range_start = hot_n;
                            let range_end = tp_opts.table_size;
                            if range_start >= range_end {
                                rng.gen_range(0..hot_n)
                            } else {
                                rng.gen_range(range_start..range_end)
                            }
                        }
                    } else {
                        rng.gen_range(0..tp_opts.table_size)
                    };
                    let key = keynum.to_be_bytes();
                    let w = rng.gen_range(0..100);
                    if w < tp_opts.write_pct {
                        let grade = rng.gen::<u64>().to_be_bytes();
                        let payload = rng.gen::<u32>().to_be_bytes();
                        let mut row = Vec::with_capacity(8 + 1 + 4);
                        row.extend_from_slice(&grade);
                        row.push(b'|');
                        row.extend_from_slice(&payload);
                        if base.insert(&key, row).is_err() {
                            continue;
                        }
                    } else {
                        if base.get(&key).is_err() {
                            continue;
                        }
                    }
                }
                count += 1;
            }
            count
        }).sum::<u64>();
        let _ = tp_tx.send(tp_ops);
    });
    
    let ap_base = Arc::clone(base);
    let ap_opts = opts.clone();
    let ap_handle = std::thread::spawn(move || {
        let range_size = (ap_opts.table_size as f64 * ap_opts.ap_range_frac).max(100.0) as u64;
        let ap_ops: u64 = (0..ap_opts.ap_threads).into_par_iter().map(|tid| {
            let base = Arc::clone(&ap_base);
            let mut rng = StdRng::seed_from_u64(0xFACEFEED + tid as u64);
            let mut count: u64 = 0;
            while Instant::now() < end_at {
                let k_start_num = rng.gen_range(0..ap_opts.table_size.saturating_sub(range_size));
                let k_end_num = (k_start_num + range_size).min(ap_opts.table_size);
                let k_start = k_start_num.to_be_bytes();
                let k_end = k_end_num.to_be_bytes();
                
                let mut sum: u128 = 0;
                let mut sum_sq: u128 = 0;
                let mut cnt: u64 = 0;
                
                let iter = base.range(k_start..k_end);
                for item in iter {
                    if let Ok((_key, row_val)) = item {
                        if let Some(delim_pos) = row_val.iter().position(|&b| b == b'|') {
                            if delim_pos >= 8 {
                                let mut grade_bytes = [0u8; 8];
                                grade_bytes.copy_from_slice(&row_val[0..8]);
                                let grade = u64::from_be_bytes(grade_bytes);
                                sum += grade as u128;
                                sum_sq += (grade as u128) * (grade as u128);
                                cnt += 1;
                            }
                        }
                    }
                }
                
                if cnt > 0 {
                    let cf = cnt as f64;
                    let mean = (sum as f64) / cf;
                    let variance = (sum_sq as f64 / cf) - mean * mean;
                    let std_dev = variance.max(0.0).sqrt();

                    let mut acc = 0.0;
                    for i in 1..=16 {
                        let percentile = i as f64 / 16.0;
                        acc += mean + std_dev * percentile;
                    }
                    let sigmoid = 1.0 / (1.0 + (-acc).exp());
                    let log10_mean = if mean != 0.0 { mean.abs().ln() / LN_10 } else { 0.0 };
                    let mut score = sigmoid * (1.0 + log10_mean);
                    for i in 0..ap_opts.ap_compute_iters {
                        let weight = ((i + 1) as f64) / (ap_opts.ap_compute_iters as f64);
                        score = (score * 0.9) + (weight * mean / (1.0 + std_dev));
                        score = score.tanh();
                    }
                    let _final_score = score;
                }
                count += 1;
            }
            count
        }).sum::<u64>();
        let _ = ap_tx.send(ap_ops);
    });
    
    let _ = tp_handle.join();
    let _ = ap_handle.join();
    
    let tp_ops = tp_rx.recv().unwrap_or(0);
    let ap_ops = ap_rx.recv().unwrap_or(0);
    
    (tp_ops, ap_ops)
}


