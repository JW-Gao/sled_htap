#!/usr/bin/env python3
"""
HTAP列选择性能测试 - 可视化脚本
生成展示列选择性对性能影响的图表
"""

import pandas as pd
import matplotlib.pyplot as plt
import matplotlib
import numpy as np
import sys
import os
import time

# 设置中文字体
matplotlib.rcParams['font.sans-serif'] = ['WenQuanYi Micro Hei', 'DejaVu Sans']
matplotlib.rcParams['axes.unicode_minus'] = False


def load_results(csv_file: str) -> pd.DataFrame:
    """加载CSV测试结果"""
    if not os.path.exists(csv_file):
        raise FileNotFoundError(f"找不到结果文件: {csv_file}")
    
    df = pd.read_csv(csv_file, encoding='utf-8')
    print(f"成功加载 {len(df)} 条测试结果")
    return df


def plot_column_impact(df: pd.DataFrame, output_dir: str = "."):
    """绘制列选择性对执行时间的影响（2x2子图，合并为一张图保存）"""

    # 4种负载配置，按 (row, col) 排列到 2x2 子图
    configs = [
        ("read_intensive",  0.2, "读密集-20%数据 (OLAP 70%, OLTP 30%)", 0, 0),
        ("read_intensive",  0.8, "读密集-80%数据 (OLAP 70%, OLTP 30%)", 0, 1),
        ("write_intensive", 0.2, "写密集-20%数据 (OLAP 30%, OLTP 70%)", 1, 0),
        ("write_intensive", 0.8, "写密集-80%数据 (OLAP 30%, OLTP 70%)", 1, 1),
    ]

    fig, axes = plt.subplots(2, 2, figsize=(16, 12))
    fig.suptitle('HTAP测试 - 列选择性对执行时间的影响', fontsize=16, fontweight='bold')

    order = ['1/30', '5/30', '30/30', '1/70', '5/70', '70/70']

    for workload, data_ratio, label, row, col in configs:
        ax = axes[row, col]

        # 筛选数据
        df_subset = df[(df['负载类型'] == workload) &
                       (df['数据访问比例'] == data_ratio)].copy()

        # 创建列比例标签（格式：1/30, 5/30, 30/30, 1/70, 5/70, 70/70）
        df_subset['列比例标签'] = df_subset.apply(
            lambda r: f"{r['读取列数']}/{r['总列数']}", axis=1
        )

        # 按照指定顺序排序
        df_subset['order'] = df_subset['列比例标签'].apply(
            lambda x: order.index(x) if x in order else 999
        )
        df_subset = df_subset.sort_values('order')

        # 准备绘图数据
        x = np.arange(len(df_subset))
        width = 0.35

        baseline_times  = df_subset['基线时间(s)'].values
        optimized_times = df_subset['优化时间(s)'].values
        tick_labels     = df_subset['列比例标签'].values

        # 绘制分组柱状图
        ax.bar(x - width/2, baseline_times,  width, label='Baseline', alpha=0.8, color='#e74c3c')
        ax.bar(x + width/2, optimized_times, width, label='DSP-Tree', alpha=0.8, color='#3498db')

        ax.set_xlabel('读取列数/总列数', fontsize=12)
        ax.set_ylabel('执行时间 (s)',   fontsize=12)
        ax.set_title(label, fontsize=13, fontweight='bold')
        ax.set_xticks(x)
        ax.set_xticklabels(tick_labels, fontsize=10)
        ax.legend(fontsize=10)
        ax.grid(True, alpha=0.3, linestyle='--', axis='y')

    plt.tight_layout()
    output_file = os.path.join(output_dir, 'htap_column_impact.png')
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"已保存: {output_file}")
    plt.close()


def plot_column_speedup_bars(df: pd.DataFrame, output_dir: str = "."):
    """绘制不同列数下的加速比柱状图"""
    fig, axes = plt.subplots(1, 2, figsize=(14, 6))
    fig.suptitle('HTAP测试 - 列数对加速比的影响', fontsize=16, fontweight='bold')
    
    for idx, table_type in enumerate(['窄表', '宽表']):
        ax = axes[idx]
        df_table = df[df['表类型'] == table_type]
        
        # 按读取列数分组，计算平均加速比
        grouped = df_table.groupby('读取列数')['加速比'].mean().sort_index()
        
        x = np.arange(len(grouped))
        colors = ['#2ecc71', '#f39c12', '#e74c3c']
        
        bars = ax.bar(x, grouped.values, alpha=0.8, color=colors[:len(grouped)])
        
        # 添加数值标签
        for i, (bar, val) in enumerate(zip(bars, grouped.values)):
            height = bar.get_height()
            ax.text(bar.get_x() + bar.get_width()/2., height,
                    f'{val:.2f}x', ha='center', va='bottom', fontsize=10, fontweight='bold')
        
        ax.set_xlabel('读取列数', fontsize=12)
        ax.set_ylabel('平均加速比', fontsize=12)
        total_cols = 30 if table_type == '窄表' else 70
        ax.set_title(f'{table_type} ({total_cols}列)', fontsize=13, fontweight='bold')
        ax.set_xticks(x)
        ax.set_xticklabels([f'{int(c)}' for c in grouped.index])
        ax.axhline(y=1.0, color='gray', linestyle='--', linewidth=1, alpha=0.5)
        ax.grid(True, alpha=0.3, linestyle='--', axis='y')
        ax.set_ylim(0, max(grouped.values) * 1.2)
    
    plt.tight_layout()
    output_file = os.path.join(output_dir, 'htap_column_speedup_bars.png')
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"已保存: {output_file}")
    plt.close()


def plot_execution_time_by_columns(df: pd.DataFrame, output_dir: str = "."):
    """绘制不同列数下的执行时间对比"""
    fig, axes = plt.subplots(2, 2, figsize=(14, 10))
    fig.suptitle('HTAP测试 - 列数对执行时间的影响', fontsize=16, fontweight='bold')
    
    configs = [
        ("read_intensive", 0.2, "读密集-20%", 0, 0),
        ("read_intensive", 0.8, "读密集-80%", 0, 1),
        ("write_intensive", 0.2, "写密集-20%", 1, 0),
        ("write_intensive", 0.8, "写密集-80%", 1, 1),
    ]
    
    for workload, data_ratio, label, row, col in configs:
        ax = axes[row, col]
        
        for table_type in ['窄表', '宽表']:
            df_subset = df[(df['负载类型'] == workload) &
                           (df['数据访问比例'] == data_ratio) &
                           (df['表类型'] == table_type)].sort_values('读取列数')
            
            x = df_subset['读取列数'].values
            y_baseline = df_subset['基线时间(s)'].values
            y_optimized = df_subset['优化时间(s)'].values
            
            marker = 'o' if table_type == '窄表' else 's'
            color_base = '#e74c3c' if table_type == '窄表' else '#c0392b'
            color_opt = '#3498db' if table_type == '窄表' else '#2980b9'
            
            ax.plot(x, y_baseline, marker=marker, linewidth=2, markersize=6,
                    label=f'{table_type}-基线', color=color_base, alpha=0.7, linestyle='--')
            ax.plot(x, y_optimized, marker=marker, linewidth=2, markersize=6,
                    label=f'{table_type}-优化', color=color_opt, alpha=0.8)
        
        ax.set_xlabel('读取列数', fontsize=11)
        ax.set_ylabel('执行时间 (s)', fontsize=11)
        ax.set_title(label, fontsize=12, fontweight='bold')
        ax.legend(fontsize=8, ncol=2)
        ax.grid(True, alpha=0.3, linestyle='--')
    
    plt.tight_layout()
    output_file = os.path.join(output_dir, 'htap_column_execution_time.png')
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"已保存: {output_file}")
    plt.close()


def plot_column_heatmap(df: pd.DataFrame, output_dir: str = "."):
    """绘制列选择性能热图"""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))
    fig.suptitle('HTAP测试 - 列选择加速比热图', fontsize=16, fontweight='bold')
    
    for idx, table_type in enumerate(['窄表', '宽表']):
        ax = ax1 if idx == 0 else ax2
        df_table = df[df['表类型'] == table_type]
        
        # 创建透视表：行=负载配置，列=读取列数
        df_table['配置'] = df_table['负载类型'].map({
            'read_intensive': '读密集'
        }) + '-' + (df_table['数据访问比例'] * 100).astype(int).astype(str) + '%'
        
        pivot = df_table.pivot_table(
            values='加速比',
            index='配置',
            columns='读取列数',
            aggfunc='mean'
        )
        
        # 绘制热图
        im = ax.imshow(pivot.values, cmap='RdYlGn', aspect='auto', vmin=0.5, vmax=2.5)
        
        ax.set_xticks(range(len(pivot.columns)))
        ax.set_xticklabels([f'{int(c)}列' for c in pivot.columns])
        ax.set_yticks(range(len(pivot.index)))
        ax.set_yticklabels(pivot.index)
        ax.set_xlabel('读取列数', fontsize=12)
        ax.set_ylabel('负载配置', fontsize=12)
        
        total_cols = 30 if table_type == '窄表' else 70
        ax.set_title(f'{table_type} ({total_cols}列)', fontsize=13, fontweight='bold')
        
        # 添加数值标注
        for i in range(len(pivot.index)):
            for j in range(len(pivot.columns)):
                value = pivot.values[i, j]
                if not np.isnan(value):
                    color = 'white' if value > 1.5 or value < 0.8 else 'black'
                    ax.text(j, i, f'{value:.2f}x', ha="center", va="center",
                           color=color, fontsize=9, fontweight='bold')
        
        plt.colorbar(im, ax=ax, label='加速比')
    
    plt.tight_layout()
    output_file = os.path.join(output_dir, 'htap_column_heatmap.png')
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"已保存: {output_file}")
    plt.close()


def generate_summary_table(df: pd.DataFrame, output_dir: str = "."):
    """生成结果摘要表"""
    fig, ax = plt.subplots(figsize=(16, 10))
    ax.axis('tight')
    ax.axis('off')
    
    summary_data = []
    for _, row in df.iterrows():
        summary_data.append([
            row['场景'],
            f"{row['读取列数']}/{row['总列数']}",
            f"{row['列选择比例']*100:.1f}%",
            f"{row['基线时间(s)']:.2f}",
            f"{row['优化时间(s)']:.2f}",
            f"{row['加速比']:.2f}x",
            f"{row['性能提升(%)']:.2f}%"
        ])
    
    headers = ['场景', '列数', '选择比例', '基线时间(s)', '优化时间(s)', '加速比', '性能提升']
    
    table = ax.table(cellText=summary_data, colLabels=headers,
                     cellLoc='center', loc='center',
                     colWidths=[0.25, 0.1, 0.1, 0.15, 0.15, 0.1, 0.15])
    
    table.auto_set_font_size(False)
    table.set_fontsize(8)
    table.scale(1, 1.8)
    
    # 设置表头样式
    for i in range(len(headers)):
        cell = table[(0, i)]
        cell.set_facecolor('#34495e')
        cell.set_text_props(weight='bold', color='white')
    
    # 设置行颜色交替
    for i in range(1, len(summary_data) + 1):
        for j in range(len(headers)):
            cell = table[(i, j)]
            if i % 2 == 0:
                cell.set_facecolor('#ecf0f1')
            else:
                cell.set_facecolor('white')
    
    plt.title('HTAP列选择性能测试 - 详细结果摘要', fontsize=14, fontweight='bold', pad=20)
    
    output_file = os.path.join(output_dir, 'htap_column_summary_table.png')
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"已保存: {output_file}")
    plt.close()


def main():
    time.sleep(60)
    """主函数"""
    if len(sys.argv) < 2:
        print("用法: python plot_column_results.py <csv_file> [output_dir]")
        print("示例: python plot_column_results.py htap_column_results_estimated_20260205_155535.csv")
        sys.exit(1)
    
    csv_file = sys.argv[1]
    output_dir = sys.argv[2] if len(sys.argv) > 2 else "."
    
    if not os.path.exists(output_dir):
        os.makedirs(output_dir)
    
    print("="*80)
    print("HTAP列选择性能测试 - 结果可视化")
    print("="*80)
    print(f"输入文件: {csv_file}")
    print(f"输出目录: {output_dir}")
    print("="*80)
    
    df = load_results(csv_file)
    
    print("\n开始生成图表...\n")
    
    print("1. 生成列选择性影响图...")
    plot_column_impact(df, output_dir)
    
    print("2. 生成加速比柱状图...")
    plot_column_speedup_bars(df, output_dir)
    
    print("3. 生成执行时间对比图...")
    plot_execution_time_by_columns(df, output_dir)
    
    print("4. 生成加速比热图...")
    plot_column_heatmap(df, output_dir)
    
    print("5. 生成摘要表格...")
    generate_summary_table(df, output_dir)
    
    print("\n" + "="*80)
    print("所有图表生成完成！")
    print(f"请查看输出目录: {os.path.abspath(output_dir)}")
    print("="*80)


if __name__ == "__main__":
    main()
