#!/usr/bin/env python3
"""
HTAP混合负载测试 - 分批运行脚本
将24个场景分成6个批次，每批4个场景
"""

import subprocess
import csv
import os
import re
import sys
from datetime import datetime
from typing import Optional

# 测试配置
TOTAL_OPERATIONS = 50000
PREPOPULATE_ROWS = 100000
TABLE_NARROW_COLS = 30
TABLE_WIDE_COLS = 70

# 定义6个批次，每批4个场景
BATCHES = {
    1: {
        "name": "窄表-读密集型",
        "scenarios": [
            ("narrow", TABLE_NARROW_COLS, "read_intensive", 0.7, 0.3, 0.1, "窄表-读密集-10%"),
            ("narrow", TABLE_NARROW_COLS, "read_intensive", 0.7, 0.3, 0.4, "窄表-读密集-40%"),
            ("narrow", TABLE_NARROW_COLS, "read_intensive", 0.7, 0.3, 0.7, "窄表-读密集-70%"),
            ("narrow", TABLE_NARROW_COLS, "read_intensive", 0.7, 0.3, 1.0, "窄表-读密集-100%"),
        ]
    },
    2: {
        "name": "窄表-均衡型",
        "scenarios": [
            ("narrow", TABLE_NARROW_COLS, "balanced", 0.5, 0.5, 0.1, "窄表-均衡-10%"),
            ("narrow", TABLE_NARROW_COLS, "balanced", 0.5, 0.5, 0.4, "窄表-均衡-40%"),
            ("narrow", TABLE_NARROW_COLS, "balanced", 0.5, 0.5, 0.7, "窄表-均衡-70%"),
            ("narrow", TABLE_NARROW_COLS, "balanced", 0.5, 0.5, 1.0, "窄表-均衡-100%"),
        ]
    },
    3: {
        "name": "窄表-写密集型",
        "scenarios": [
            ("narrow", TABLE_NARROW_COLS, "write_intensive", 0.3, 0.7, 0.1, "窄表-写密集-10%"),
            ("narrow", TABLE_NARROW_COLS, "write_intensive", 0.3, 0.7, 0.4, "窄表-写密集-40%"),
            ("narrow", TABLE_NARROW_COLS, "write_intensive", 0.3, 0.7, 0.7, "窄表-写密集-70%"),
            ("narrow", TABLE_NARROW_COLS, "write_intensive", 0.3, 0.7, 1.0, "窄表-写密集-100%"),
        ]
    },
    4: {
        "name": "宽表-读密集型",
        "scenarios": [
            ("wide", TABLE_WIDE_COLS, "read_intensive", 0.7, 0.3, 0.1, "宽表-读密集-10%"),
            ("wide", TABLE_WIDE_COLS, "read_intensive", 0.7, 0.3, 0.4, "宽表-读密集-40%"),
            ("wide", TABLE_WIDE_COLS, "read_intensive", 0.7, 0.3, 0.7, "宽表-读密集-70%"),
            ("wide", TABLE_WIDE_COLS, "read_intensive", 0.7, 0.3, 1.0, "宽表-读密集-100%"),
        ]
    },
    5: {
        "name": "宽表-均衡型",
        "scenarios": [
            ("wide", TABLE_WIDE_COLS, "balanced", 0.5, 0.5, 0.1, "宽表-均衡-10%"),
            ("wide", TABLE_WIDE_COLS, "balanced", 0.5, 0.5, 0.4, "宽表-均衡-40%"),
            ("wide", TABLE_WIDE_COLS, "balanced", 0.5, 0.5, 0.7, "宽表-均衡-70%"),
            ("wide", TABLE_WIDE_COLS, "balanced", 0.5, 0.5, 1.0, "宽表-均衡-100%"),
        ]
    },
    6: {
        "name": "宽表-写密集型",
        "scenarios": [
            ("wide", TABLE_WIDE_COLS, "write_intensive", 0.3, 0.7, 0.1, "宽表-写密集-10%"),
            ("wide", TABLE_WIDE_COLS, "write_intensive", 0.3, 0.7, 0.4, "宽表-写密集-40%"),
            ("wide", TABLE_WIDE_COLS, "write_intensive", 0.3, 0.7, 0.7, "宽表-写密集-70%"),
            ("wide", TABLE_WIDE_COLS, "write_intensive", 0.3, 0.7, 1.0, "宽表-写密集-100%"),
        ]
    },
}


def run_single_test(table_type: str, num_cols: int, olap_ratio: float,
                    oltp_ratio: float, data_access_ratio: float, 
                    mode: str = "baseline") -> Optional[float]:
    """运行单个测试"""
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
            timeout=600,
            cwd=os.path.dirname(os.path.abspath(__file__))
        )
        
        execution_time = parse_execution_time(result.stdout)
        return execution_time
        
    except subprocess.TimeoutExpired:
        print(f"✗ 超时")
        return None
    except Exception as e:
        print(f"✗ 错误: {e}")
        return None


def parse_execution_time(output: str) -> Optional[float]:
    """从程序输出中解析执行时间"""
    pattern = r"Execution Time:\s+(\d+\.?\d*)\s+seconds"
    match = re.search(pattern, output)
    return float(match.group(1)) if match else None


def cleanup_databases():
    """清理测试数据库"""
    import glob
    import shutil
    
    for path in glob.glob("htap_test_db_*"):
        try:
            if os.path.isdir(path):
                shutil.rmtree(path)
        except:
            pass


def run_batch(batch_num: int, timestamp: str):
    """运行单个批次的测试"""
    batch_info = BATCHES[batch_num]
    batch_name = batch_info["name"]
    scenarios = batch_info["scenarios"]
    
    csv_file = f"htap_batch{batch_num}_{timestamp}.csv"
    
    print(f"\n{'='*80}")
    print(f"批次 {batch_num}/6: {batch_name}")
    print(f"场景数: {len(scenarios)}")
    print(f"输出文件: {csv_file}")
    print(f"{'='*80}\n")
    
    results = []
    
    for idx, (table_type, num_cols, workload_type, olap_ratio, oltp_ratio,
              data_access_ratio, label) in enumerate(scenarios, 1):
        
        print(f"[{idx}/4] {label}")
        
        cleanup_databases()
        
        # 基线版本
        print("  基线版本...", end=" ", flush=True)
        baseline_time = run_single_test(
            table_type, num_cols, olap_ratio, oltp_ratio,
            data_access_ratio, mode="baseline"
        )
        if baseline_time:
            print(f"✓ {baseline_time:.2f}s")
        else:
            print("✗ 失败")
            baseline_time = 0.0
        
        cleanup_databases()
        
        # 优化版本
        print("  优化版本...", end=" ", flush=True)
        optimized_time = run_single_test(
            table_type, num_cols, olap_ratio, oltp_ratio,
            data_access_ratio, mode="optimized"
        )
        if optimized_time:
            print(f"✓ {optimized_time:.2f}s")
        else:
            print("✗ 失败")
            optimized_time = 0.0
        
        # 计算性能指标
        if baseline_time > 0 and optimized_time > 0:
            speedup = baseline_time / optimized_time
            improvement = ((baseline_time - optimized_time) / baseline_time) * 100
        else:
            speedup = 0.0
            improvement = 0.0
        
        print(f"  结果: 加速比={speedup:.2f}x, 提升={improvement:.1f}%\n")
        
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
    
    # 保存CSV
    with open(csv_file, 'w', newline='', encoding='utf-8') as f:
        if results:
            writer = csv.DictWriter(f, fieldnames=results[0].keys())
            writer.writeheader()
            writer.writerows(results)
    
    cleanup_databases()
    
    print(f"✓ 批次 {batch_num} 完成，结果已保存到 {csv_file}\n")
    return csv_file


def main():
    """主函数"""
    if len(sys.argv) < 2:
        print("用法: python run_htap_test_batch.py <batch_number>")
        print("\n可用批次:")
        for num, info in BATCHES.items():
            print(f"  {num}: {info['name']} (4个场景)")
        print("\n示例: python run_htap_test_batch.py 1")
        sys.exit(1)
    
    batch_num = int(sys.argv[1])
    
    if batch_num not in BATCHES:
        print(f"错误: 无效的批次号 {batch_num}，请选择 1-6")
        sys.exit(1)
    
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    
    # 构建程序（只需一次）
    print("构建测试程序...")
    build_result = subprocess.run(
        ["cargo", "build", "--release"],
        capture_output=True,
        text=True,
        cwd=os.path.dirname(os.path.abspath(__file__))
    )
    
    if build_result.returncode != 0:
        print("✗ 构建失败")
        print(build_result.stderr)
        sys.exit(1)
    
    print("✓ 构建成功\n")
    
    # 运行批次
    csv_file = run_batch(batch_num, timestamp)
    
    print(f"{'='*80}")
    print(f"批次 {batch_num} 测试完成！")
    print(f"结果文件: {csv_file}")
    print(f"{'='*80}")


if __name__ == "__main__":
    main()
