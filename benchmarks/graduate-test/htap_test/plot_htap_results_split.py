#!/usr/bin/env python3
"""
HTAP测试结果可视化脚本 - 拆分版
将 htap_execution_time_comparison 大图拆分为六张独立的小图：
  - 窄表_读密集型, 窄表_均衡型, 窄表_写密集型
  - 宽表_读密集型, 宽表_均衡型, 宽表_写密集型
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


def plot_single_execution_time(df: pd.DataFrame, table_type: str, workload_key: str,
                                workload_name: str, output_dir: str = "."):
    """
    绘制单个执行时间对比图（基线 vs 优化版本）
    
    Args:
        df: 完整数据集
        table_type: '窄表' 或 '宽表'
        workload_key: 'read_intensive', 'balanced', 'write_intensive'
        workload_name: 中文负载名称
        output_dir: 输出目录
    """
    fig, ax = plt.subplots(figsize=(6, 4.5))
    
    df_filtered = df[(df['负载类型'] == workload_key) & (df['表类型'] == table_type)]
    df_filtered = df_filtered.sort_values('数据访问比例')
    
    if not df_filtered.empty:
        x = df_filtered['数据访问比例'] * 100
        y_baseline = df_filtered['基线时间(s)']
        y_optimized = df_filtered['优化时间(s)']
        
        width = 3
        ax.bar(x - width/2, y_baseline, width, label='Baseline', alpha=0.8, color='#e74c3c')
        ax.bar(x + width/2, y_optimized, width, label='DSP-Tree', alpha=0.8, color='#2ecc71')
    
    ax.set_xlabel('AP数据访问比例 (%)', fontsize=11)
    ax.set_ylabel('执行时间 (s)', fontsize=11)
    ax.set_title(f'{table_type} - {workload_name}', fontsize=12, fontweight='bold')
    ax.legend()
    ax.grid(True, alpha=0.3, linestyle='--')
    
    plt.tight_layout()
    filename = f'htap_exec_time_{table_type}_{workload_key}.png'
    output_file = os.path.join(output_dir, filename)
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"  已保存: {output_file}")
    plt.close()


def plot_all_execution_time_split(df: pd.DataFrame, output_dir: str = "."):
    """
    将原来的 2x3 大图拆分为六张独立小图
    """
    workload_types = {
        'read_intensive': '读密集型',
        'balanced': '均衡型',
        'write_intensive': '写密集型'
    }
    
    table_types = ['窄表', '宽表']
    
    for table_type in table_types:
        for workload_key, workload_name in workload_types.items():
            plot_single_execution_time(df, table_type, workload_key, workload_name, output_dir)


def main():
    """主函数"""
    if len(sys.argv) < 2:
        print("用法: python plot_htap_results_split.py <csv_file> [output_dir]")
        print("示例: python plot_htap_results_split.py htap_test_results_20260123_105700.csv")
        sys.exit(1)
    
    csv_file = sys.argv[1]
    output_dir = sys.argv[2] if len(sys.argv) > 2 else "."
    
    # 创建输出目录
    if not os.path.exists(output_dir):
        os.makedirs(output_dir)
    
    print("=" * 80)
    print("HTAP测试结果可视化 - 拆分版 (执行时间对比)")
    print("=" * 80)
    print(f"输入文件: {csv_file}")
    print(f"输出目录: {output_dir}")
    print(f"中文字体: WenQuanYi Micro Hei")
    print("=" * 80)
    
    # 加载数据
    df = load_results(csv_file)
    
    # 生成六张拆分图
    print("\n开始生成六张执行时间对比图...")
    plot_all_execution_time_split(df, output_dir)
    
    print("\n" + "=" * 80)
    print("所有图表生成完成！")
    print(f"请查看输出目录: {os.path.abspath(output_dir)}")
    print("=" * 80)
    
    # 列出生成的文件
    print("\n生成的图表文件:")
    generated_files = [
        'htap_exec_time_窄表_read_intensive.png',
        'htap_exec_time_窄表_balanced.png',
        'htap_exec_time_窄表_write_intensive.png',
        'htap_exec_time_宽表_read_intensive.png',
        'htap_exec_time_宽表_balanced.png',
        'htap_exec_time_宽表_write_intensive.png',
    ]
    for filename in generated_files:
        filepath = os.path.join(output_dir, filename)
        if os.path.exists(filepath):
            print(f"  ✓ {filename}")
        else:
            print(f"  ✗ {filename} (未生成)")


if __name__ == "__main__":
    main()
