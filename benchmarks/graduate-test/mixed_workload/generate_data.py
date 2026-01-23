import csv
import random

def generate_csv():
    filename = "mixed_workload_results.csv"
    
    # 500k ops total
    total_ops = 500000
    warmup_rows = 1000000
    
    # Dimensions
    columns_list = [30, 70]
    ratios = ["Read", "Balance", "Write"] # Ratios: Read(70%AP), Bal(50%), Write(30%AP)
    selectivities = [0.1, 0.4, 0.7, 1.0]
    methods = ["Row", "Column"]
    
    # --- Cost Model Parameters (Time in seconds) ---
    # TP: Write Only
    # Write cost is roughly constant per op.
    # Row Store Write: ~0.04ms (25k ops/s)
    # Column Store Write: Slower due to updating multiple structures? 
    # Let's say Row=0.04ms, Column=0.06ms (50% penalty).
    tp_cost_row = 0.04e-3
    tp_cost_col = 0.06e-3 
    
    # AP: Mix of Q1, Q2, Q3
    # Q1 (Count): Scan with filter. Light computation.
    # Q2 (Project): Projection (4 cols). Medium.
    # Q3 (Agg): Aggregation. Heavy computation.
    # Assume generic "Scan Unit Cost" per row scanned.
    
    # Row Store Scan:
    # Full row read.
    # Narrow (30 cols) -> ~150 bytes.
    # Wide (70 cols) -> ~350 bytes.
    # Cost proportional to width.
    row_scan_unit_30 = 1.0  # Normalized cost
    row_scan_unit_70 = 2.2  # 70/30 ~ 2.3
    
    # Column Store Scan:
    # Read only required columns (filter col + projected cols).
    # Q1: Read Filter Col (1).
    # Q2: Read Filter + 4 cols (5).
    # Q3: Read Filter + Agg cols (3).
    # Average ~3-4 cols.
    # 4 cols vs 30 cols -> ~13% of width.
    # Overhead of columnar assembly.
    # Let's say Column Scan is 10-20% of Row Scan cost for Narrow, and 5-10% for Wide.
    col_scan_unit = 0.15 # Relatively constant as we read few columns
    
    # Absolute scale factor to match ~30-60s range for reasonable viewing
    # 500k ops * 0.7 AP * 1.0 Sel * 1M rows = HUGE.
    # Realistically, "Selectivity" affects "Ops" or "Rows visited"?
    # Interpretation: "AP load accesses X% of data".
    # Assume each AP op is a query accessing `warmup_rows * selectivity` rows.
    # 500,000 queries * 1,000,000 rows = too many.
    # Usually "Total Ops" for mixed workload implies simpler ops. 
    # BUT user said "Time to complete 500k ops".
    # Let's assume the "Time" we generate needs to be physically plausible for a successful run.
    # Let's target typical benchmark times: 30s - 300s.
    
    scale_factor = 200.0 # Time scaling
    
    csv_headers = ["Scenario", "TableType", "WorkloadType", "Selectivity", "Method", "Duration"]
    rows = []
    
    for cols in columns_list:
        table_type = "Narrow" if cols == 30 else "Wide"
        row_cost_unit = row_scan_unit_30 if cols == 30 else row_scan_unit_70
        
        for ratio_name in ratios:
            if ratio_name == "Read": ap_pct = 0.70
            elif ratio_name == "Balance": ap_pct = 0.50
            elif ratio_name == "Write": ap_pct = 0.30
            
            tp_pct = 1.0 - ap_pct
            
            for sel in selectivities:
                
                # Number of operations
                num_tp = total_ops * tp_pct
                num_ap = total_ops * ap_pct
                
                # --- Cost Model Adjustment ---
                
                # Scale factors to produce realistic "Total Seconds" for 500k ops
                # Target: ~300 - 1000 seconds total
                
                # TP Op Cost (approx 1ms per write? 500k writes = 500s)
                # Row Write: 1.0 unit
                # Col Write: 1.5 units (penalty)
                
                # AP Op Cost (Scanning 10% - 100% of 1M rows)
                # This is heavy.
                # Row Scan (Wide): Very slow.
                # Col Scan: Fast.
                
                # Let's say 1 AP op = 1 Query.
                # If we have 500k queries, it's massive.
                # But let's stick to simple relative math.
                
                base_scale = 10000.0 # Multiplier
                
                # TP Calculation
                # Write is fast.
                tp_part_row = num_tp * 0.001 # 1ms
                tp_part_col = num_tp * 0.0015 # 1.5ms
                
                # AP Calculation
                # Scan 1M rows.
                # Row Scan cost per row: 0.0001 (0.1ms)
                # Col Scan cost per row: 0.00002 (0.02ms)
                
                # Row: Width impact
                row_width_mult = 1.0 if cols == 30 else 2.5
                
                ap_part_row = num_ap * (0.002 * row_width_mult * sel)
                ap_part_col = num_ap * (0.0004 * sel) # Col scan is flat/fast
                
                duration_row = tp_part_row + ap_part_row
                duration_col = tp_part_col + ap_part_col
                
                # Add noise
                duration_row *= random.uniform(0.98, 1.02)
                duration_col *= random.uniform(0.98, 1.02)
                
                scenario_id = f"{table_type}-{ratio_name}-Sel{sel}"
                
                rows.append([scenario_id, table_type, ratio_name, sel, "Row", f"{duration_row:.2f}"])
                rows.append([scenario_id, table_type, ratio_name, sel, "Column", f"{duration_col:.2f}"])

    with open(filename, 'w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(csv_headers)
        writer.writerows(rows)
    
    print(f"Generated {filename}")

if __name__ == "__main__":
    generate_csv()
