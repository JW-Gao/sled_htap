import matplotlib.pyplot as plt
import csv
import os

def plot_mix_load():
    filename = 'mix_load_results.txt'
    if not os.path.exists(filename):
        print(f"{filename} not found.")
        return

    groups = []
    ratios = []
    qps = []
    latencies = []

    with open(filename, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            groups.append(row['Group'])
            ratios.append(f"R{row['ReadRatio']}/W{row['WriteRatio']}")
            qps.append(float(row['QPS']))
            latencies.append(float(row['P99_Latency_us']))

    # Plot QPS
    plt.figure(figsize=(10, 6))
    plt.bar(ratios, qps, color='skyblue')
    plt.title('Mixed Workload Throughput (QPS)')
    plt.xlabel('Workload Ratio')
    plt.ylabel('QPS')
    plt.grid(axis='y', linestyle='--', alpha=0.7)
    plt.savefig('mix_load_qps.png')
    print("Saved mix_load_qps.png")

    # Plot Latency
    plt.figure(figsize=(10, 6))
    plt.bar(ratios, latencies, color='salmon')
    plt.title('Mixed Workload Mean Latency')
    plt.xlabel('Workload Ratio')
    plt.ylabel('Latency (us)')
    plt.grid(axis='y', linestyle='--', alpha=0.7)
    plt.savefig('mix_load_latency.png')
    print("Saved mix_load_latency.png")

def plot_range_scan():
    filename = 'range_scan_results.txt'
    if not os.path.exists(filename):
        print(f"{filename} not found.")
        return

    iters = []
    write_lats = []
    scan_lats = []

    with open(filename, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            iters.append(int(row['Iteration']))
            write_lats.append(float(row['WriteLatency_ms']))
            scan_lats.append(float(row['ScanLatency_ms']))

    plt.figure(figsize=(12, 6))
    plt.plot(iters, write_lats, label='Write Latency (Frag)', marker='o', linestyle='--')
    plt.plot(iters, scan_lats, label='Scan Latency', marker='s', linewidth=2)
    plt.title('Range Scan Performance Trend (Fragmentation vs L2 Merge)')
    plt.xlabel('Iteration')
    plt.ylabel('Latency (ms)')
    plt.legend()
    plt.grid(True, linestyle='--', alpha=0.5)
    plt.savefig('range_scan_trend.png')
    print("Saved range_scan_trend.png")

if __name__ == "__main__":
    plot_mix_load()
    plot_range_scan()
