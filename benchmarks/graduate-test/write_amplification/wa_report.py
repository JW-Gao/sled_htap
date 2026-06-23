import subprocess
import os
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import time

# Configuration
ops_counts = [3000, 5000, 8000, 15000, 30000]
modes = ["BASELINE", "OPTIMIZED"]
binary_path = "./target/release/write_amp_validation"
csv_path = "wa_results.csv"

# 1. Compilation
print("Compiling Benchmark...")
subprocess.run(["cargo", "build", "--release"], check=True)

# 2. Key Generation & Execution
# We run the binary multiple times. Each time it cleans up previous data.
# New requirement: Random keys are handled inside the binary. Code updated.

# Clear previous results if we want a fresh start for the full report
# if os.path.exists(csv_path):
#     os.remove(csv_path)

for mode in modes:
    for ops in ops_counts:
        print(f"\n---> Running Mode: {mode}, Ops: {ops}")
        env = os.environ.copy()
        env["TIERED_MODE"] = mode
        env["BENCH_OPS"] = str(ops)
        env["RUST_LOG"] = "info"
        
        # Skipped re-running tests as requested by user. 
        # Only generating plot from existing wa_results.csv
        # try:
        #     subprocess.run([binary_path], env=env, check=True)
        # except subprocess.CalledProcessError as e:
        #     print(f"Error running benchmark: {e}")

print("\n=== Benchmark Completed. Analyzing Results... ===")

# 3. Analysis & Visualization
if os.path.exists(csv_path):
    time.sleep(120)
    df = pd.read_csv(csv_path)
    print(df)
    
    # Calculate Write Amplification
    # Payload = 100 bytes approx. 
    # WA = Total Written / (Ops * 100)
    # Note: 'total_written_bytes' column exists from the Rust binary
    
    # Ensure numeric
    df['wa_factor'] = df['total_written_bytes'] / (df['updates'] * 100.0)
    
    # Plotting Grouped Bar Chart
    plt.figure(figsize=(12, 8))
    sns.set_style("whitegrid")
    
    # Font setup for Chinese support
    plt.rcParams['font.sans-serif'] = ['WenQuanYi Micro Hei', 'AR PL UMing CN', 'SimHei', 'DejaVu Sans']
    plt.rcParams['axes.unicode_minus'] = False # Solve minus sign display issue

    # Create the comparison plot
    # Convert bytes to MB for better readability
    df['total_written_mb'] = df['total_written_bytes'] / (1024 * 1024)
    
    # Custom color palette: High contrast (Red vs Blue)
    custom_palette = {"BASELINE": "#d62728", "OPTIMIZED": "#1f77b4"}
    
    ax = sns.barplot(x='updates', y='total_written_mb', hue='mode', data=df, palette=custom_palette, edgecolor="black")
    
    # Labels and Title (Pure Chinese)
    plt.title('写放大对比：分层合并 vs 全量合并', fontsize=29, pad=20, fontweight='bold')
    plt.xlabel('更新操作次数', fontsize=20, fontweight='bold')
    plt.ylabel('总写入量 (MB)', fontsize=20, fontweight='bold')
    
    # Legend: remove title, use Chinese labels
    # Note: hue order is determined by data occurrence or alphabet, usually BASELINE then OPTIMIZED.
    # To be safe, we explicitly map the legend labels based on the hue categories.
    h, l = ax.get_legend_handles_labels()
    # Map labels: BASELINE -> 全量合并, OPTIMIZED -> 分层合并
    new_labels = ["全量合并 (Baseline)" if txt == "BASELINE" else "分层合并 (Optimized)" for txt in l]
    # Actually user wanted NO English.
    new_labels_clean = ["全量合并" if "BASELINE" in txt else "分层合并" for txt in l]
    
    plt.legend(h, new_labels_clean, title='合并策略', title_fontsize='13', fontsize='12')

    # Add value annotations on top of bars
    for container in ax.containers:
        ax.bar_label(container, fmt='%.1f', padding=3, fontsize=20)

    plt.yscale('linear') # Bar chart usually uses linear scale
    plt.grid(True, axis='y', linestyle='--', alpha=0.5)
    
    output_image = 'write_amplification_histogram.png'
    plt.savefig(output_image, dpi=300, bbox_inches='tight')
    print(f"Comparison histogram saved to {output_image}")
else:
    print("No results file found.")
