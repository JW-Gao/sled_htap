#!/usr/bin/env python3
"""
HTAP混合负载测试启动脚本
自动运行所有24个测试场景并生成CSV结果文件
"""

import subprocess
import csv
import time
import os
import re
from datetime import datetime
from typing import Dict, List, Tuple, Optional

# 测试配置
TOTAL_OPERATIONS = 50000  # 每个场景的总操作数
PREPOPULATE_ROWS = 100000  # 预填充行数
TABLE_NARROW_COLS = 30    # 窄表列数
TABLE_WIDE_COLS = 70      # 宽表列数

# 测试场景配置（共24个）
TEST_SCENARIOS = [
    # 格式: (表类型, 列数, 负载类型, OLAP比例, OLTP比例, AP数据访问比例, 场景标签)
    
    # 窄表场景 (12个)
    ("narrow", TABLE_NARROW_COLS, "read_intensive", 0.7, 0.3, 0.1, "窄表-读密集-10%"),
    ("narrow", TABLE_NARROW_COLS, "read_intensive", 0.7, 0.3, 0.4, "窄表-读密集-40%"),
    ("narrow", TABLE_NARROW_COLS, "read_intensive", 0.7, 0.3, 0.7, "窄表-读密集-70%"),
    ("narrow", TABLE_NARROW_COLS, "read_intensive", 0.7, 0.3, 1.0, "窄表-读密集-100%"),
    
    ("narrow", TABLE_NARROW_COLS, "balanced", 0.5, 0.5, 0.1, "窄表-均衡-10%"),
    ("narrow", TABLE_NARROW_COLS, "balanced", 0.5, 0.5, 0.4, "窄表-均衡-40%"),
    ("narrow", TABLE_NARROW_COLS, "balanced", 0.5, 0.5, 0.7, "窄表-均衡-70%"),
    ("narrow", TABLE_NARROW_COLS, "balanced", 0.5, 0.5, 1.0, "窄表-均衡-100%"),
    
    ("narrow", TABLE_NARROW_COLS, "write_intensive", 0.3, 0.7, 0.1, "窄表-写密集-10%"),
    ("narrow", TABLE_NARROW_COLS, "write_intensive", 0.3, 0.7, 0.4, "窄表-写密集-40%"),
    ("narrow", TABLE_NARROW_COLS, "write_intensive", 0.3, 0.7, 0.7, "窄表-写密集-70%"),
    ("narrow", TABLE_NARROW_COLS, "write_intensive", 0.3, 0.7, 1.0, "窄表-写密集-100%"),
    
    # 宽表场景 (12个)
    ("wide", TABLE_WIDE_COLS, "read_intensive", 0.7, 0.3, 0.1, "宽表-读密集-10%"),
    ("wide", TABLE_WIDE_COLS, "read_intensive", 0.7, 0.3, 0.4, "宽表-读密集-40%"),
    ("wide", TABLE_WIDE_COLS, "read_intensive", 0.7, 0.3, 0.7, "宽表-读密集-70%"),
    ("wide", TABLE_WIDE_COLS, "read_intensive", 0.7, 0.3, 1.0, "宽表-读密集-100%"),
    
    ("wide", TABLE_WIDE_COLS, "balanced", 0.5, 0.5, 0.1, "宽表-均衡-10%"),
    ("wide", TABLE_WIDE_COLS, "balanced", 0.5, 0.5, 0.4, "宽表-均衡-40%"),
    ("wide", TABLE_WIDE_COLS, "balanced", 0.5, 0.5, 0.7, "宽表-均衡-70%"),
    ("wide", TABLE_WIDE_COLS, "balanced", 0.5, 0.5, 1.0, "宽表-均衡-100%"),
    
    ("wide", TABLE_WIDE_COLS, "write_intensive", 0.3, 0.7, 0.1, "宽表-写密集-10%"),
    ("wide", TABLE_WIDE_COLS, "write_intensive", 0.3, 0.7, 0.4, "宽表-写密集-40%"),
    ("wide", TABLE_WIDE_COLS, "write_intensive", 0.3, 0.7, 0.7, "宽表-写密集-70%"),
    ("wide", TABLE_WIDE_COLS, "write_intensive", 0.3, 0.7, 1.0, "宽表-写密集-100%"),
]


def run_single_test(table_type: str, num_cols: int, olap_ratio: float,
                    oltp_ratio: float, data_access_ratio: float, 
                    mode: str = "baseline") -> Optional[float]:
    """
    运行单个测试
    
    Returns:
        执行时间（秒），如果失败返回None
    """
    cmd = [
        "cargo", "run", "--release", "--",
        "--num-columns", str(num_cols),
        "--olap-ratio", str(olap_ratio),
        "--oltp-ratio", str(oltp_ratio),
        "--data-access-ratio", str(data_access_ratio),
        "--total-ops", str(TOTAL_OPERATIONS),
        "--prepopulate-rows", str(PREPOPULATE_ROWS),
        "--mode", mode
    ]
    
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=600,  # 10分钟超时
            cwd=os.path.dirname(os.path.abspath(__file__))
        )
        
        # 从输出中解析执行时间
        execution_time = parse_execution_time(result.stdout)
        
        if execution_time is None:
            print(f"    警告: 未能解析执行时间")
            print(f"    输出: {result.stdout[-200:]}")  # 显示最后200字符
        
        return execution_time
        
    except subprocess.TimeoutExpired:
        print(f"    错误: 测试超时")
        return None
    except Exception as e:
        print(f"    错误: {e}")
        return None


def parse_execution_time(output: str) -> Optional[float]:
    """
    从程序输出中解析执行时间
    期望格式: "  Execution Time: 12.345 seconds"
    """
    pattern = r"Execution Time:\s+(\d+\.?\d*)\s+seconds"
    match = re.search(pattern, output)
    
    if match:
        return float(match.group(1))
    
    return None


def cleanup_databases():
    """清理所有测试数据库"""
    import glob
    import shutil
    
    patterns = ["htap_test_db_*"]
    for pattern in patterns:
        for path in glob.glob(pattern):
            try:
                if os.path.isdir(path):
                    shutil.rmtree(path)
                    print(f"  已清理: {path}")
            except Exception as e:
                print(f"  清理失败 {path}: {e}")


def main():
    """主函数：运行所有测试场景并生成CSV"""
    
    print("=" * 80)
    print("HTAP混合负载测试 - 自动化测试启动器")
    print("=" * 80)
    print(f"总场景数: {len(TEST_SCENARIOS)}")
    print(f"每场景操作数: {TOTAL_OPERATIONS}")
    print(f"预填充行数: {PREPOPULATE_ROWS}")
    print("=" * 80)
    
    # 生成输出文件名
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    csv_file = f"htap_test_results_{timestamp}.csv"
    
    print(f"\n结果将保存到: {csv_file}\n")
    
    # 确保在正确的目录下
    script_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(script_dir)
    
    # 构建项目
    print("构建测试程序...")
    build_result = subprocess.run(
        ["cargo", "build", "--release"],
        capture_output=True,
        text=True,
        cwd=script_dir
    )
    
    if build_result.returncode != 0:
        print("构建失败！")
        print(build_result.stderr)
        return
    
    print("构建成功！\n")
    
    # 收集结果
    results = []
    
    # 遍历所有测试场景
    for idx, (table_type, num_cols, workload_type, olap_ratio, oltp_ratio,
              data_access_ratio, label) in enumerate(TEST_SCENARIOS, 1):
        
        print(f"\n{'='*80}")
        print(f"[{idx}/{len(TEST_SCENARIOS)}] 场景: {label}")
        print(f"  表类型: {table_type} ({num_cols}列)")
        print(f"  OLAP: {olap_ratio*100:.0f}%, OLTP: {oltp_ratio*100:.0f}%")
        print(f"  数据访问比例: {data_access_ratio*100:.0f}%")
        print(f"{'='*80}")
        
        # 清理之前的数据库
        cleanup_databases()
        
        # 运行基线版本
        print("\n[1/2] 运行基线版本...")
        baseline_time = run_single_test(
            table_type, num_cols, olap_ratio, oltp_ratio,
            data_access_ratio, mode="baseline"
        )
        
        if baseline_time is not None:
            print(f"  ✓ 基线版本完成: {baseline_time:.3f}s")
        else:
            print(f"  ✗ 基线版本失败")
            baseline_time = 0.0
        
        # 清理数据库
        cleanup_databases()
        
        # 运行优化版本
        print("\n[2/2] 运行优化版本...")
        optimized_time = run_single_test(
            table_type, num_cols, olap_ratio, oltp_ratio,
            data_access_ratio, mode="optimized"
        )
        
        if optimized_time is not None:
            print(f"  ✓ 优化版本完成: {optimized_time:.3f}s")
        else:
            print(f"  ✗ 优化版本失败")
            optimized_time = 0.0
        
        # 计算性能指标
        if baseline_time > 0 and optimized_time > 0:
            speedup = baseline_time / optimized_time
            improvement = ((baseline_time - optimized_time) / baseline_time) * 100
        else:
            speedup = 0.0
            improvement = 0.0
        
        print(f"\n结果汇总:")
        print(f"  基线时间: {baseline_time:.3f}s")
        print(f"  优化时间: {optimized_time:.3f}s")
        print(f"  加速比: {speedup:.2f}x")
        print(f"  性能提升: {improvement:.2f}%")
        
        # 保存结果
        results.append({
            "场景": label,
            "表类型": "窄表" if table_type == "narrow" else "宽表",
            "列数": num_cols,
            "负载类型": workload_type,
            "OLAP比例": olap_ratio,
            "OLTP比例": oltp_ratio,
            "数据访问比例": data_access_ratio,
            "基线时间(s)": baseline_time,
            "优化时间(s)": optimized_time,
            "加速比": speedup,
            "性能提升(%)": improvement
        })
        
        # 实时保存中间结果
        with open(csv_file, 'w', newline='', encoding='utf-8') as f:
            if results:
                writer = csv.DictWriter(f, fieldnames=results[0].keys())
                writer.writeheader()
                writer.writerows(results)
    
    # 最终清理
    print(f"\n{'='*80}")
    print("清理测试数据库...")
    cleanup_databases()
    
    # 显示最终摘要
    print(f"\n{'='*80}")
    print("测试完成！")
    print(f"{'='*80}")
    print(f"结果已保存到: {os.path.abspath(csv_file)}")
    print(f"总场景数: {len(results)}")
    
    if results:
        valid_results = [r for r in results if r['加速比'] > 0]
        if valid_results:
            avg_speedup = sum(r['加速比'] for r in valid_results) / len(valid_results)
            max_speedup = max(r['加速比'] for r in valid_results)
            max_improvement = max(r['性能提升(%)'] for r in valid_results)
            
            print(f"\n性能统计:")
            print(f"  平均加速比: {avg_speedup:.2f}x")
            print(f"  最大加速比: {max_speedup:.2f}x")
            print(f"  最大性能提升: {max_improvement:.2f}%")
    
    print(f"\n{'='*80}")
    print(f"下一步: 运行绘图脚本生成可视化结果")
    print(f"  python plot_htap_results.py {csv_file}")
    print(f"{'='*80}")


if __name__ == "__main__":
    main()
