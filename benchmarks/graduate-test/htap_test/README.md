# HTAP混合负载测试

这个目录包含HTAP（混合事务/分析处理）混合负载测试的完整实现。

## 文件说明

### Rust测试程序
- `Cargo.toml` - 项目配置
- `src/schema.rs` - 表结构定义（支持窄表30列、宽表70列）
- `src/olap_queries.rs` - OLAP查询实现（Q1/Q2/Q3）
- `src/workload.rs` - 混合负载生成器
- `src/main.rs` - 主程序

### Python脚本
- `run_htap_test.py` - 自动化测试启动脚本
- `plot_htap_results.py` - 结果可视化脚本（中文图表）

## 测试方案

根据 `plan.txt` 的设计：

### 数据模型
- **窄表**: 30列（1个主键 + 29个数值列）
- **宽表**: 70列（1个主键 + 69个数值列）

### 负载类型
- **OLTP**: 单条写入操作
- **OLAP**: 三种查询
  - Q1: 选择性统计 (COUNT)
  - Q2: 多列投影 (SELECT多列)
  - Q3: 聚合分析 (MAX, AVG)

### 测试场景（共24个）
- 窄表 × 3种负载类型 × 4种数据访问比例 = 12个场景
- 宽表 × 3种负载类型 × 4种数据访问比例 = 12个场景

**负载类型**:
- 读密集型: OLAP 70%, OLTP 30%
- 均衡型: OLAP 50%, OLTP 50%
- 写密集型: OLAP 30%, OLTP 70%

**数据访问比例**: 10%, 40%, 70%, 100%

## 使用方法

### 1. 构建测试程序

```bash
cd /home/rat/sled/benchmarks/graduate-test/htap_test
cargo build --release
```

### 2. 运行单个测试（验证）

```bash
cargo run --release -- \
  --num-columns 30 \
  --olap-ratio 0.5 \
  --oltp-ratio 0.5 \
  --data-access-ratio 0.5 \
  --total-ops 10000 \
  --prepopulate-rows 50000 \
  --mode baseline
```

### 3. 运行完整测试套件

```bash
python run_htap_test.py
```

这将自动运行所有24个测试场景，并生成CSV结果文件：
- `htap_test_results_<timestamp>.csv`

### 4. 生成可视化图表

```bash
python plot_htap_results.py htap_test_results_<timestamp>.csv
```

将生成以下图表（使用WenQuanYi Micro Hei中文字体）：
- `htap_execution_time_comparison.png` - 执行时间对比
- `htap_speedup_heatmap.png` - 加速比热图
- `htap_performance_improvement.png` - 性能提升百分比
- `htap_workload_comparison.png` - 负载类型对比
- `htap_summary_table.png` - 详细结果摘要表

## 命令行参数

Rust测试程序支持以下参数：

- `--num-columns <N>` - 列数（30或70）
- `--olap-ratio <R>` - OLAP操作比例（0.0-1.0）
- `--oltp-ratio <R>` - OLTP操作比例（0.0-1.0）
- `--data-access-ratio <R>` - AP数据访问比例（0.0-1.0）
- `--total-ops <N>` - 总操作数（默认50000）
- `--prepopulate-rows <N>` - 预填充行数（默认100000）
- `--mode <MODE>` - 模式: baseline 或 optimized

## 注意事项

1. **中文字体**: 绘图脚本使用 WenQuanYi Micro Hei 字体，请确保系统已安装
2. **测试时间**: 完整测试套件（24个场景）可能需要较长时间
3. **磁盘空间**: 测试会创建临时数据库，确保有足够磁盘空间
4. **数据库清理**: 脚本会自动清理临时数据库

## 输出示例

CSV文件格式：
```
场景,表类型,列数,负载类型,OLAP比例,OLTP比例,数据访问比例,基线时间(s),优化时间(s),加速比,性能提升(%)
窄表-读密集-10%,窄表,30,read_intensive,0.7,0.3,0.1,12.34,8.56,1.44,30.64
...
```

## 参考文档

详细的测试方案请参考：`plan.txt`
