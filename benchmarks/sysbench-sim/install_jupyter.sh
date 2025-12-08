#!/bin/bash
# Install Jupyter environment for L2 cache test visualization

set -e

echo "========================================="
echo "Installing Jupyter Environment"
echo "========================================="
echo ""

# Check if pip3 is available
if ! command -v pip3 &> /dev/null; then
    echo "pip3 not found. Installing pip3..."
    echo "Please run: sudo apt install python3-pip"
    echo "Then run this script again."
    exit 1
fi

# Check if python3 is available
if ! command -v python3 &> /dev/null; then
    echo "python3 not found. Please install Python 3 first."
    exit 1
fi

echo "Installing required packages..."
echo "This may take a few minutes..."
echo ""

# Install packages
python3 -m pip install --user pandas matplotlib seaborn jupyter

echo ""
echo "========================================="
echo "Installation Complete!"
echo "========================================="
echo ""
echo "To use Jupyter:"
echo "  1. Add ~/.local/bin to PATH (if not already):"
echo "     export PATH=\$HOME/.local/bin:\$PATH"
echo ""
echo "  2. Start Jupyter:"
echo "     jupyter notebook batch_l2_report_*.ipynb"
echo ""
echo "  3. Or use JupyterLab:"
echo "     jupyter lab batch_l2_report_*.ipynb"
echo ""




