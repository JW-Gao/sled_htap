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

if __name__ == "__main__":
    if not os.path.exists(BENCH_DIR):
        os.makedirs(BENCH_DIR)
        
    run_bench()
    print(f"Data generation complete. Results saved to {CSV_FILE}")

