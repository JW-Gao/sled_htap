import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import sys
import matplotlib.font_manager as fm

def main():
    # Set Chinese Font: WenQuanYi Micro Hei
    try:
        plt.rcParams['font.sans-serif'] = ['WenQuanYi Micro Hei', 'Noto Sans CJK SC', 'Droid Sans Fallback', 'SimHei']
        plt.rcParams['axes.unicode_minus'] = False
    except Exception as e:
        print(f"Warning: Font setting failed: {e}")

    csv_file = "mixed_workload_results.csv"
    if len(sys.argv) > 1:
        csv_file = sys.argv[1]
        
    try:
        df = pd.read_csv(csv_file)
    except Exception as e:
        print(f"Error loading CSV: {e}")
        sys.exit(1)

    if df.empty:
        print("No valid data.")
        sys.exit(1)

    print(f"Loaded {len(df)} records.")

    # Updated Translation Maps (Actually already translated in CSV, but need for plotting order/colors)
    # The CSV columns are now: ["场景", "表类型", "负载类型", "选择率", "模式", "耗时(秒)"]
    
    # Check headers
    if "场景" not in df.columns:
        print("CSV headers mismatch. Expected Chinese headers.")
        print("Columns found:", df.columns)
        sys.exit(1)

    print(f"Loaded {len(df)} records.")

    # Setup Plot
    sns.set_theme(style="whitegrid", font="WenQuanYi Micro Hei")
    
    # Define order (Must match CSV values)
    ratio_order = ["读密集", "均衡", "写密集"]
    table_order = ["窄表(30列)", "宽表(70列)"]
    
    # Create FacetGrid: Row=表类型, Col=负载类型, Hue=模式
    g = sns.FacetGrid(df, row="表类型", col="负载类型", hue="模式", 
                      col_order=ratio_order, row_order=table_order,
                      height=4, aspect=1.2, sharey=False)
    
    g.map(sns.lineplot, "选择率", "耗时(秒)", marker="o")
    g.add_legend(title="存储模式")
    
    # Titles and labels (Already handled by column mapping but explicit setting is safe)
    g.set_axis_labels("选择率 (Selectivity)", "耗时 (秒)")
    # Adjust titles for facets
    g.set_titles(row_template="{row_name}", col_template="{col_name}")

    output_img = "mixed_workload_results.png"
    plt.savefig(output_img, dpi=300)
    print(f"Plot saved to {output_img}")

if __name__ == "__main__":
    main()
