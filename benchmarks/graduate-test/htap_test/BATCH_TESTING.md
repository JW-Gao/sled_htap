# HTAP分批测试使用说明

## 测试结构

24个测试场景分成6个批次，每批4个场景：

| 批次 | 名称 | 场景数 |
|------|------|--------|
| 1 | 窄表-读密集型 | 4 |
| 2 | 窄表-均衡型 | 4 |
| 3 | 窄表-写密集型 | 4 |
| 4 | 宽表-读密集型 | 4 |
| 5 | 宽表-均衡型 | 4 |
| 6 | 宽表-写密集型 | 4 |

每个批次测试4种数据访问比例：10%, 40%, 70%, 100%

## 使用方法

### 1. 运行单个批次

```bash
python run_htap_test_batch.py <batch_number>
```

示例：
```bash
# 运行批次1（窄表-读密集型）
python run_htap_test_batch.py 1

# 运行批次2（窄表-均衡型）
python run_htap_test_batch.py 2
```

每个批次会生成独立的CSV文件：`htap_batch<N>_<timestamp>.csv`

### 2. 合并所有批次结果

运行完所有6个批次后，合并结果：

```bash
python merge_results.py
```

这将生成合并后的CSV文件：`htap_test_results_merged_<timestamp>.csv`

### 3. 生成可视化图表

使用合并后的CSV文件生成图表：

```bash
python plot_htap_results.py htap_test_results_merged_<timestamp>.csv
```

## 完整工作流

```bash
# 步骤1: 依次运行6个批次
python run_htap_test_batch.py 1
python run_htap_test_batch.py 2
python run_htap_test_batch.py 3
python run_htap_test_batch.py 4
python run_htap_test_batch.py 5
python run_htap_test_batch.py 6

# 步骤2: 合并结果
python merge_results.py

# 步骤3: 生成图表
python plot_htap_results.py htap_test_results_merged_*.csv
```

## 单批次预计时间

每个批次包含4个场景 × 2个版本（基线+优化）= 8次测试
- 每次测试约 1-3 分钟
- 单批次总时间约 10-25 分钟

## 输出文件

### 批次CSV文件
- `htap_batch1_<timestamp>.csv` - 窄表-读密集型
- `htap_batch2_<timestamp>.csv` - 窄表-均衡型
- `htap_batch3_<timestamp>.csv` - 窄表-写密集型
- `htap_batch4_<timestamp>.csv` - 宽表-读密集型
- `htap_batch5_<timestamp>.csv` - 宽表-均衡型
- `htap_batch6_<timestamp>.csv` - 宽表-写密集型

### 合并文件
- `htap_test_results_merged_<timestamp>.csv`

### 图表文件
- `htap_execution_time_comparison.png`
- `htap_speedup_heatmap.png`
- `htap_performance_improvement.png`
- `htap_workload_comparison.png`
- `htap_summary_table.png`

## 优势

分批测试的优势：
1. **可中断**: 每批次独立，可随时停止
2. **快速反馈**: 每批完成即可查看结果
3. **灵活性**: 可选择性运行特定批次
4. **风险低**: 错误影响范围小
