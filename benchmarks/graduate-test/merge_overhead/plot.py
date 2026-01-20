import csv
import matplotlib.pyplot as plt

thresholds = []
write_ops = []
read_ops = []
db_sizes = []

with open('merge_overhead_results.csv', 'r') as f:
    reader = csv.DictReader(f)
    for row in reader:
        thresholds.append(int(row['threshold']))
        write_ops.append(float(row['write_only_qps']))
        read_ops.append(float(row['mixed_rw_qps']))
        db_sizes.append(int(row['db_size_bytes']) / 1024 / 1024) # MB

fig, ax1 = plt.subplots(figsize=(10, 6))

# Set font for Chinese characters
# Using "AR PL UMing CN" which is available and typically covers both CJK and Latin well or falls back
plt.rcParams['font.sans-serif'] = ['AR PL UMing CN', 'DejaVu Sans', 'Bitstream Vera Sans', 'sans-serif'] 
plt.rcParams['axes.unicode_minus'] = False 

color = 'tab:red'
ax1.set_xlabel('页面合并阈值 (Page Consolidation Threshold)')
ax1.set_ylabel('纯写入 QPS (Write-Only)', color=color)
ax1.plot(thresholds, write_ops, marker='o', color=color, label='纯写入 QPS')
ax1.tick_params(axis='y', labelcolor=color)

ax2 = ax1.twinx()  
color = 'tab:blue'
ax2.set_ylabel('读写混合 QPS (Mixed R/W)', color=color)  
ax2.plot(thresholds, read_ops, marker='s', color=color, linestyle='--', label='读写混合 QPS')
ax2.tick_params(axis='y', labelcolor=color)

plt.title('合并阈值对 HTAP 节点性能的影响')
# Combine legends
lines_1, labels_1 = ax1.get_legend_handles_labels()
lines_2, labels_2 = ax2.get_legend_handles_labels()
ax1.legend(lines_1 + lines_2, labels_1 + labels_2, loc='center right')

fig.tight_layout()  
plt.grid(True)
plt.savefig('benchmark_plot.png')
print("Plot saved to benchmark_plot.png")
