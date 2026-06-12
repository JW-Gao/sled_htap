import csv
import matplotlib.pyplot as plt

ops = []
latencies = []

with open('merge_impact_results.csv', 'r') as f:
    reader = csv.DictReader(f)
    for row in reader:
        ops.append(int(row['op_index']))
        latencies.append(int(row['latency_us']))

fig, ax = plt.subplots(figsize=(12, 6))

# Set font for Chinese characters
plt.rcParams['font.sans-serif'] = ['WenQuanYi Micro Hei', 'DejaVu Sans', 'Bitstream Vera Sans', 'sans-serif'] 
plt.rcParams['axes.unicode_minus'] = False 

ax.plot(ops, latencies, marker='.', linestyle='-', color='tab:purple', label='单次写入延迟 (Latency)')

ax.set_xlabel('操作序号 (Operation Index)')
ax.set_ylabel('延迟 (Latency µs)')
ax.set_title('Merge Context 写入延迟抖动分析 (Threshold=10)')

# Highlight spikes?
# Let's just let the plot show it.

plt.grid(True, which='both', linestyle='--', linewidth=0.5)
plt.legend()
plt.tight_layout()
plt.savefig('impact_plot.png')
print("Plot saved to impact_plot.png")
