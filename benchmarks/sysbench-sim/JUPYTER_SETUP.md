# Jupyter Notebook 使用指南

## 环境要求

Jupyter notebook 需要以下 Python 包：

- `pandas` - 数据处理
- `matplotlib` - 绘图
- `seaborn` - 美化图表
- `jupyter` 或 `jupyterlab` - Jupyter 环境

## 安装方法

### 方法1：使用 pip 安装（推荐）

```bash
# 安装所有必需的包
pip3 install pandas matplotlib seaborn jupyter

# 或者使用用户安装（不需要sudo）
pip3 install --user pandas matplotlib seaborn jupyter
```

### 方法2：使用 conda（如果使用conda环境）

```bash
conda install pandas matplotlib seaborn jupyter
```

### 方法3：创建虚拟环境（推荐用于项目隔离）

```bash
# 创建虚拟环境
python3 -m venv venv

# 激活虚拟环境
source venv/bin/activate

# 安装依赖
pip install pandas matplotlib seaborn jupyter
```

## 使用方法

### 1. 启动 Jupyter Notebook

```bash
cd benchmarks/sysbench-sim

# 启动 Jupyter Notebook（传统界面）
jupyter notebook

# 或者启动 JupyterLab（更现代的界面）
jupyter lab
```

这会在浏览器中打开 Jupyter 界面（通常是 http://localhost:8888）

### 2. 打开 notebook 文件

在 Jupyter 界面中：
1. 找到 `batch_l2_report_*.ipynb` 文件
2. 点击打开
3. 运行所有单元格：菜单 `Cell` → `Run All`

### 3. 或者直接打开特定文件

```bash
# 直接打开指定的notebook
jupyter notebook batch_l2_report_20251117_174312.ipynb

# 或使用 JupyterLab
jupyter lab batch_l2_report_20251117_174312.ipynb
```

## 运行 notebook

### 方式1：在浏览器中运行

1. 打开 Jupyter 界面
2. 点击 notebook 文件
3. 逐个运行单元格（Shift+Enter）或运行所有（Cell → Run All）

### 方式2：命令行运行（无需浏览器）

```bash
# 安装 nbconvert（如果还没有）
pip3 install nbconvert

# 转换为HTML查看
jupyter nbconvert --to html batch_l2_report_*.ipynb

# 转换为PDF（需要LaTeX）
jupyter nbconvert --to pdf batch_l2_report_*.ipynb
```

## 故障排除

### 问题1：ModuleNotFoundError: No module named 'pandas'

**解决**：
```bash
pip3 install pandas matplotlib seaborn
```

### 问题2：jupyter: command not found

**解决**：
```bash
pip3 install jupyter
# 如果使用用户安装，可能需要添加到PATH
export PATH=$HOME/.local/bin:$PATH
```

### 问题3：端口被占用

**解决**：
```bash
# 使用其他端口
jupyter notebook --port 8889
```

### 问题4：浏览器没有自动打开

**解决**：
- 手动访问显示的URL（通常是 http://localhost:8888）
- 或者使用 `--no-browser` 参数，然后手动打开

## 快速检查脚本

创建一个检查脚本 `check_jupyter_env.sh`：

```bash
#!/bin/bash
echo "Checking Jupyter environment..."

echo -n "Python: "
python3 --version

echo -n "pandas: "
python3 -c "import pandas; print(pandas.__version__)" 2>/dev/null || echo "NOT INSTALLED"

echo -n "matplotlib: "
python3 -c "import matplotlib; print(matplotlib.__version__)" 2>/dev/null || echo "NOT INSTALLED"

echo -n "seaborn: "
python3 -c "import seaborn; print(seaborn.__version__)" 2>/dev/null || echo "NOT INSTALLED"

echo -n "jupyter: "
jupyter --version 2>/dev/null || echo "NOT INSTALLED"

echo ""
echo "To install missing packages:"
echo "  pip3 install pandas matplotlib seaborn jupyter"
```

## 最小化安装（如果只需要查看结果）

如果只需要查看图表而不需要交互，可以：

1. **转换为HTML**：
```bash
pip3 install nbconvert
jupyter nbconvert --to html batch_l2_report_*.ipynb
# 然后用浏览器打开生成的HTML文件
```

2. **或者使用在线Jupyter**：
   - 上传到 Google Colab
   - 或使用 Jupyter Notebook Viewer (nbviewer)

## 示例：完整使用流程

```bash
# 1. 进入目录
cd benchmarks/sysbench-sim

# 2. 检查环境（可选）
python3 -c "import pandas, matplotlib, seaborn, jupyter; print('All packages installed!')"

# 3. 如果缺少包，安装
pip3 install pandas matplotlib seaborn jupyter

# 4. 启动Jupyter
jupyter notebook

# 5. 在浏览器中打开 batch_l2_report_*.ipynb
# 6. 运行所有单元格查看结果
```

## 注意事项

1. **数据文件位置**：确保CSV文件在notebook同一目录，或修改notebook中的文件路径
2. **Python版本**：建议使用Python 3.7+
3. **内存**：处理大量数据时可能需要足够内存




