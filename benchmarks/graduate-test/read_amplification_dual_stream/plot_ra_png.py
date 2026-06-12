import csv
import matplotlib.pyplot as plt
import os

def plot_png():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    data_file = os.path.join(script_dir, 'ra_results.csv')
    
    if not os.path.exists(data_file):
        print(f"Error: {data_file} not found. Please run run_mock_experiment.py first.")
        return

    # Data structure: workload -> {'deltas': [], 'ratios': []}
    data = {}
    
    with open(data_file, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            wl = row['workload']
            delta = float(row['delta'])
            ratio = float(row['ra_ratio'])
            
            if wl not in data:
                data[wl] = {'deltas': [], 'ratios': []}
            
            data[wl]['deltas'].append(delta)
            data[wl]['ratios'].append(ratio)

    # 设置大字体以保证论文中的可读性
    plt.rcParams.update({
        'font.size': 16,
        'axes.labelsize': 16,
        'axes.titlesize': 18,
        'xtick.labelsize': 15,
        'ytick.labelsize': 15,
        'legend.fontsize': 14,
        'font.sans-serif': ['WenQuanYi Micro Hei', 'SimHei', 'Noto Sans CJK SC', 'Arial Unicode MS'],
        'axes.unicode_minus': False
    })

    plt.figure(figsize=(9, 6))

    # 样式配置
    styles = [
        {'marker': 'o', 'color': '#d62728', 'linestyle': '-', 'label': 'TP:AP = 4:1'},
        {'marker': 's', 'color': '#2ca02c', 'linestyle': '--', 'label': 'TP:AP = 1:1'},
        {'marker': '^', 'color': '#1f77b4', 'linestyle': '-.', 'label': 'TP:AP = 1:4'}
    ]

    for i, (wl, w_data) in enumerate(data.items()):
        style = styles[i % len(styles)]
        
        plt.plot(w_data['deltas'], w_data['ratios'], 
                 label=style['label'], 
                 color=style['color'], 
                 marker=style['marker'], 
                 linestyle=style['linestyle'], 
                 markersize=8, 
                 linewidth=2.5, 
                 alpha=0.9)

    plt.xlabel('数据交叉分散度 (δ)')
    plt.ylabel('单数据流读放大 / 双数据流读放大')
    
    # 调大坐标轴上数字（刻度）的大小
    plt.tick_params(axis='both', which='major', labelsize=15)

    plt.grid(True, linestyle='--', alpha=0.6)
    plt.legend(loc='upper left')
    
    plt.tight_layout()

    output_file = os.path.join(script_dir, 'read_amplification_plot.png')
    plt.savefig(output_file, format='png', dpi=300)
    print(f"Saved raster plot (PNG) to {output_file}")

if __name__ == '__main__':
    plot_png()
