#!/usr/bin/env python3
"""
Generate Jupyter Notebook Report with Line Charts from Batch L2 Test Results

Usage:
    python generate_batch_report.py <summary_csv_file>
    python generate_batch_report.py results/batch_l2_tests/l2_batch_summary_*.csv
"""

import argparse
import json
import sys
import csv
from pathlib import Path
from datetime import datetime

try:
    import pandas as pd
    HAS_PANDAS = True
except ImportError:
    HAS_PANDAS = False


def read_csv_simple(csv_file):
    """Read CSV file without pandas."""
    data = []
    with open(csv_file, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            data.append(row)
    return data


def create_notebook(summary_file, output_file):
    """Create a Jupyter notebook with line charts from batch test summary."""
    
    # Read summary CSV
    try:
        if HAS_PANDAS:
            df = pd.read_csv(summary_file)
            data = df.to_dict('records')
        else:
            data = read_csv_simple(summary_file)
            # Create a simple DataFrame-like structure
            class SimpleDF:
                def __init__(self, data):
                    self.data = data
                    self.columns = list(data[0].keys()) if data else []
                
                def __len__(self):
                    return len(self.data)
                
                def sort_values(self, col):
                    return SimpleDF(sorted(self.data, key=lambda x: x.get(col, '')))
                
                def __getitem__(self, key):
                    return [row.get(key) for row in self.data]
            
            df = SimpleDF(data)
    except Exception as e:
        print(f"Error: Could not read {summary_file}: {e}", file=sys.stderr)
        return False
    
    if len(df) == 0:
        print("Error: No data in summary file", file=sys.stderr)
        return False
    
    # Create notebook cells
    cells = []
    
    # Markdown header
    cells.append({
        "cell_type": "markdown",
        "metadata": {},
        "source": [
            "# L2 Cache Batch Test Results - Line Chart Analysis\n",
            f"\n**Generated:** {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n",
            f"\n**Data Source:** {Path(summary_file).name}\n",
            f"\n**Total Tests:** {len(df)}\n"
        ]
    })
    
    # Import statements
    cells.append({
        "cell_type": "code",
        "execution_count": None,
        "metadata": {},
        "outputs": [],
        "source": [
            "import pandas as pd\n",
            "import numpy as np\n",
            "import matplotlib.pyplot as plt\n",
            "import seaborn as sns\n",
            "from IPython.display import display, HTML\n",
            "\n",
            "# Set style\n",
            "sns.set_style('whitegrid')\n",
            "plt.rcParams['figure.figsize'] = (14, 8)\n",
            "plt.rcParams['font.size'] = 10\n",
            "%matplotlib inline\n"
        ]
    })
    
    # Load data
    cells.append({
        "cell_type": "code",
        "execution_count": None,
        "metadata": {},
        "outputs": [],
        "source": [
            f"# Load batch test summary\n",
            f"df = pd.read_csv('{Path(summary_file).name}')\n",
            "\n",
            "print(f'Total test configurations: {len(df)}')\n",
            "print(f'Columns: {list(df.columns)}')\n",
            "display(df.head(10))\n",
            "\n",
            "# Convert numeric columns\n",
            "df['l2_on_tp_tps'] = pd.to_numeric(df['l2_on_tp_tps'], errors='coerce')\n",
            "df['l2_off_tp_tps'] = pd.to_numeric(df['l2_off_tp_tps'], errors='coerce')\n",
            "df['l2_on_ap_qps'] = pd.to_numeric(df['l2_on_ap_qps'], errors='coerce')\n",
            "df['l2_off_ap_qps'] = pd.to_numeric(df['l2_off_ap_qps'], errors='coerce')\n",
            "df['tp_change_pct'] = pd.to_numeric(df['tp_change_pct'], errors='coerce')\n",
            "df['ap_change_pct'] = pd.to_numeric(df['ap_change_pct'], errors='coerce')\n"
        ]
    })
    
    # Chart 1: TP Performance Comparison by TP:AP Ratio
    cells.append({
        "cell_type": "markdown",
        "metadata": {},
        "source": [
            "## Chart 1: TP Performance (TPS) by TP:AP Ratio\n",
            "\n",
            "Shows how TP performance changes with different TP:AP ratios:\n"
        ]
    })
    
    cells.append({
        "cell_type": "code",
        "execution_count": None,
        "metadata": {},
        "outputs": [],
        "source": [
            "# Prepare data for TP performance chart\n",
            "df_chart = df.copy()\n",
            "df_chart = df_chart.sort_values('tp_ap_ratio')\n",
            "\n",
            "fig, ax = plt.subplots(figsize=(14, 6))\n",
            "\n",
            "# Plot L2 ON and L2 OFF lines\n",
            "x_pos = range(len(df_chart))\n",
            "ax.plot(x_pos, df_chart['l2_off_tp_tps'], marker='o', linewidth=2, markersize=8, \n",
            "        label='L2 OFF', color='#e74c3c', linestyle='--')\n",
            "ax.plot(x_pos, df_chart['l2_on_tp_tps'], marker='s', linewidth=2, markersize=8, \n",
            "        label='L2 ON', color='#2ecc71')\n",
            "\n",
            "# Customize x-axis\n",
            "ax.set_xticks(x_pos)\n",
            "ax.set_xticklabels(df_chart['tp_ap_ratio'], rotation=45, ha='right')\n",
            "ax.set_xlabel('TP:AP Ratio', fontsize=12, fontweight='bold')\n",
            "ax.set_ylabel('TPS (Transactions Per Second)', fontsize=12, fontweight='bold')\n",
            "ax.set_title('TP Performance: L2 ON vs L2 OFF by TP:AP Ratio', fontsize=14, fontweight='bold', pad=20)\n",
            "ax.legend(loc='best', fontsize=11)\n",
            "ax.grid(True, alpha=0.3, linestyle=':')\n",
            "\n",
            "# Add value labels on points\n",
            "for i, (off_val, on_val) in enumerate(zip(df_chart['l2_off_tp_tps'], df_chart['l2_on_tp_tps'])):\n",
            "    ax.annotate(f'{int(off_val)}', (i, off_val), textcoords='offset points', \n",
            "                xytext=(0,10), ha='center', fontsize=8, color='#e74c3c')\n",
            "    ax.annotate(f'{int(on_val)}', (i, on_val), textcoords='offset points', \n",
            "                xytext=(0,-15), ha='center', fontsize=8, color='#2ecc71')\n",
            "\n",
            "plt.tight_layout()\n",
            "plt.show()\n"
        ]
    })
    
    # Chart 2: AP Performance Comparison by TP:AP Ratio
    cells.append({
        "cell_type": "markdown",
        "metadata": {},
        "source": [
            "## Chart 2: AP Performance (QPS) by TP:AP Ratio\n",
            "\n",
            "Shows how AP performance changes with different TP:AP ratios:\n"
        ]
    })
    
    cells.append({
        "cell_type": "code",
        "execution_count": None,
        "metadata": {},
        "outputs": [],
        "source": [
            "# Prepare data for AP performance chart\n",
            "fig, ax = plt.subplots(figsize=(14, 6))\n",
            "\n",
            "# Plot L2 ON and L2 OFF lines\n",
            "ax.plot(x_pos, df_chart['l2_off_ap_qps'], marker='o', linewidth=2, markersize=8, \n",
            "        label='L2 OFF', color='#e74c3c', linestyle='--')\n",
            "ax.plot(x_pos, df_chart['l2_on_ap_qps'], marker='s', linewidth=2, markersize=8, \n",
            "        label='L2 ON', color='#2ecc71')\n",
            "\n",
            "# Customize x-axis\n",
            "ax.set_xticks(x_pos)\n",
            "ax.set_xticklabels(df_chart['tp_ap_ratio'], rotation=45, ha='right')\n",
            "ax.set_xlabel('TP:AP Ratio', fontsize=12, fontweight='bold')\n",
            "ax.set_ylabel('QPS (Queries Per Second)', fontsize=12, fontweight='bold')\n",
            "ax.set_title('AP Performance: L2 ON vs L2 OFF by TP:AP Ratio', fontsize=14, fontweight='bold', pad=20)\n",
            "ax.legend(loc='best', fontsize=11)\n",
            "ax.grid(True, alpha=0.3, linestyle=':')\n",
            "\n",
            "# Add value labels on points\n",
            "for i, (off_val, on_val) in enumerate(zip(df_chart['l2_off_ap_qps'], df_chart['l2_on_ap_qps'])):\n",
            "    ax.annotate(f'{int(off_val)}', (i, off_val), textcoords='offset points', \n",
            "                xytext=(0,10), ha='center', fontsize=8, color='#e74c3c')\n",
            "    ax.annotate(f'{int(on_val)}', (i, on_val), textcoords='offset points', \n",
            "                xytext=(0,-15), ha='center', fontsize=8, color='#2ecc71')\n",
            "\n",
            "plt.tight_layout()\n",
            "plt.show()\n"
        ]
    })
    
    # Chart 3: Performance Improvement Percentage
    cells.append({
        "cell_type": "markdown",
        "metadata": {},
        "source": [
            "## Chart 3: Performance Improvement Percentage\n",
            "\n",
            "Shows the percentage change for TP and AP workloads:\n"
        ]
    })
    
    cells.append({
        "cell_type": "code",
        "execution_count": None,
        "metadata": {},
        "outputs": [],
        "source": [
            "# Performance improvement chart\n",
            "fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 6))\n",
            "\n",
            "# TP improvement\n",
            "ax1.plot(x_pos, df_chart['tp_change_pct'], marker='o', linewidth=2, markersize=8, \n",
            "         color='#3498db', label='TP Change %')\n",
            "ax1.axhline(y=0, color='black', linestyle='--', linewidth=1, alpha=0.5)\n",
            "ax1.set_xticks(x_pos)\n",
            "ax1.set_xticklabels(df_chart['tp_ap_ratio'], rotation=45, ha='right')\n",
            "ax1.set_xlabel('TP:AP Ratio', fontsize=11, fontweight='bold')\n",
            "ax1.set_ylabel('Change Percentage (%)', fontsize=11, fontweight='bold')\n",
            "ax1.set_title('TP Performance Change (L2 ON vs L2 OFF)', fontsize=12, fontweight='bold')\n",
            "ax1.grid(True, alpha=0.3, linestyle=':')\n",
            "ax1.legend()\n",
            "\n",
            "# Add value labels\n",
            "for i, val in enumerate(df_chart['tp_change_pct']):\n",
            "    ax1.annotate(f'{val:.1f}%', (i, val), textcoords='offset points', \n",
            "                xytext=(0,10), ha='center', fontsize=8)\n",
            "\n",
            "# AP improvement\n",
            "ax2.plot(x_pos, df_chart['ap_change_pct'], marker='s', linewidth=2, markersize=8, \n",
            "         color='#9b59b6', label='AP Change %')\n",
            "ax2.axhline(y=0, color='black', linestyle='--', linewidth=1, alpha=0.5)\n",
            "ax2.set_xticks(x_pos)\n",
            "ax2.set_xticklabels(df_chart['tp_ap_ratio'], rotation=45, ha='right')\n",
            "ax2.set_xlabel('TP:AP Ratio', fontsize=11, fontweight='bold')\n",
            "ax2.set_ylabel('Change Percentage (%)', fontsize=11, fontweight='bold')\n",
            "ax2.set_title('AP Performance Change (L2 ON vs L2 OFF)', fontsize=12, fontweight='bold')\n",
            "ax2.grid(True, alpha=0.3, linestyle=':')\n",
            "ax2.legend()\n",
            "\n",
            "# Add value labels\n",
            "for i, val in enumerate(df_chart['ap_change_pct']):\n",
            "    ax2.annotate(f'{val:.1f}%', (i, val), textcoords='offset points', \n",
            "                xytext=(0,10), ha='center', fontsize=8)\n",
            "\n",
            "plt.tight_layout()\n",
            "plt.show()\n"
        ]
    })
    
    # Chart 4: Combined Performance View
    cells.append({
        "cell_type": "markdown",
        "metadata": {},
        "source": [
            "## Chart 4: Combined Performance View\n",
            "\n",
            "Side-by-side comparison of TP and AP performance:\n"
        ]
    })
    
    cells.append({
        "cell_type": "code",
        "execution_count": None,
        "metadata": {},
        "outputs": [],
        "source": [
            "# Combined performance chart\n",
            "fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(14, 10))\n",
            "\n",
            "# TP Performance\n",
            "ax1.plot(x_pos, df_chart['l2_off_tp_tps'], marker='o', linewidth=2.5, markersize=10, \n",
            "         label='L2 OFF', color='#e74c3c', linestyle='--', alpha=0.8)\n",
            "ax1.plot(x_pos, df_chart['l2_on_tp_tps'], marker='s', linewidth=2.5, markersize=10, \n",
            "         label='L2 ON', color='#2ecc71', alpha=0.8)\n",
            "ax1.set_xticks(x_pos)\n",
            "ax1.set_xticklabels(df_chart['tp_ap_ratio'], rotation=45, ha='right')\n",
            "ax1.set_ylabel('TPS', fontsize=12, fontweight='bold')\n",
            "ax1.set_title('TP Performance Comparison', fontsize=13, fontweight='bold')\n",
            "ax1.legend(loc='best', fontsize=11)\n",
            "ax1.grid(True, alpha=0.3, linestyle=':')\n",
            "\n",
            "# AP Performance\n",
            "ax2.plot(x_pos, df_chart['l2_off_ap_qps'], marker='o', linewidth=2.5, markersize=10, \n",
            "         label='L2 OFF', color='#e74c3c', linestyle='--', alpha=0.8)\n",
            "ax2.plot(x_pos, df_chart['l2_on_ap_qps'], marker='s', linewidth=2.5, markersize=10, \n",
            "         label='L2 ON', color='#2ecc71', alpha=0.8)\n",
            "ax2.set_xticks(x_pos)\n",
            "ax2.set_xticklabels(df_chart['tp_ap_ratio'], rotation=45, ha='right')\n",
            "ax2.set_xlabel('TP:AP Ratio', fontsize=12, fontweight='bold')\n",
            "ax2.set_ylabel('QPS', fontsize=12, fontweight='bold')\n",
            "ax2.set_title('AP Performance Comparison', fontsize=13, fontweight='bold')\n",
            "ax2.legend(loc='best', fontsize=11)\n",
            "ax2.grid(True, alpha=0.3, linestyle=':')\n",
            "\n",
            "plt.tight_layout()\n",
            "plt.show()\n"
        ]
    })
    
    # Summary table
    cells.append({
        "cell_type": "markdown",
        "metadata": {},
        "source": [
            "## Summary Table\n",
            "\n",
            "Complete test results:\n"
        ]
    })
    
    cells.append({
        "cell_type": "code",
        "execution_count": None,
        "metadata": {},
        "outputs": [],
        "source": [
            "# Format summary table\n",
            "display_df = df_chart[['test_id', 'tp_ap_ratio', 'write_pct', 'l2_off_tp_tps', 'l2_on_tp_tps', \n",
            "                       'tp_change_pct', 'l2_off_ap_qps', 'l2_on_ap_qps', 'ap_change_pct']].copy()\n",
            "\n",
            "# Format columns\n",
            "display_df['l2_off_tp_tps'] = display_df['l2_off_tp_tps'].apply(lambda x: f'{int(x):,}')\n",
            "display_df['l2_on_tp_tps'] = display_df['l2_on_tp_tps'].apply(lambda x: f'{int(x):,}')\n",
            "display_df['l2_off_ap_qps'] = display_df['l2_off_ap_qps'].apply(lambda x: f'{int(x):,}')\n",
            "display_df['l2_on_ap_qps'] = display_df['l2_on_ap_qps'].apply(lambda x: f'{int(x):,}')\n",
            "display_df['tp_change_pct'] = display_df['tp_change_pct'].apply(lambda x: f'{x:+.1f}%')\n",
            "display_df['ap_change_pct'] = display_df['ap_change_pct'].apply(lambda x: f'{x:+.1f}%')\n",
            "\n",
            "display_df.columns = ['Test ID', 'TP:AP Ratio', 'Write %', 'TP L2 OFF', 'TP L2 ON', \n",
            "                      'TP Change', 'AP L2 OFF', 'AP L2 ON', 'AP Change']\n",
            "\n",
            "display(HTML(display_df.to_html(index=False, classes='table table-striped table-hover', escape=False)))\n"
        ]
    })
    
    # Create notebook structure
    notebook = {
        "cells": cells,
        "metadata": {
            "kernelspec": {
                "display_name": "Python 3",
                "language": "python",
                "name": "python3"
            },
            "language_info": {
                "name": "python",
                "version": "3.8.0"
            }
        },
        "nbformat": 4,
        "nbformat_minor": 4
    }
    
    # Write notebook
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(notebook, f, indent=2, ensure_ascii=False)
    
    print(f"✓ Generated Jupyter notebook: {output_file}")
    print(f"  To view: jupyter notebook {output_file}")
    return True


def main():
    parser = argparse.ArgumentParser(
        description='Generate Jupyter notebook with line charts from batch L2 test summary',
        formatter_class=argparse.RawDescriptionHelpFormatter
    )
    
    parser.add_argument(
        'summary_file',
        help='CSV file containing batch test summary'
    )
    
    parser.add_argument(
        '--output',
        '-o',
        default=None,
        help='Output notebook filename (default: batch_l2_report_TIMESTAMP.ipynb)'
    )
    
    args = parser.parse_args()
    
    # Validate input file
    summary_path = Path(args.summary_file)
    if not summary_path.exists():
        print(f"Error: File not found: {args.summary_file}", file=sys.stderr)
        return 1
    
    # Determine output filename
    if args.output:
        output_file = args.output
    else:
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        output_file = f"batch_l2_report_{timestamp}.ipynb"
    
    # Create notebook
    if create_notebook(str(summary_path), output_file):
        return 0
    else:
        return 1


if __name__ == '__main__':
    sys.exit(main())

