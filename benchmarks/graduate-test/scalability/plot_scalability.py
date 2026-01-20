import csv
import os
import matplotlib.pyplot as plt

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

def plot_charts():
    # Read Data
    data = {} # {Scenario: {Mode: {Threads: QPS}}}
    if not os.path.exists(CSV_FILE):
        print(f"No data found at {CSV_FILE}.")
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
        if name not in data: 
            print(f"Skipping {name}, no data.")
            continue
        
        plt.figure(figsize=(10, 6))
        
        # Plot LockFree
        lf_data = data[name].get('lockfree', {})
        if lf_data:
            lf_x = sorted(lf_data.keys())
            lf_y = [lf_data[k] for k in lf_x]
            plt.plot(lf_x, lf_y, marker='o', label='无锁 (Lock-Free CAS)', linewidth=2)

        # Plot Mutex
        mu_data = data[name].get('mutex', {})
        if mu_data:
            mu_x = sorted(mu_data.keys())
            mu_y = [mu_data[k] for k in mu_x]
            plt.plot(mu_x, mu_y, marker='x', label='有锁 (Global Mutex)', linestyle='--', linewidth=2)

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
    plot_charts()
