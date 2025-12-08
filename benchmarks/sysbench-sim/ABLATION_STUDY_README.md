# Ablation Study for HTAP System

This directory contains a comprehensive ablation study script for testing the HTAP (Hybrid Transactional/Analytical Processing) system.

## Overview

The `ablation_study.sh` script runs three sets of experiments to evaluate the system's performance:

### Experiment 1: All Optimizations Enabled
- Tests system performance with **all optimizations enabled** (L2 columnar storage + Pull-Push strategy)
- Varies TP:AP ratios: 1:0, 3:1, 1:1, 1:3, 0:1
- Purpose: Establish baseline performance under optimal configuration

### Experiment 2: L2 Columnar Storage On/Off
- Compares **L2 columnar storage** (OverlayTree) vs **direct Tree** (row-oriented storage)
- Varies TP:AP ratios: 3:1, 1:1, 1:3
- Purpose: Evaluate the impact of columnar storage on mixed workload performance

### Experiment 3: Pull-Push Strategy On/Off
- Compares **Pull-Push strategy enabled** vs **disabled** (with L2 enabled)
- Varies TP:AP ratios: 3:1, 1:1, 1:3
- Purpose: Evaluate the effectiveness of frequency-based data promotion (Pull) strategy

## Usage

### Prerequisites

1. Build the benchmark:
   ```bash
   cd benchmarks/sysbench-sim
   cargo build --release
   ```

2. Ensure `bc` command is available (for calculations):
   ```bash
   which bc  # Should show path to bc
   ```

### Running the Ablation Study

```bash
cd benchmarks/sysbench-sim
./ablation_study.sh
```

### Output

The script generates a CSV file with results:
- Location: `benchmarks/sysbench-sim/results/ablation_YYYYMMDD_HHMMSS.csv`
- Format: CSV with columns: experiment, config, tp_ap_ratio, l2_enabled, pull_enabled, tp_threads, ap_threads, tp_ops, ap_ops, tp_ops_per_sec, ap_ops_per_sec, total_ops_per_sec

### Configuration Parameters

You can modify these parameters in the script:

- `TABLE_SIZE`: Total key space (default: 1,000,000)
- `TIME`: Test duration in seconds (default: 10)
- `PRELOAD_TIME`: Preload duration in seconds (default: 30)
- `THREADS_TOTAL`: Total threads for preloading (default: 8)
- `HOTSPOT_FRAC`: **Optional cheat interface** - Fraction of keys that are artificially "hot" (default: not set, uses uniform random distribution)
  - **Normal mode**: Not set - TP workload uses uniform random key distribution, letting the Pull strategy discover hotspots naturally
  - **Cheat mode**: Set to value (e.g., 0.01) - Artificially concentrates 90% of TP requests in first N% of keys to test system behavior with known hotspots
- `AP_RANGE_FRAC`: AP range query size as fraction of table_size (default: 0.1)
- `AP_COMPUTE_ITERS`: CPU iterations per AP query (default: 256)

## Understanding the Results

### Experiment 1 (All Optimizations)
- Look for overall throughput trends across different TP:AP ratios
- Identify optimal workload balance points

### Experiment 2 (L2 On/Off)
- Compare `l2_on` vs `l2_off` results for each TP:AP ratio
- Higher AP throughput with `l2_on` indicates columnar storage benefit
- Check if TP performance degrades with L2 enabled

### Experiment 3 (Pull-Push On/Off)
- Compare `pull_on` vs `pull_off` results for each TP:AP ratio
- Higher TP throughput with `pull_on` indicates effective hotspot promotion
- Check if AP performance is maintained with Pull strategy

## Analysis Tips

1. **Isolation Check**: Compare TP ops/s with AP running vs without AP
2. **L2 Benefit**: Compare AP ops/s with L2 on vs L2 off
3. **Pull Effectiveness**: Compare TP ops/s with Pull on vs Pull off (should be higher for hot data)
4. **Mixed Load Impact**: Check how TP performance changes as AP load increases

## Troubleshooting

### "cannot sample empty range" Error
- This occurs when `--hotspot-frac` is set too high (e.g., 1.0)
- Solution: Use `--hotspot-frac < 1.0` if using cheat interface, or don't set it at all (default normal mode)

### Zero Throughput Results
- Check if data was preloaded successfully
- Verify benchmark compiled in release mode
- Check system resources (CPU, memory, disk I/O)

### Parsing Errors in Script
- Ensure output format matches expected grep patterns
- Check if benchmark output changed
- Verify `bc` command is available

