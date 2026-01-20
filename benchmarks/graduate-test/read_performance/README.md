# Read Performance Benchmarks (读性能验证实验)

本目录包含针对新 HTAP 架构 (Row-Level L1 + Column-Level L2) 与原生 BW-Tree (Baseline) 的核心性能对比测试。

## 目录结构
*   `mix_load.rs`: 混合负载测试源码 (Experiment 1)。
*   `range_scan_cmp/`: Range Scan 稳定性测试目录 (Experiment 2)。
    *   `range_scan.rs`: Range Scan 测试源码。
    *   `run_range_comparison.py`: 自动化对比脚本。
*   `run_comparison.py`: 混合负载自动化对比脚本。

---

## 实验一：混合读写吞吐量对比 (Mixed Workload Throughput)

验证在不同读写比例下，新架构消除读放大带来的吞吐量提升。

### 运行方式
在项目根目录下执行：

```bash
# 自动运行 Baseline 与 Ours 的对比测试，并生成图表
python3 benchmarks/graduate-test/read_performance/run_comparison.py
```

### 结果产物
脚本运行完成后，将在本目录下生成：
1.  **对比数据**: `comparison_results.csv`
2.  **吞吐量对比图**: `comparison_qps.png` (柱状图)
3.  **延迟对比图**: `comparison_latency.png` (柱状图)

---

## 实验二：Range Scan 稳定性对比 (Range Scan Stability)

验证在持续高频更新（数据碎片化）场景下，新架构的后台合并机制对 Range Query 延迟的稳定作用。

### 运行方式
在项目根目录下执行：

```bash
# 自动运行 Baseline 与 Ours 的对比测试，并生成图表
python3 benchmarks/graduate-test/read_performance/range_scan_cmp/run_range_comparison.py
```

### 结果产物
脚本运行完成后，将在 `range_scan_cmp/` 目录下生成：
1.  **趋势对比图**: `comparison_range_scan.png` (折线图)
    *   展示了随着迭代次数（碎片化程度）增加，Baseline 与 Ours 的扫描延迟变化趋势。
2.  **原始日志**: `results_Baseline.txt`, `results_Ours.txt`

---

## 注意事项
1.  **环境要求**: 确保已安装 `matplotlib` (`pip install matplotlib`)。脚本会自动配置中文字体 (`WenQuanYi Micro Hei`)。
2.  **缓存配置**: 测试默认配置为 IO-Bound 模式 (64MB Cache)，以逼真模拟大数据量下的外存表现。
