#!/usr/bin/env python3
"""
Generate Jupyter Notebook Report from L2 Cache Test Results

This script reads CSV test results and generates a Jupyter notebook with
tables and visualizations.

Usage:
    python generate_l2_report.py <csv_file> [--output notebook.ipynb]
    python generate_l2_report.py results/*.csv  # Process all CSV files
"""

import argparse
import json
import sys
from pathlib import Path
from datetime import datetime
import pandas as pd


def create_notebook(csv_files, output_file):
    """Create a Jupyter notebook from CSV test results."""
    
    # Read all CSV files
    dfs = []
    for csv_file in csv_files:
        try:
            df = pd.read_csv(csv_file)
            df['source_file'] = Path(csv_file).name
            dfs.append(df)
        except Exception as e:
            print(f"Warning: Could not read {csv_file}: {e}", file=sys.stderr)
    
    if not dfs:
        print("Error: No valid CSV files found", file=sys.stderr)
        return False
    
    # Combine all dataframes
    combined_df = pd.concat(dfs, ignore_index=True)
    
    # Create notebook cells
    cells = []
    
    # Markdown header
    cells.append({
        "cell_type": "markdown",
        "metadata": {},
        "source": [
            "# L2 Cache Performance Test Report\n",
            f"\n**Generated:** {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n",
            f"\n**Data Sources:** {', '.join(Path(f).name for f in csv_files)}\n"
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
            "plt.rcParams['figure.figsize'] = (12, 6)\n",
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
            "# Load data\n",
            f"df = pd.read_csv('{Path(csv_files[0]).name}')\n",
            "\n",
            "# Normalize column names for compatibility\n",
            "# Handle different CSV formats\n",
            "if 'ops_per_sec' not in df.columns and 'total_ops_per_sec' in df.columns:\n",
            "    df['ops_per_sec'] = df['total_ops_per_sec']\n",
            "\n",
            "# Handle l2_enabled column\n",
            "if 'l2_enabled' not in df.columns:\n",
            "    # Try to infer from other columns\n",
            "    if 'config' in df.columns:\n",
            "        df['l2_enabled'] = df['config'].apply(lambda x: 'yes' if 'l2_on' in str(x) else 'no')\n",
            "    else:\n",
            "        # Default: assume all tests have L2 enabled if column doesn't exist\n",
            "        df['l2_enabled'] = 'yes'\n",
            "\n",
            "# Display basic info\n",
            "print(f'Total records: {len(df)}')\n",
            "print(f'Columns: {list(df.columns)}')\n",
            "df.head()\n"
        ]
    })
    
    # Summary statistics
    cells.append({
        "cell_type": "markdown",
        "metadata": {},
        "source": [
            "## Summary Statistics\n",
            "\n",
            "Overview of test results:\n"
        ]
    })
    
    cells.append({
        "cell_type": "code",
        "execution_count": None,
        "metadata": {},
        "outputs": [],
        "source": [
            "# Summary statistics\n",
            "summary = df.groupby(['l2_enabled', 'workload']).agg({\n",
            "    'ops_per_sec': ['mean', 'std', 'min', 'max', 'count']\n",
            "}).round(2)\n",
            "summary.columns = ['Mean', 'Std', 'Min', 'Max', 'Count']\n",
            "display(summary)\n"
        ]
    })
    
    # Performance comparison table
    cells.append({
        "cell_type": "markdown",
        "metadata": {},
        "source": [
            "## Performance Comparison: L2 ON vs L2 OFF\n",
            "\n",
            "Direct comparison of L2 cache enabled vs disabled:\n"
        ]
    })
    
    cells.append({
        "cell_type": "code",
        "execution_count": None,
        "metadata": {},
        "outputs": [],
        "source": [
            "# Create comparison table\n",
            "comparison = df.pivot_table(\n",
            "    index=['workload', 'tp_threads', 'ap_threads'],\n",
            "    columns='l2_enabled',\n",
            "    values='ops_per_sec',\n",
            "    aggfunc='mean'\n",
            ").round(2)\n",
            "\n",
            "# Calculate improvement percentage\n",
            "if 'yes' in comparison.columns and 'no' in comparison.columns:\n",
            "    comparison['improvement_pct'] = (\n",
            "        (comparison['yes'] - comparison['no']) / comparison['no'] * 100\n",
            "    ).round(2)\n",
            "\n",
            "comparison.columns.name = None\n",
            "comparison.index.names = ['Workload', 'TP Threads', 'AP Threads']\n",
            "comparison = comparison.reset_index()\n",
            "\n",
            "# Format for display\n",
            "display(HTML(comparison.to_html(index=False, classes='table table-striped')))\n"
        ]
    })
    
    # Visualization: Bar chart by workload
    cells.append({
        "cell_type": "markdown",
        "metadata": {},
        "source": [
            "## Performance by Workload Type\n",
            "\n",
            "Comparison of L2 ON vs L2 OFF across different workload types:\n"
        ]
    })
    
    cells.append({
        "cell_type": "code",
        "execution_count": None,
        "metadata": {},
        "outputs": [],
        "source": [
            "# Bar chart: Performance by workload\n",
            "fig, ax = plt.subplots(figsize=(10, 6))\n",
            "\n",
            "workload_data = df.groupby(['workload', 'l2_enabled'])['ops_per_sec'].mean().unstack()\n",
            "workload_data.plot(kind='bar', ax=ax, color=['#2ecc71', '#e74c3c'])\n",
            "\n",
            "ax.set_xlabel('Workload Type', fontsize=12)\n",
            "ax.set_ylabel('Operations per Second', fontsize=12)\n",
            "ax.set_title('Performance Comparison: L2 ON vs L2 OFF by Workload', fontsize=14, fontweight='bold')\n",
            "ax.legend(['L2 ON', 'L2 OFF'], loc='upper left')\n",
            "ax.grid(axis='y', alpha=0.3)\n",
            "\n",
            "plt.xticks(rotation=0)\n",
            "plt.tight_layout()\n",
            "plt.show()\n"
        ]
    })
    
    # Visualization: Improvement percentage
    cells.append({
        "cell_type": "markdown",
        "metadata": {},
        "source": [
            "## Performance Improvement Percentage\n",
            "\n",
            "How much faster (or slower) L2 ON is compared to L2 OFF:\n"
        ]
    })
    
    cells.append({
        "cell_type": "code",
        "execution_count": None,
        "metadata": {},
        "outputs": [],
        "source": [
            "# Calculate improvement for each test\n",
            "improvement_df = df[df['improvement_pct'].notna()].copy()\n",
            "\n",
            "if len(improvement_df) > 0:\n",
            "    fig, ax = plt.subplots(figsize=(12, 6))\n",
            "    \n",
            "    # Group by workload\n",
            "    for workload in improvement_df['workload'].unique():\n",
            "        workload_data = improvement_df[improvement_df['workload'] == workload]\n",
            "        ax.bar(\n",
            "            workload_data.index,\n",
            "            workload_data['improvement_pct'],\n",
            "            label=workload.upper(),\n",
            "            alpha=0.7\n",
            "        )\n",
            "    \n",
            "    ax.axhline(y=0, color='black', linestyle='--', linewidth=1)\n",
            "    ax.set_xlabel('Test Index', fontsize=12)\n",
            "    ax.set_ylabel('Improvement (%)', fontsize=12)\n",
            "    ax.set_title('L2 Cache Performance Improvement by Workload', fontsize=14, fontweight='bold')\n",
            "    ax.legend()\n",
            "    ax.grid(axis='y', alpha=0.3)\n",
            "    \n",
            "    plt.tight_layout()\n",
            "    plt.show()\n",
            "else:\n",
            "    print('No improvement data available (need both L2 ON and L2 OFF tests)')\n"
        ]
    })
    
    # Mixed workload analysis
    cells.append({
        "cell_type": "markdown",
        "metadata": {},
        "source": [
            "## Mixed Workload Analysis\n",
            "\n",
            "Performance analysis for mixed TP:AP workloads:\n"
        ]
    })
    
    cells.append({
        "cell_type": "code",
        "execution_count": None,
        "metadata": {},
        "outputs": [],
        "source": [
            "# Filter mixed workload data\n",
            "mixed_df = df[df['workload'] == 'mixed'].copy()\n",
            "\n",
            "if len(mixed_df) > 0:\n",
            "    # Create TP:AP ratio column\n",
            "    mixed_df['tp_ap_ratio'] = (\n",
            "        mixed_df['tp_threads'].astype(str) + ':' + \n",
            "        mixed_df['ap_threads'].astype(str)\n",
            "    )\n",
            "    \n",
            "    # Pivot table\n",
            "    mixed_pivot = mixed_df.pivot_table(\n",
            "        index='tp_ap_ratio',\n",
            "        columns='l2_enabled',\n",
            "        values='ops_per_sec',\n",
            "        aggfunc='mean'\n",
            "    ).round(2)\n",
            "    \n",
            "    if 'yes' in mixed_pivot.columns and 'no' in mixed_pivot.columns:\n",
            "        mixed_pivot['improvement_pct'] = (\n",
            "            (mixed_pivot['yes'] - mixed_pivot['no']) / mixed_pivot['no'] * 100\n",
            "        ).round(2)\n",
            "    \n",
            "    display(HTML(mixed_pivot.to_html(classes='table table-striped')))\n",
            "    \n",
            "    # Visualization\n",
            "    fig, ax = plt.subplots(figsize=(12, 6))\n",
            "    mixed_pivot[['yes', 'no']].plot(kind='bar', ax=ax, color=['#2ecc71', '#e74c3c'])\n",
            "    ax.set_xlabel('TP:AP Ratio', fontsize=12)\n",
            "    ax.set_ylabel('Operations per Second', fontsize=12)\n",
            "    ax.set_title('Mixed Workload Performance: L2 ON vs L2 OFF', fontsize=14, fontweight='bold')\n",
            "    ax.legend(['L2 ON', 'L2 OFF'], loc='upper left')\n",
            "    ax.grid(axis='y', alpha=0.3)\n",
            "    plt.xticks(rotation=45, ha='right')\n",
            "    plt.tight_layout()\n",
            "    plt.show()\n",
            "else:\n",
            "    print('No mixed workload data available')\n"
        ]
    })
    
    # Detailed table
    cells.append({
        "cell_type": "markdown",
        "metadata": {},
        "source": [
            "## Detailed Results Table\n",
            "\n",
            "Complete test results with all parameters:\n"
        ]
    })
    
    cells.append({
        "cell_type": "code",
        "execution_count": None,
        "metadata": {},
        "outputs": [],
        "source": [
            "# Display full table with formatting\n",
            "display_df = df.copy()\n",
            "\n",
            "# Format columns for better display\n",
            "if 'ops_per_sec' in display_df.columns:\n",
            "    display_df['ops_per_sec'] = display_df['ops_per_sec'].apply(lambda x: f'{x:,.0f}' if pd.notna(x) else 'N/A')\n",
            "if 'improvement_pct' in display_df.columns:\n",
            "    display_df['improvement_pct'] = display_df['improvement_pct'].apply(\n",
            "        lambda x: f'{x:+.2f}%' if pd.notna(x) else 'N/A'\n",
            "    )\n",
            "\n",
            "display(HTML(display_df.to_html(index=False, classes='table table-striped table-hover')))\n"
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
        description='Generate Jupyter notebook report from L2 cache test results',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Generate report from single CSV file
  python generate_l2_report.py results/l2_test_20241117_120000.csv

  # Generate report with custom output name
  python generate_l2_report.py results/*.csv --output l2_analysis.ipynb

  # Process all CSV files in results directory
  python generate_l2_report.py results/*.csv
        """
    )
    
    parser.add_argument(
        'csv_files',
        nargs='+',
        help='CSV file(s) containing test results'
    )
    
    parser.add_argument(
        '--output',
        '-o',
        default=None,
        help='Output notebook filename (default: l2_report_TIMESTAMP.ipynb)'
    )
    
    args = parser.parse_args()
    
    # Validate CSV files
    csv_files = []
    for csv_file in args.csv_files:
        path = Path(csv_file)
        if not path.exists():
            print(f"Warning: File not found: {csv_file}", file=sys.stderr)
            continue
        csv_files.append(str(path))
    
    if not csv_files:
        print("Error: No valid CSV files found", file=sys.stderr)
        return 1
    
    # Determine output filename
    if args.output:
        output_file = args.output
    else:
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        output_file = f"l2_report_{timestamp}.ipynb"
    
    # Create notebook
    if create_notebook(csv_files, output_file):
        return 0
    else:
        return 1


if __name__ == '__main__':
    sys.exit(main())

