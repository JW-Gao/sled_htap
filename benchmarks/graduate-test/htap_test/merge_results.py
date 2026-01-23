#!/usr/bin/env python3
"""
HTAP测试结果汇总脚本
合并所有批次的CSV结果文件
"""

import pandas as pd
import sys
import os
import glob

def merge_batch_results(pattern="htap_batch*.csv", output_file=None):
    """
    合并所有批次的测试结果
    
    Args:
        pattern: CSV文件的匹配模式
        output_file: 输出文件名，如果为None则自动生成
    """
    # 查找所有匹配的CSV文件
    batch_files = sorted(glob.glob(pattern))
    
    if not batch_files:
        print(f"错误: 没有找到匹配 '{pattern}' 的文件")
        return None
    
    print(f"找到 {len(batch_files)} 个批次文件:")
    for f in batch_files:
        print(f"  - {f}")
    
    # 读取并合并所有CSV
    dfs = []
    for file in batch_files:
        try:
            df = pd.read_csv(file, encoding='utf-8')
            dfs.append(df)
            print(f"✓ 加载 {file}: {len(df)} 条记录")
        except Exception as e:
            print(f"✗ 加载 {file} 失败: {e}")
    
    if not dfs:
        print("错误: 没有成功加载任何文件")
        return None
    
    # 合并数据
    merged_df = pd.concat(dfs, ignore_index=True)
    
    # 生成输出文件名
    if output_file is None:
        from datetime import datetime
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        output_file = f"htap_test_results_merged_{timestamp}.csv"
    
    # 保存合并结果
    merged_df.to_csv(output_file, index=False, encoding='utf-8')
    
    print(f"\n{'='*80}")
    print(f"合并完成！")
    print(f"  总记录数: {len(merged_df)}")
    print(f"  输出文件: {output_file}")
    print(f"{'='*80}")
    
    # 显示摘要
    print(f"\n数据摘要:")
    print(f"  窄表场景: {len(merged_df[merged_df['表类型']=='窄表'])} 个")
    print(f"  宽表场景: {len(merged_df[merged_df['表类型']=='宽表'])} 个")
    
    if '加速比' in merged_df.columns:
        valid_speedup = merged_df[merged_df['加速比'] > 0]['加速比']
        if len(valid_speedup) > 0:
            print(f"\n性能统计:")
            print(f"  平均加速比: {valid_speedup.mean():.2f}x")
            print(f"  最大加速比: {valid_speedup.max():.2f}x")
            print(f"  最小加速比: {valid_speedup.min():.2f}x")
    
    return output_file


def main():
    """主函数"""
    if len(sys.argv) > 1:
        pattern = sys.argv[1]
    else:
        pattern = "htap_batch*.csv"
    
    output_file = sys.argv[2] if len(sys.argv) > 2 else None
    
    print("="*80)
    print("HTAP测试结果汇总工具")
    print("="*80)
    
    merged_file = merge_batch_results(pattern, output_file)
    
    if merged_file:
        print(f"\n下一步: 生成可视化图表")
        print(f"  python plot_htap_results.py {merged_file}")


if __name__ == "__main__":
    main()
