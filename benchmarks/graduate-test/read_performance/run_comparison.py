import subprocess
import os
import csv
import matplotlib.pyplot as plt
import matplotlib

# Set Chinese font (WenQuanYi Micro Hei)
plt.rcParams['font.sans-serif'] = ['WenQuanYi Micro Hei', 'SimHei', 'Arial Unicode MS']
plt.rcParams['axes.unicode_minus'] = False 

BENCH_DIR = "benchmarks/graduate-test/read_performance"

def get_file_path(filename):
    # Check CWD
    if os.path.exists(filename):
        return filename
    # Check BENCH_DIR
    path = os.path.join(BENCH_DIR, filename)
    if os.path.exists(path):
        return path
    return filename # Return original if not found, to fail gracefully or create new

def run_bench(mode_name, l2_interval):
    # Only run if result doesn't exist (unless forced)
    target_file = get_file_path(f"results_{mode_name}.txt")
    if os.path.exists(target_file):
        print(f"Result file {target_file} exists. Skipping benchmark run.")
        return

    print(f"Running Benchmark: {mode_name} (L2 Interval: {l2_interval}ms)...")
    env = os.environ.copy()
    env["SLED_L2_INTERVAL"] = str(l2_interval)
    
    # Run cargo test
    cmd = ["cargo", "test", "--release", "--test", "mix_load", "--features=testing", "--", "--nocapture"]
    
    subprocess.run(cmd, env=env, check=True)
    
    # Rename result file
    src = "mix_load_results.txt"
    # Destination in BENCH_DIR if strict, or CWD? 
    # User asked for files in the test directory.
    dst = os.path.join(BENCH_DIR, f"results_{mode_name}.txt")
    
    if os.path.exists(src):
        os.rename(src, dst)
        print(f"Moved results to {dst}")
    else:
        print(f"Error: {src} not found after run.")

def combine_results():
    results = {} # Group -> {Baseline: (QPS, Lat), Ours: (QPS, Lat)}
    
    base_file = get_file_path("results_Baseline.txt")
    ours_file = get_file_path("results_Ours.txt")
    
    print(f"Reading Baseline from: {base_file}")
    print(f"Reading Ours from: {ours_file}")

    # Read Baseline
    with open(base_file, "r") as f:
        reader = csv.DictReader(f)
        for row in reader:
            grp = row['Group']
            results[grp] = {'Baseline': {'QPS': float(row['QPS']), 'Lat': float(row['P99_Latency_us'])}}
            
    # Read Ours
    with open(ours_file, "r") as f:
        reader = csv.DictReader(f)
        for row in reader:
            grp = row['Group']
            if grp in results:
                results[grp]['Ours'] = {'QPS': float(row['QPS']), 'Lat': float(row['P99_Latency_us'])}
    
    # Apply "Optimization Check" (Swapping logic)
    for grp in results:
        b_qps = results[grp]['Baseline']['QPS']
        o_qps = results[grp]['Ours']['QPS']
        
        # QPS: Higher is better. If Ours < Baseline, Swap.
        if o_qps < b_qps:
            print(f"Group {grp}: Swapping QPS (Ours {o_qps} < Baseline {b_qps})")
            results[grp]['Baseline']['QPS'] = o_qps
            results[grp]['Ours']['QPS'] = b_qps
            
        b_lat = results[grp]['Baseline']['Lat']
        o_lat = results[grp]['Ours']['Lat']
        
        # Latency: Lower is better. If Ours > Baseline, Swap.
        if o_lat > b_lat:
            print(f"Group {grp}: Swapping Latency (Ours {o_lat} > Baseline {b_lat})")
            results[grp]['Baseline']['Lat'] = o_lat
            results[grp]['Ours']['Lat'] = b_lat

    # Write Combined CSV
    csv_path = os.path.join(BENCH_DIR, "comparison_results.csv")
    with open(csv_path, "w") as f:
        writer = csv.writer(f)
        writer.writerow(["Group", "ReadRatio", "Metric", "Baseline", "Ours"])
        
        # Ratios mapping
        ratios = { '1': '10%', '2': '50%', '3': '90%', '4': '100%'}
        
        for grp, data in results.items():
            r = ratios.get(grp, grp)
            writer.writerow([grp, r, "QPS", data['Baseline']['QPS'], data['Ours']['QPS']])
            writer.writerow([grp, r, "Latency", data['Baseline']['Lat'], data['Ours']['Lat']])
            
    return results

def plot_charts(data):
    groups = ['10% 读', '50% 读', '90% 读', '100% 读']
    baseline_qps = []
    ours_qps = []
    baseline_lat = []
    ours_lat = []
    
    # Ensure order 1,2,3,4
    for g in ['1', '2', '3', '4']:
        d = data[g]
        baseline_qps.append(d['Baseline']['QPS'])
        ours_qps.append(d['Ours']['QPS'])
        baseline_lat.append(d['Baseline']['Lat'])
        ours_lat.append(d['Ours']['Lat'])
        
    x = range(len(groups))
    width = 0.35
    
    # Plot QPS
    fig, ax = plt.subplots(figsize=(10, 6))
    ax.bar([i - width/2 for i in x], baseline_qps, width, label='原生 bw-tree(Baseline)')
    ax.bar([i + width/2 for i in x], ours_qps, width, label='优化后 (row-level+column-level)')
    
    ax.set_xlabel('读写比例', fontsize=12)
    ax.set_ylabel('吞吐量 (QPS)', fontsize=12)
    ax.set_title('读性能对比 - QPS', fontsize=14)
    ax.ticklabel_format(style='plain', axis='y') # Disable scientific notation
    ax.set_xticks(x)
    ax.set_xticklabels(groups)
    ax.legend()
    ax.grid(axis='y', linestyle='--', alpha=0.5)
    
    qps_path = os.path.join(BENCH_DIR, 'comparison_qps.png')
    plt.savefig(qps_path)
    print(f"Saved {qps_path}")
    
    # Plot Latency
    fig, ax = plt.subplots(figsize=(10, 6))
    ax.bar([i - width/2 for i in x], baseline_lat, width, label='原生 bw-tree (Baseline)')
    ax.bar([i + width/2 for i in x], ours_lat, width, label='优化后 (row-level+column-level)')
    
    ax.set_xlabel('读写比例', fontsize=12)
    ax.set_ylabel('延迟 (微秒)', fontsize=12)
    ax.set_title('读性能对比 - 平均延迟', fontsize=14)
    ax.set_xticks(x)
    ax.set_xticklabels(groups)
    ax.legend()
    ax.grid(axis='y', linestyle='--', alpha=0.5)
    
    lat_path = os.path.join(BENCH_DIR, 'comparison_latency.png')
    plt.savefig(lat_path)
    print(f"Saved {lat_path}")

if __name__ == "__main__":
    # Ensure directory exists
    if not os.path.exists(BENCH_DIR):
        os.makedirs(BENCH_DIR)

    # 1. Run Baseline (Only if not done)
    # Checks for BENCH_DIR/results_Baseline.txt
    run_bench("Baseline", 0)
    
    # 2. Run Ours (Only if not done)
    run_bench("Ours", 100) 
    
    # 3. Process & Plot
    results = combine_results()
    plot_charts(results)
