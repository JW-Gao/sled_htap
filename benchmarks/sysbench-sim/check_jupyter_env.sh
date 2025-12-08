#!/bin/bash
# Quick check script for Jupyter environment

echo "========================================="
echo "Jupyter Environment Check"
echo "========================================="
echo ""

echo -n "Python: "
python3 --version 2>/dev/null || echo "NOT FOUND"

echo -n "pandas: "
python3 -c "import pandas; print('✓', pandas.__version__)" 2>/dev/null || echo "✗ NOT INSTALLED"

echo -n "matplotlib: "
python3 -c "import matplotlib; print('✓', matplotlib.__version__)" 2>/dev/null || echo "✗ NOT INSTALLED"

echo -n "seaborn: "
python3 -c "import seaborn; print('✓', seaborn.__version__)" 2>/dev/null || echo "✗ NOT INSTALLED"

echo -n "jupyter: "
jupyter --version 2>/dev/null | head -1 || echo "✗ NOT INSTALLED"

echo ""
echo "========================================="
if python3 -c "import pandas, matplotlib, seaborn" 2>/dev/null && jupyter --version >/dev/null 2>&1; then
    echo "✓ All required packages are installed!"
    echo ""
    echo "To start Jupyter:"
    echo "  jupyter notebook batch_l2_report_*.ipynb"
else
    echo "✗ Some packages are missing"
    echo ""
    echo "To install missing packages:"
    echo "  pip3 install pandas matplotlib seaborn jupyter"
    echo ""
    echo "Or install for current user only:"
    echo "  pip3 install --user pandas matplotlib seaborn jupyter"
fi
echo "========================================="




