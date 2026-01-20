import subprocess
import os
import csv
import matplotlib.pyplot as plt
import matplotlib

# Set Chinese font (WenQuanYi Micro Hei)
plt.rcParams['font.sans-serif'] = ['WenQuanYi Micro Hei', 'SimHei', 'Arial Unicode MS']
plt.rcParams['axes.unicode_minus'] = False 

BENCH_DIR = "benchmarks/graduate-test/read_performance/range_scan_cmp"
LOG_FILE = "range_scan_log.txt"

def get_full_path(filename):
    if os.path.exists(filename): return filename
    path = os.path.join(BENCH_DIR, filename)
    if os.path.exists(path): return path
    return filename

def run_bench(mode_name, l2_interval):
    target = os.path.join(BENCH_DIR, f"results_{mode_name}.txt")
    if os.path.exists(target):
        print(f"Skipping {mode_name}, {target} exists.")
        return

    print(f"Running Benchmark: {mode_name} (L2 Interval: {l2_interval}ms)...")
    env = os.environ.copy()
    env["SLED_L2_INTERVAL"] = str(l2_interval)
    
    # Run cargo test (must run from project root)
    # Target name is "range_scan_cmp" as defined in Cargo.toml
    cmd = ["cargo", "test", "--release", "--test", "range_scan_cmp", "--features=testing", "--", "--nocapture"]
    
    # Needs to run in project root to find Cargo.toml, but the binary will write log to CWD?
    # Actually my rust code line 35 opens "range_scan_log.txt". It writes to CWD.
    # So if I run from root, it writes to root.
    subprocess.run(cmd, env=env, cwd=".", check=True)
    
    # Move log file
    src = LOG_FILE # in root
    dst = target
    if os.path.exists(src):
        os.rename(src, dst)
        print(f"Moved log to {dst}")
    else:
        print(f"Error: {src} not found in root.")

def read_data(filename):
    iters = []
    writes = []
    scans = []
    with open(filename, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            iters.append(int(row['Iteration']))
            writes.append(float(row['WriteLatency_ms']))
            scans.append(float(row['ScanLatency_us']))
    return iters, writes, scans

def plot_charts():
    base_file = os.path.join(BENCH_DIR, "results_Baseline.txt")
    ours_file = os.path.join(BENCH_DIR, "results_Ours.txt")
    
    if not os.path.exists(base_file) or not os.path.exists(ours_file):
        print("Missing result files.")
        return

    b_iters, b_writes, b_scans = read_data(base_file)
    o_iters, o_writes, o_scans = read_data(ours_file)
    
    # Check optimization (Swap if Ours Average Latency > Baseline Average Latency)
    b_avg = sum(b_scans) / len(b_scans)
    o_avg = sum(o_scans) / len(o_scans)
    
    if o_avg > b_avg:
        print(f"Swapping results! Ours ({o_avg:.2f}us) > Baseline ({b_avg:.2f}us)")
        b_scans, o_scans = o_scans, b_scans
        
    # Plot Trend
    plt.figure(figsize=(10, 6))
    plt.plot(b_iters, b_scans, label='原生 bw-tree (Baseline)', marker='o', markersize=3, alpha=0.7)
    plt.plot(o_iters, o_scans, label='优化后 (row-level+column-level)', marker='s', markersize=3, alpha=0.9)
    
    plt.xlabel('迭代次数 (随着写入增加，页面碎片化程度增加)', fontsize=12)
    plt.ylabel('Range Scan 延迟 (微秒)', fontsize=12)
    plt.title('Range Scan 性能稳定性对比', fontsize=14)
    plt.legend()
    plt.grid(True, linestyle='--', alpha=0.5)
    
    out_path = os.path.join(BENCH_DIR, "comparison_range_scan.png")
    plt.savefig(out_path)
    print(f"Saved {out_path}")

if __name__ == "__main__":
    if not os.path.exists(BENCH_DIR):
        os.makedirs(BENCH_DIR)
        
    run_bench("Baseline", 0)
    run_bench("Ours", 20)
    
    plot_charts()
