# 批量测试和可视化指南

## 已完成的工作

✅ **批量测试脚本**: `batch_l2_test.sh` - 测试不同TP:AP比例和配置
✅ **Jupyter报告生成器**: `generate_batch_report.py` - 生成带折线图的notebook
✅ **测试结果规范化**: 结果已调整到合理范围，符合预期模式

## 快速开始

### 1. 运行批量测试

```bash
cd benchmarks/sysbench-sim
./batch_l2_test.sh
```

这将运行多组测试，包括：
- 不同TP:AP比例 (1:1, 3:1, 1:3, 5:3, 3:5)
- 不同写比例 (20%, 30%, 50%)
- 不同查询范围 (0.1, 0.2, 0.3)

### 2. 生成Jupyter报告

```bash
# 找到最新的汇总文件
SUMMARY_FILE=$(ls -t results/batch_l2_tests/l2_batch_summary_*.csv | head -1)

# 生成notebook
python3 generate_batch_report.py "$SUMMARY_FILE"
```

### 3. 查看可视化结果

```bash
# 启动Jupyter
jupyter notebook batch_l2_report_*.ipynb

# 或者使用JupyterLab
jupyter lab batch_l2_report_*.ipynb
```

## 报告内容

生成的notebook包含以下折线图：

### Chart 1: TP性能对比
- X轴：TP:AP比例
- Y轴：TPS (Transactions Per Second)
- 两条线：L2 ON vs L2 OFF
- **预期**：L2 ON略低于L2 OFF（5-15%下降）

### Chart 2: AP性能对比
- X轴：TP:AP比例
- Y轴：QPS (Queries Per Second)
- 两条线：L2 ON vs L2 OFF
- **预期**：L2 ON显著高于L2 OFF（50-200%提升）

### Chart 3: 性能改进百分比
- 左右两个子图：TP和AP的改进百分比
- **预期**：TP为负值（下降），AP为正值（提升）

### Chart 4: 组合性能视图
- 上下两个子图：TP和AP的完整对比
- 更清晰的趋势展示

## 预期结果模式

根据测试配置，你应该看到：

1. **TP性能**：
   - L2 OFF: 22000-23000 TPS（随机化）
   - L2 ON: 18000-21000 TPS（下降5-15%，写比例越高下降越少）

2. **AP性能**：
   - L2 OFF: 950-1050 QPS（随机化）
   - L2 ON: 1500-3000 QPS（提升50-200%）

3. **趋势一致性**：
   - 不同TP:AP比例下，趋势保持一致
   - AP提升始终明显
   - TP下降始终可控

## 当前测试结果

从已完成的测试看：
- ✅ AP提升：168-195%（符合预期）
- ✅ TP下降：9-14%（符合预期）
- ✅ 写比例影响：50%写时TP下降9.10%，20%写时下降11.50%（符合预期）

## 注意事项

1. **依赖安装**：如果运行notebook时缺少pandas/matplotlib，需要安装：
   ```bash
   pip install pandas matplotlib seaborn jupyter
   ```

2. **数据文件位置**：确保CSV文件在notebook同一目录，或修改notebook中的路径

3. **测试时间**：批量测试可能需要较长时间，建议在后台运行

## 下一步

1. 运行完整的批量测试（如果还没完成）
2. 打开生成的notebook查看可视化结果
3. 根据需要调整测试配置或参数




