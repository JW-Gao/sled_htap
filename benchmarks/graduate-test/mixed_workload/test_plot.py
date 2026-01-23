import json
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import re
import sys

def parse_scenario(scenario_str):
    # Format: Cols30-Read-Sel0.1
    # Note: Ratio can be Read, Balance, Write
    # Sel is float
    
    # Regex might be safer
    match = re.match(r"Cols(\d+)-(\w+)-Sel([\d\.]+)", scenario_str)
    if match:
        return {
            "Columns": int(match.group(1)),
            "Ratio": match.group(2),
            "Selectivity": float(match.group(3))
        }
    return None

def main():
    json_file = "dummy_results.json"
    try:
        with open(json_file, 'r') as f:
            data = json.load(f)
    except FileNotFoundError:
        print(f"Error: {json_file} not found.")
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error parsing JSON: {e}")
        print("Ensure the benchmark run has completed successfully.")
        sys.exit(1)

    # Process data
    records = []
    for entry in data:
        meta = parse_scenario(entry['scenario'])
        if meta:
            rec = meta.copy()
            rec['Method'] = entry['method'] # Row or Column
            rec['Duration'] = entry['duration_sec']
            rec['TotalOps'] = entry['total_ops']
            records.append(rec)
    
    df = pd.DataFrame(records)
    
    if df.empty:
        print("No valid data records found.")
        sys.exit(1)

    print("Data loaded. Records:", len(df))
    print(df.head())

    # Plotting
    sns.set_theme(style="whitegrid")
    
    # We want a grid of plots: 
    # Rows: Columns (30, 70)
    # Cols: Ratio (Read, Balance, Write) -- ordering might need to be fixed
    
    # Define order for Ratio
    ratio_order = ["Read", "Balance", "Write"]
    
    g = sns.FacetGrid(df, row="Columns", col="Ratio", hue="Method", 
                      col_order=ratio_order, height=4, aspect=1.2,
                      sharey=False) # ShareY=False because duration might vary wildly
    
    g.map(sns.lineplot, "Selectivity", "Duration", marker="o")
    g.add_legend()
    
    # Titles and labels
    g.set_axis_labels("Selectivity (Fraction of Data)", "Time to Complete 500k Ops (s)")
    g.set_titles("Cols: {row_name} | Workload: {col_name}")
    
    output_img = "mixed_workload_results.png"
    plt.savefig(output_img)
    print(f"Plot saved to {output_img}")

if __name__ == "__main__":
    main()
