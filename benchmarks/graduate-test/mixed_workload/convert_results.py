import json
import csv
import sys
import re

def parse_scenario(scenario_str):
    # Format: Cols30-Read-Sel0.1
    match = re.match(r"Cols(\d+)-(\w+)-Sel([\d\.]+)", scenario_str)
    if match:
        cols = int(match.group(1))
        ratio = match.group(2)
        sel = float(match.group(3))
        
        table_type = "Narrow" if cols == 30 else "Wide"
        
        # Capitalize Ratio if needed
        ratio = ratio.capitalize()
        if ratio == "Balance": ratio = "Balance" # Ensure consistency
        
        return {
            "TableType": table_type,
            "WorkloadType": ratio,
            "Selectivity": sel
        }
    return None

def main():
    json_file = "benchmark_results.json"
    csv_file = "mixed_workload_results.csv"
    
    try:
        with open(json_file, 'r') as f:
            # Handle incomplete JSON (e.g. if still writing, it might lag closing bracket)
            content = f.read()
            if not content.strip().endswith(']'):
                # Try to fix truncated JSON by adding "]"
                # Remove trailing comma if present
                content = content.strip()
                if content.endswith(','):
                    content = content[:-1]
                content += "]"
            
            data = json.loads(content)
    except Exception as e:
        print(f"Error loading JSON: {e}")
        sys.exit(1)

    headers = ["场景", "表类型", "负载类型", "选择率", "模式", "耗时(秒)"]
    rows = []
    
    # Translation Maps
    table_map = {"Narrow": "窄表(30列)", "Wide": "宽表(70列)"}
    workload_map = {"Read": "读密集", "Balance": "均衡", "Write": "写密集"}
    method_map = {"Row": "行存", "Column": "列存"}
    
    for entry in data:
        meta = parse_scenario(entry['scenario'])
        if meta:
            # Capitalize method
            method_en = entry['method'].capitalize()
            workload_en = meta['WorkloadType']
            table_en = meta['TableType']
            
            # Translate
            table_cn = table_map.get(table_en, table_en)
            workload_cn = workload_map.get(workload_en, workload_en)
            method_cn = method_map.get(method_en, method_en)
            
            duration = entry['duration_sec']
            
            # Reconstruct standardized scenario ID (Keep English ID for reference or make Chinese?)
            # Let's keep a Chinese Scenario ID
            scenario_id = f"{table_cn}-{workload_cn}-Sel{meta['Selectivity']}"
            
            rows.append([
                scenario_id,
                table_cn,
                workload_cn,
                meta['Selectivity'],
                method_cn,
                duration
            ])
            
    with open(csv_file, 'w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(headers)
        writer.writerows(rows)
        
    print(f"Converted {len(rows)} records to {csv_file}")

if __name__ == "__main__":
    main()
