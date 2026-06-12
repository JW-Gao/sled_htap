import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np
import os

# Set style
sns.set_theme(style="whitegrid")
plt.rcParams['font.sans-serif'] = ['WenQuanYi Micro Hei'] # Use SimHei for Chinese characters if needed, or remove for English
plt.rcParams['axes.unicode_minus'] = False

def plot_benchmark_results(csv_path):
    if not os.path.exists(csv_path):
        print(f"Error: {csv_path} not found.")
        return

    df = pd.read_csv(csv_path)
    
    # Clean data: Replace 0s with NaN to avoid plotting empty bars if data is missing
    df.replace(0, np.nan, inplace=True)

    # Define custom palette to highlight blp-tree
    # User feedback: No Gray, No Green.
    # bf-tree: Blue, fjall: Orange, blp-tree: Red (Highlight)
    custom_palette = {
        "Bf-tree": "#96c9e6",  # Blue
        "Disco-LSM": "#2c99b7",    # Orange
        "Blp-tree": "#FF6600"  # Red - Highlight
    }

    # Plot Throughput
    plt.figure(figsize=(10, 6))
    ax1 = sns.barplot(
        data=df, 
        x="Workload", 
        y="Throughput(ops/sec)", 
        hue="Engine", 
        palette=custom_palette
    )
    plt.title("YCSB 吞吐量对比", fontsize=14)
    plt.ylabel("吞吐量 (Ops)", fontsize=12)
    plt.xlabel("负载类型 (Workload)", fontsize=12)
    plt.legend(title=None) # Start of legend title removal
    
    # Remove value labels as requested
    # for container in ax1.containers:
    #     ax1.bar_label(container, fmt='%.0f', padding=3)
        
    plt.tight_layout()
    plt.savefig("ycsb_throughput.png", dpi=300)
    print("Saved ycsb_throughput.png")

    # Plot Latency P99
    plt.figure(figsize=(10, 6))
    ax2 = sns.barplot(
        data=df, 
        x="Workload", 
        y="Latency_P99(us)", 
        hue="Engine", 
        palette=custom_palette
    )
    plt.title("YCSB P99 延迟对比", fontsize=14)
    plt.ylabel("P99 延迟 (us)", fontsize=12)
    plt.xlabel("负载类型 (Workload)", fontsize=12)
    plt.legend(title=None) # Start of legend title removal
    
    # Remove value labels as requested
    # for container in ax2.containers:
    #     ax2.bar_label(container, fmt='%.0f', padding=3)

    plt.tight_layout()
    plt.savefig("ycsb_latency.png", dpi=300)
    print("Saved ycsb_latency.png")

if __name__ == "__main__":
    plot_benchmark_results("benchmark_results_manual.csv")
