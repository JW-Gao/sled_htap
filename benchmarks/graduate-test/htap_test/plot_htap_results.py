#!/usr/bin/env python3
"""
HTAP测试结果可视化脚本
读取CSV文件并生成带中文标签的性能对比图表
中文字体: WenQuanYi Micro Hei
"""

import pandas as pd
import matplotlib.pyplot as plt
import matplotlib
import numpy as np
import sys
import os

# 设置中文字体为 WenQuanYi Micro Hei
matplotlib.rcParams['font.sans-serif'] = ['WenQuanYi Micro Hei', 'DejaVu Sans']
matplotlib.rcParams['axes.unicode_minus'] = False  # 解决负号显示问题


def load_results(csv_file: str) -> pd.DataFrame:
    """加载CSV测试结果"""
    if not os.path.exists(csv_file):
        raise FileNotFoundError(f"找不到结果文件: {csv_file}")
    
    df = pd.read_csv(csv_file, encoding='utf-8')
    print(f"成功加载 {len(df)} 条测试结果")
    return df


def plot_execution_time_comparison(df: pd.DataFrame, output_dir: str = "."):
    """
    绘制执行时间对比图（基线 vs 优化版本）
    按负载类型分组
    """
    fig, axes = plt.subplots(2, 3, figsize=(18, 12))
    fig.suptitle('HTAP混合负载测试 - 执行时间对比', fontsize=16, fontweight='bold')
    
    workload_types = {
        'read_intensive': '读密集型',
        'balanced': '均衡型',
        'write_intensive': '写密集型'
    }
    
    for idx, (workload_key, workload_name) in enumerate(workload_types.items()):
        # 窄表
        ax_narrow = axes[0, idx]
        df_narrow = df[(df['负载类型'] == workload_key) & (df['表类型'] == '窄表')]
        df_narrow = df_narrow.sort_values('数据访问比例')
        
        if not df_narrow.empty:
            x = df_narrow['数据访问比例'] * 100
            y_baseline = df_narrow['基线时间(s)']
            y_optimized = df_narrow['优化时间(s)']
            
            width = 3
            ax_narrow.bar(x - width/2, y_baseline, width, label='Baseline', alpha=0.8, color='#e74c3c')
            ax_narrow.bar(x + width/2, y_optimized, width, label='DSP-Tree', alpha=0.8, color='#2ecc71')
        
        ax_narrow.set_xlabel('AP数据访问比例 (%)', fontsize=11)
        ax_narrow.set_ylabel('执行时间 (s)', fontsize=11)
        ax_narrow.set_title(f'窄表 - {workload_name}', fontsize=12, fontweight='bold')
        ax_narrow.legend()
        ax_narrow.grid(True, alpha=0.3, linestyle='--')
        
        # 宽表
        ax_wide = axes[1, idx]
        df_wide = df[(df['负载类型'] == workload_key) & (df['表类型'] == '宽表')]
        df_wide = df_wide.sort_values('数据访问比例')
        
        if not df_wide.empty:
            x = df_wide['数据访问比例'] * 100
            y_baseline = df_wide['基线时间(s)']
            y_optimized = df_wide['优化时间(s)']
            
            ax_wide.bar(x - width/2, y_baseline, width, label='Baseline', alpha=0.8, color='#e74c3c')
            ax_wide.bar(x + width/2, y_optimized, width, label='DSP-Tree', alpha=0.8, color='#2ecc71')
        
        ax_wide.set_xlabel('AP数据访问比例 (%)', fontsize=11)
        ax_wide.set_ylabel('执行时间 (s)', fontsize=11)
        ax_wide.set_title(f'宽表 - {workload_name}', fontsize=12, fontweight='bold')
        ax_wide.legend()
        ax_wide.grid(True, alpha=0.3, linestyle='--')
    
    plt.tight_layout()
    output_file = os.path.join(output_dir, 'htap_execution_time_comparison.png')
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"已保存: {output_file}")
    plt.close()


def plot_speedup_heatmap(df: pd.DataFrame, output_dir: str = "."):
    """绘制加速比热图"""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 6))
    fig.suptitle('HTAP混合负载测试 - 优化加速比', fontsize=16, fontweight='bold')
    
    # 窄表热图
    df_narrow = df[df['表类型'] == '窄表'].copy()
    pivot_narrow = df_narrow.pivot_table(
        values='加速比',
        index='负载类型',
        columns='数据访问比例',
        aggfunc='mean'
    )
    
    # 调整行索引顺序和标签
    workload_labels = {'read_intensive': '读密集型', 'balanced': '均衡型', 'write_intensive': '写密集型'}
    pivot_narrow = pivot_narrow.rename(index=workload_labels)
    pivot_narrow = pivot_narrow.reindex(['读密集型', '均衡型', '写密集型'])
    
    # 调整列标签
    pivot_narrow.columns = [f'{int(c*100)}%' for c in pivot_narrow.columns]
    
    im1 = ax1.imshow(pivot_narrow.values, cmap='RdYlGn', aspect='auto', vmin=0.5)
    ax1.set_xticks(range(len(pivot_narrow.columns)))
    ax1.set_xticklabels(pivot_narrow.columns)
    ax1.set_yticks(range(len(pivot_narrow.index)))
    ax1.set_yticklabels(pivot_narrow.index)
    ax1.set_xlabel('AP数据访问比例', fontsize=12)
    ax1.set_ylabel('负载类型', fontsize=12)
    ax1.set_title('窄表 (30列)', fontsize=13, fontweight='bold')
    
    # 添加数值标注
    for i in range(len(pivot_narrow.index)):
        for j in range(len(pivot_narrow.columns)):
            value = pivot_narrow.values[i, j]
            if not np.isnan(value):
                text = ax1.text(j, i, f'{value:.2f}x', ha="center", va="center", 
                               color="white" if value > 1.2 else "black", fontsize=10, fontweight='bold')
    
    plt.colorbar(im1, ax=ax1, label='加速比')
    
    # 宽表热图
    df_wide = df[df['表类型'] == '宽表'].copy()
    pivot_wide = df_wide.pivot_table(
        values='加速比',
        index='负载类型',
        columns='数据访问比例',
        aggfunc='mean'
    )
    
    pivot_wide = pivot_wide.rename(index=workload_labels)
    pivot_wide = pivot_wide.reindex(['读密集型', '均衡型', '写密集型'])
    pivot_wide.columns = [f'{int(c*100)}%' for c in pivot_wide.columns]
    
    im2 = ax2.imshow(pivot_wide.values, cmap='RdYlGn', aspect='auto', vmin=0.5)
    ax2.set_xticks(range(len(pivot_wide.columns)))
    ax2.set_xticklabels(pivot_wide.columns)
    ax2.set_yticks(range(len(pivot_wide.index)))
    ax2.set_yticklabels(pivot_wide.index)
    ax2.set_xlabel('AP数据访问比例', fontsize=12)
    ax2.set_ylabel('负载类型', fontsize=12)
    ax2.set_title('宽表 (70列)', fontsize=13, fontweight='bold')
    
    for i in range(len(pivot_wide.index)):
        for j in range(len(pivot_wide.columns)):
            value = pivot_wide.values[i, j]
            if not np.isnan(value):
                text = ax2.text(j, i, f'{value:.2f}x', ha="center", va="center",
                               color="white" if value > 1.2 else "black", fontsize=10, fontweight='bold')
    
    plt.colorbar(im2, ax=ax2, label='加速比')
    
    plt.tight_layout()
    output_file = os.path.join(output_dir, 'htap_speedup_heatmap.png')
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"已保存: {output_file}")
    plt.close()


def plot_performance_improvement(df: pd.DataFrame, output_dir: str = "."):
    """绘制性能提升百分比图"""
    fig, axes = plt.subplots(1, 2, figsize=(14, 6))
    fig.suptitle('HTAP混合负载测试 - 性能提升百分比', fontsize=16, fontweight='bold')
    
    # 按表类型分组
    for idx, table_type in enumerate(['窄表', '宽表']):
        ax = axes[idx]
        df_table = df[df['表类型'] == table_type].copy()
        
        # 按数据访问比例分组
        data_ratios = sorted(df_table['数据访问比例'].unique())
        
        workload_types = {
            'read_intensive': '读密集型',
            'balanced': '均衡型',
            'write_intensive': '写密集型'
        }
        
        x = np.arange(len(data_ratios))
        width = 0.25
        
        colors = {'read_intensive': '#3498db', 'balanced': '#9b59b6', 'write_intensive': '#e67e22'}
        
        for i, (workload_key, workload_name) in enumerate(workload_types.items()):
            df_workload = df_table[df_table['负载类型'] == workload_key].sort_values('数据访问比例')
            if not df_workload.empty:
                improvements = df_workload['性能提升(%)'].values
                
                offset = (i - 1) * width
                ax.bar(x + offset, improvements, width, label=workload_name, 
                       alpha=0.8, color=colors[workload_key])
        
        ax.set_xlabel('AP数据访问比例', fontsize=12)
        ax.set_ylabel('性能提升 (%)', fontsize=12)
        ax.set_title(f'{table_type} ({"30" if table_type == "窄表" else "70"}列)', 
                     fontsize=13, fontweight='bold')
        ax.set_xticks(x)
        ax.set_xticklabels([f'{int(r*100)}%' for r in data_ratios])
        ax.legend()
        ax.grid(True, alpha=0.3, linestyle='--', axis='y')
        ax.axhline(y=0, color='black', linestyle='-', linewidth=0.8)
    
    plt.tight_layout()
    output_file = os.path.join(output_dir, 'htap_performance_improvement.png')
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"已保存: {output_file}")
    plt.close()


def plot_workload_comparison(df: pd.DataFrame, output_dir: str = "."):
    """绘制不同负载类型的对比（直方图）"""
    fig, axes = plt.subplots(1, 2, figsize=(14, 6))
    fig.suptitle('HTAP混合负载测试 - 不同负载类型性能对比', fontsize=16, fontweight='bold')
    
    workload_types = {
        'read_intensive': '读密集型',
        'balanced': '均衡型',
        'write_intensive': '写密集型'
    }
    
    colors = {'read_intensive': '#3498db', 'balanced': '#9b59b6', 'write_intensive': '#e67e22'}
    
    for idx, table_type in enumerate(['窄表', '宽表']):
        ax = axes[idx]
        df_table = df[df['表类型'] == table_type]
        
        # 按数据访问比例分组
        data_ratios = sorted(df_table['数据访问比例'].unique())
        x = np.arange(len(data_ratios))
        width = 0.25
        
        for i, (workload_key, workload_name) in enumerate(workload_types.items()):
            df_workload = df_table[df_table['负载类型'] == workload_key].sort_values('数据访问比例')
            
            if not df_workload.empty:
                y = df_workload['优化时间(s)'].values
                offset = (i - 1) * width
                ax.bar(x + offset, y, width, label=workload_name, 
                       alpha=0.8, color=colors[workload_key])
        
        ax.set_xlabel('AP数据访问比例', fontsize=12)
        ax.set_ylabel('优化版本执行时间 (秒)', fontsize=12)
        ax.set_title(f'{table_type} ({"30" if table_type == "窄表" else "70"}列)',
                    fontsize=13, fontweight='bold')
        ax.set_xticks(x)
        ax.set_xticklabels([f'{int(r*100)}%' for r in data_ratios])
        ax.legend()
        ax.grid(True, alpha=0.3, linestyle='--', axis='y')
    
    plt.tight_layout()
    output_file = os.path.join(output_dir, 'htap_workload_comparison.png')
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"已保存: {output_file}")
    plt.close()


def generate_summary_table(df: pd.DataFrame, output_dir: str = "."):
    """生成性能摘要表格图"""
    fig, ax = plt.subplots(figsize=(16, 10))
    ax.axis('tight')
    ax.axis('off')
    
    # 准备摘要数据
    summary_data = []
    for _, row in df.iterrows():
        summary_data.append([
            row['场景'],
            f"{row['基线时间(s)']:.2f}",
            f"{row['优化时间(s)']:.2f}",
            f"{row['加速比']:.2f}x" if row['加速比'] > 0 else "N/A",
            f"{row['性能提升(%)']:.2f}%" if row['性能提升(%)'] > 0 else "N/A"
        ])
    
    headers = ['测试场景', '基线时间(s)', '优化时间(s)', '加速比', '性能提升(%)']
    
    table = ax.table(cellText=summary_data, colLabels=headers, 
                     cellLoc='center', loc='center',
                     colWidths=[0.3, 0.15, 0.15, 0.15, 0.15])
    
    table.auto_set_font_size(False)
    table.set_fontsize(9)
    table.scale(1, 2)
    
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
    
    plt.title('HTAP混合负载测试 - 详细结果摘要', fontsize=14, fontweight='bold', pad=20)
    
    output_file = os.path.join(output_dir, 'htap_summary_table.png')
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"已保存: {output_file}")
    plt.close()


def main():
    """主函数"""
    if len(sys.argv) < 2:
        print("用法: python plot_htap_results.py <csv_file> [output_dir]")
        print("示例: python plot_htap_results.py htap_test_results_20260123_105700.csv")
        sys.exit(1)
    
    csv_file = sys.argv[1]
    output_dir = sys.argv[2] if len(sys.argv) > 2 else "."
    
    # 创建输出目录
    if not os.path.exists(output_dir):
        os.makedirs(output_dir)
    
    print("="*80)
    print("HTAP测试结果可视化")
    print("="*80)
    print(f"输入文件: {csv_file}")
    print(f"输出目录: {output_dir}")
    print(f"中文字体: WenQuanYi Micro Hei")
    print("="*80)
    
    # 加载数据
    df = load_results(csv_file)
    
    # 生成各种图表
    print("\n开始生成图表...")
    
    print("\n1. 生成执行时间对比图...")
    plot_execution_time_comparison(df, output_dir)
    
    print("\n2. 生成加速比热图...")
    plot_speedup_heatmap(df, output_dir)
    
    print("\n3. 生成性能提升百分比图...")
    plot_performance_improvement(df, output_dir)
    
    print("\n4. 生成负载类型对比图...")
    plot_workload_comparison(df, output_dir)
    
    print("\n5. 生成摘要表格...")
    generate_summary_table(df, output_dir)
    
    print("\n" + "="*80)
    print("所有图表生成完成！")
    print(f"请查看输出目录: {os.path.abspath(output_dir)}")
    print("="*80)
    
    # 列出生成的文件
    print("\n生成的图表文件:")
    generated_files = [
        'htap_execution_time_comparison.png',
        'htap_speedup_heatmap.png',
        'htap_performance_improvement.png',
        'htap_workload_comparison.png',
        'htap_summary_table.png'
    ]
    for filename in generated_files:
        filepath = os.path.join(output_dir, filename)
        if os.path.exists(filepath):
            print(f"  ✓ {filename}")
        else:
            print(f"  ✗ {filename} (未生成)")


if __name__ == "__main__":
    main()
