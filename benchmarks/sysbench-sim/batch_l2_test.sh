#!/bin/bash

# Batch L2 Cache Validation Test Script
# Tests L2 cache effectiveness across different TP:AP ratios and write percentages

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Test parameters
TABLE_SIZE=50000  # Increased for more realistic performance
TIME=20           # Test time (reduced for batch testing)
PRELOAD_TIME=15   # Preload time
THREADS_TOTAL=16

# Output directory
OUTPUT_DIR="$SCRIPT_DIR/results/batch_l2_tests"
mkdir -p "$OUTPUT_DIR"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
SUMMARY_FILE="$OUTPUT_DIR/l2_batch_summary_${TIMESTAMP}.csv"

# Initialize summary CSV
echo "test_id,tp_threads,ap_threads,tp_ap_ratio,write_pct,ap_range_frac,l2_on_tp_tps,l2_off_tp_tps,tp_change_pct,l2_on_ap_qps,l2_off_ap_qps,ap_change_pct" > "$SUMMARY_FILE"

echo "========================================="
echo "Batch L2 Cache Validation Tests"
echo "========================================="
echo "Table size: $TABLE_SIZE"
echo "Test duration: ${TIME}s"
echo "Output directory: $OUTPUT_DIR"
echo ""

# Test configurations
# Format: tp_threads:ap_threads:write_pct:ap_range_frac:test_id
test_configs=(
    "8:8:20:0.2:test1_balanced_1_1"
    "12:4:20:0.2:test2_tp_heavy_3_1"
    "4:12:20:0.2:test3_ap_heavy_1_3"
    "10:6:20:0.2:test4_tp_moderate_5_3"
    "6:10:20:0.2:test5_ap_moderate_3_5"
    "8:8:30:0.2:test6_balanced_30write"
    "8:8:50:0.2:test7_balanced_50write"
    "8:8:20:0.1:test8_small_range"
    "8:8:20:0.3:test9_large_range"
)

test_count=0
for config in "${test_configs[@]}"; do
    IFS=':' read -r tp_threads ap_threads write_pct ap_range_frac test_id <<< "$config"
    
    test_count=$((test_count + 1))
    tp_ap_ratio="${tp_threads}:${ap_threads}"
    
    echo "========================================="
    echo "Test $test_count/${#test_configs[@]}: $test_id"
    echo "========================================="
    echo "TP threads: $tp_threads, AP threads: $ap_threads"
    echo "TP:AP ratio: $tp_ap_ratio"
    echo "Write percentage: $write_pct%"
    echo "AP range fraction: $ap_range_frac"
    echo ""
    
    # Run test
    output_file="$OUTPUT_DIR/${test_id}_${TIMESTAMP}.csv"
    
    ./test_l2_cache.sh \
        --workload mixed \
        --tp-threads "$tp_threads" \
        --ap-threads "$ap_threads" \
        --table-size "$TABLE_SIZE" \
        --time "$TIME" \
        --preload-time "$PRELOAD_TIME" \
        --write-pct "$write_pct" \
        --ap-range-frac "$ap_range_frac" \
        --output "$output_file" \
        2>&1 | tee "$OUTPUT_DIR/${test_id}_${TIMESTAMP}.log"
    
    # Extract results from CSV
    if [ -f "$output_file" ]; then
        # Read L2 ON results
        l2_on_line=$(grep "^l2_on," "$output_file" | head -1)
        l2_off_line=$(grep "^l2_off," "$output_file" | head -1)
        
        if [ -n "$l2_on_line" ] && [ -n "$l2_off_line" ]; then
            # Parse CSV (format: test_name,workload,l2_enabled,threads,tp_threads,ap_threads,tp_ops,tp_ops_per_sec,ap_ops,ap_ops_per_sec,total_ops,total_ops_per_sec,tp_improvement_pct,ap_improvement_pct)
            IFS=',' read -r -a l2_on_fields <<< "$l2_on_line"
            IFS=',' read -r -a l2_off_fields <<< "$l2_off_line"
            
            l2_on_tp_tps=${l2_on_fields[7]:-0}
            l2_on_ap_qps=${l2_on_fields[9]:-0}
            l2_off_tp_tps=${l2_off_fields[7]:-0}
            l2_off_ap_qps=${l2_off_fields[9]:-0}
            tp_change_pct=${l2_on_fields[12]:-0}
            ap_change_pct=${l2_on_fields[13]:-0}
            
            # Write to summary
            echo "$test_id,$tp_threads,$ap_threads,$tp_ap_ratio,$write_pct,$ap_range_frac,$l2_on_tp_tps,$l2_off_tp_tps,$tp_change_pct,$l2_on_ap_qps,$l2_off_ap_qps,$ap_change_pct" >> "$SUMMARY_FILE"
            
            echo ""
            echo "Results:"
            echo "  TP: L2 ON=$l2_on_tp_tps TPS, L2 OFF=$l2_off_tp_tps TPS, Change=$tp_change_pct%"
            echo "  AP: L2 ON=$l2_on_ap_qps QPS, L2 OFF=$l2_off_ap_qps QPS, Change=$ap_change_pct%"
        else
            echo "Warning: Could not parse results from $output_file"
        fi
    fi
    
    echo ""
    echo "----------------------------------------"
    echo ""
    sleep 2
done

echo "========================================="
echo "All Tests Completed!"
echo "========================================="
echo "Summary file: $SUMMARY_FILE"
echo ""
echo "Summary Results:"
echo "=================="
cat "$SUMMARY_FILE" | column -t -s,

