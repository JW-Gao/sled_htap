# L2 Cache 测试报告生成器

## 简介

`generate_l2_report.py` 是一个 Python 脚本，用于将 L2 cache 测试的 CSV 结果文件转换为 Jupyter notebook，包含表格、统计分析和可视化图表。

## 功能特性

- 📊 **自动生成表格**：汇总统计、性能对比、详细结果
- 📈 **可视化图表**：柱状图、性能改进趋势图
- 🔍 **数据分析**：按 workload 类型、TP:AP 比例等维度分析
- 📝 **完整报告**：包含所有测试参数和结果的详细表格

## 安装依赖

```bash
pip install pandas matplotlib seaborn jupyter
```

或者使用 conda：

```bash
conda install pandas matplotlib seaborn jupyter
```

## 使用方法

### 基本用法

```bash
# 从单个 CSV 文件生成报告
python generate_l2_report.py results/l2_test_20241117_120000.csv

# 指定输出文件名
python generate_l2_report.py results/l2_test_20241117_120000.csv --output my_report.ipynb

# 处理多个 CSV 文件（合并数据）
python generate_l2_report.py results/*.csv --output combined_report.ipynb
```

### 查看报告

生成 notebook 后，使用 Jupyter 打开：

```bash
# 启动 Jupyter notebook
jupyter notebook l2_report_20241117_120000.ipynb

# 或者使用 JupyterLab
jupyter lab l2_report_20241117_120000.ipynb
```

## 报告内容

生成的 Jupyter notebook 包含以下部分：

### 1. 数据概览
- 总记录数
- 列信息
- 数据预览

### 2. 汇总统计
按 L2 启用状态和工作负载类型分组的统计信息：
- 平均值
- 标准差
- 最小值/最大值
- 测试次数

### 3. 性能对比表
L2 ON vs L2 OFF 的直接对比，包括：
- 各配置下的性能（ops/s）
- 性能改进百分比

### 4. 按工作负载类型的性能图表
柱状图展示不同 workload（tp/ap/mixed）下 L2 ON 和 L2 OFF 的性能对比

### 5. 性能改进百分比图表
可视化展示 L2 cache 带来的性能提升（或开销）

### 6. 混合负载分析
针对 mixed workload 的详细分析：
- 不同 TP:AP 比例下的性能对比
- 可视化图表

### 7. 详细结果表
包含所有测试参数和结果的完整表格

## 支持的 CSV 格式

脚本支持两种 CSV 格式：

### 格式 1：test_l2_cache.sh 生成的格式
```csv
test_name,workload,l2_enabled,threads,tp_threads,ap_threads,ops,ops_per_sec,improvement_pct
l2_on,ap,yes,8,0,0,50000,5000,150.00
l2_off,ap,no,8,0,0,20000,2000,0.00
```

### 格式 2：ablation_study.sh 生成的格式
```csv
experiment,config,tp_ap_ratio,l2_enabled,pull_enabled,tp_threads,ap_threads,tp_ops,ap_ops,tp_ops_per_sec,ap_ops_per_sec,total_ops_per_sec
exp2_l2_storage,l2_on,3:1,yes,yes,6,2,102440,18823,10229,1880,12109
exp2_l2_storage,l2_off,3:1,no,no,6,2,0,0,0,0,0
```

脚本会自动识别并处理这两种格式。

## 示例工作流

### 完整测试和报告生成流程

```bash
# 1. 运行测试
./test_l2_cache.sh --workload ap --time 30

# 2. 生成报告
python generate_l2_report.py results/l2_test_*.csv

# 3. 查看报告
jupyter notebook l2_report_*.ipynb
```

### 批量处理多个测试结果

```bash
# 运行多个测试
./test_l2_cache.sh --workload ap --time 30 --output results/test1.csv
./test_l2_cache.sh --workload tp --time 30 --output results/test2.csv
./test_l2_cache.sh --workload mixed --tp-ap-ratio 3:1 --time 30 --output results/test3.csv

# 生成综合报告
python generate_l2_report.py results/test*.csv --output comprehensive_report.ipynb
```

## 自定义报告

如果需要自定义报告内容，可以：

1. 生成基础 notebook
2. 在 Jupyter 中打开并编辑
3. 添加自定义分析代码

例如，添加更多可视化：

```python
# 在 notebook 中添加新 cell
import plotly.express as px

fig = px.scatter(df, x='tp_threads', y='ops_per_sec', 
                 color='l2_enabled', size='ap_threads',
                 hover_data=['workload'])
fig.show()
```

## 故障排除

### 问题：找不到 pandas/matplotlib

**解决**：安装依赖
```bash
pip install pandas matplotlib seaborn
```

### 问题：CSV 格式不匹配

**解决**：脚本会自动尝试识别格式，如果失败，检查 CSV 文件是否包含必需的列：
- `l2_enabled` 或可以通过其他列推断
- `ops_per_sec` 或 `total_ops_per_sec`
- `workload`

### 问题：图表不显示

**解决**：确保在 Jupyter 中运行，而不是直接运行 Python 脚本。notebook 中已包含 `%matplotlib inline`。

## 高级用法

### 导出为 HTML

在 Jupyter 中：
1. File → Download as → HTML
2. 或使用命令行：
```bash
jupyter nbconvert --to html l2_report.ipynb
```

### 导出为 PDF

```bash
jupyter nbconvert --to pdf l2_report.ipynb
```

（需要安装 LaTeX）

### 自动化报告生成

创建脚本 `auto_report.sh`：

```bash
#!/bin/bash
# Run tests and generate report
./test_l2_cache.sh --workload ap --time 30
python generate_l2_report.py results/l2_test_*.csv
echo "Report generated: l2_report_*.ipynb"
```

## 示例输出

生成的 notebook 包含：

1. **表格**：格式化的 HTML 表格，易于阅读
2. **图表**：清晰的柱状图和趋势图
3. **统计信息**：平均值、标准差等
4. **对比分析**：L2 ON vs L2 OFF 的详细对比

所有内容都可以在 Jupyter 中交互式查看和修改。




