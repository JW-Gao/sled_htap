import subprocess
import os
import csv
import matplotlib.pyplot as plt
import matplotlib

# Set Chinese font
plt.rcParams['font.sans-serif'] = ['WenQuanYi Micro Hei', 'SimHei', 'Arial Unicode MS']
plt.rcParams['axes.unicode_minus'] = False 

BENCH_DIR = "benchmarks/graduate-test/scalability"
CSV_FILE = os.path.join(BENCH_DIR, "scalability_results.csv")

SCENARIOS = [
    {"name": "WriteOnly", "ratio": 0},
    {"name": "Balanced", "ratio": 50},
    {"name": "ReadOnly", "ratio": 95},
]

MODES = ["lockfree", "mutex"]
THREADS = [1, 2, 4, 8, 16, 32, 64]

def run_bench():
    # Initialize CSV if not exists
    if not os.path.exists(CSV_FILE):
        with open(CSV_FILE, "w") as f:
            f.write("Scenario,Mode,Threads,QPS\n")

    for scen in SCENARIOS:
        for mode in MODES:
            for t in THREADS:
                print(f"Running {scen['name']} ({scen['ratio']}% Read) | Mode: {mode} | Threads: {t}...")
                
                # Check if already in CSV (Resume capability)
                if check_if_done(scen['name'], mode, t):
                    print("  -> Already done, skipping.")
                    continue

                cmd = [
                    "cargo", "test", "--release", "--test", "cas_scalability", 
                    "--features=testing", "--", "--nocapture",
                    "--threads", str(t),
                    "--mode", mode,
                    "--read-ratio", str(scen['ratio'])
                ]
                
                # Run from project root
                result = subprocess.run(cmd, cwd=".", capture_output=True, text=True)
                
                if result.returncode != 0:
                    print(f"Error running benchmark: {result.stderr}")
                    continue
                
                # Parse output
                # Look for line: "RESULT: mode,threads,ratio,qps"
                qps = 0.0
                for line in result.stdout.splitlines():
                    if line.startswith("RESULT:"):
                        parts = line.split(",")
                        qps = float(parts[3])
                        break
                
                print(f"  -> QPS: {qps}")
                
                # Append to CSV
                with open(CSV_FILE, "a") as f:
                    # Scenario Name mapping for CSV convenience
                    f.write(f"{scen['name']},{mode},{t},{qps}\n")

def check_if_done(scen_name, mode, threads):
    if not os.path.exists(CSV_FILE): return False
    with open(CSV_FILE, "r") as f:
        reader = csv.DictReader(f)
        for row in reader:
            if row['Scenario'] == scen_name and row['Mode'] == mode and int(row['Threads']) == threads:
                return True
    return False

def plot_charts():
    # Read Data
    data = {} # {Scenario: {Mode: {Threads: QPS}}}
    if not os.path.exists(CSV_FILE):
        print("No data found.")
        return

    with open(CSV_FILE, "r") as f:
        reader = csv.DictReader(f)
        for row in reader:
            scen = row['Scenario']
            mode = row['Mode']
            t = int(row['Threads'])
            qps = float(row['QPS'])
            
            if scen not in data: data[scen] = {}
            if mode not in data[scen]: data[scen][mode] = {}
            data[scen][mode][t] = qps

    # Plot
    for scen in SCENARIOS:
        name = scen['name']
        if name not in data: continue
        
        plt.figure(figsize=(10, 6))
        
        # Plot LockFree
        lf_data = data[name].get('lockfree', {})
        lf_x = sorted(lf_data.keys())
        lf_y = [lf_data[k] for k in lf_x]
        plt.plot(lf_x, lf_y, marker='o', label='无锁 (Lock-Free CAS)', linewidth=2)

        # Plot Mutex
        mu_data = data[name].get('mutex', {})
        mu_x = sorted(mu_data.keys())
        mu_y = [mu_data[k] for k in mu_x]
        plt.plot(mu_x, mu_y, marker='x', label='有锁 (Global Mutex)', linestyle='--', linewidth=2)

        # Plot Ideal Line (Based on LockFree thread 1)
        if lf_x:
            base_qps = lf_y[0]
            ideal_y = [base_qps * t for t in lf_x]
            # normalize ideal to fit graph (optional, usually confusing if real perf is much lower than ideal cpu)
            # Let's just plot real data comparison, maybe ideal is too high.
        
        plt.xlabel('并发线程数 (Threads)', fontsize=12)
        plt.ylabel('吞吐量 (OPS)', fontsize=12)
        plt.title(f'线性扩展性对比 - {name} (读比例 {scen["ratio"]}%)', fontsize=14)
        plt.legend()
        plt.grid(True, linestyle='--', alpha=0.5)
        
        # Disable scientific notation
        plt.ticklabel_format(style='plain', axis='y')
        
        out_path = os.path.join(BENCH_DIR, f"scalability_{name}.png")
        plt.savefig(out_path)
        print(f"Saved {out_path}")
        plt.close()

if __name__ == "__main__":
    if not os.path.exists(BENCH_DIR):
        os.makedirs(BENCH_DIR)
        
    run_bench()
    plot_charts()
