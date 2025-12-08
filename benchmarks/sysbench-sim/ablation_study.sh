#!/bin/bash

# Ablation Study Script for HTAP System
# This script runs three sets of experiments:
# 1. All optimizations enabled (L2 columnar + Pull-Push), varying TP:AP ratios
# 2. L2 columnar storage on/off, varying TP:AP ratios
# 3. Pull-Push strategy on/off (with L2 enabled), varying TP:AP ratios

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCH_BIN="$SCRIPT_DIR/target/release/sysbench-sim"

# Default parameters
TABLE_SIZE=100000
TIME=5
PRELOAD_TIME=10
THREADS_TOTAL=4
# HOTSPOT_FRAC is optional - not set by default (let system discover hotspots naturally)
# Use --hotspot-frac X to enable cheat interface for artificial hotspot
AP_RANGE_FRAC=0.1
AP_COMPUTE_ITERS=256
GRADE_COL=0

# Output file
OUTPUT_DIR="$SCRIPT_DIR/results"
mkdir -p "$OUTPUT_DIR"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
OUTPUT_FILE="$OUTPUT_DIR/ablation_${TIMESTAMP}.csv"

# Initialize CSV header
echo "experiment,config,tp_ap_ratio,l2_enabled,pull_enabled,tp_threads,ap_threads,tp_ops,ap_ops,tp_ops_per_sec,ap_ops_per_sec,total_ops_per_sec" > "$OUTPUT_FILE"

# Helper function to preload data
preload_data() {
    local db_path="$1"
    local use_direct_tree="${2:-false}"  # Optional: true = use direct Tree, false = use OverlayTree
    echo "Preloading data into $db_path (direct_tree=$use_direct_tree)..."
    rm -rf "$db_path"
    local preload_args=(
        --path "$db_path"
        --workload tp
        --time $PRELOAD_TIME
        --threads $THREADS_TOTAL
        --write-pct 100
        --table-size $TABLE_SIZE
    )
    # Use hotspot-frac only if specified for preload
    if [ -n "${HOTSPOT_FRAC:-}" ]; then
        preload_args+=(--hotspot-frac "$HOTSPOT_FRAC")
    fi
    # Add --use-direct-tree if needed
    if [ "$use_direct_tree" = "true" ]; then
        preload_args+=(--use-direct-tree)
    fi
    cargo run --release --manifest-path "$SCRIPT_DIR/Cargo.toml" -- "${preload_args[@]}" > /dev/null 2>&1
}

# Helper function to run a single benchmark
run_benchmark() {
    local db_path="$1"
    local workload="$2"
    local tp_threads="$3"
    local ap_threads="$4"
    local pull="$5"
    local use_overlay="$6"  # true = use OverlayTree, false = use Tree directly
    local experiment="$7"
    local config="$8"
    local tp_ap_ratio="$9"
    
    # Build command
    local cmd_args=(
        --path "$db_path"
        --workload "$workload"
        --time $TIME
        --table-size $TABLE_SIZE
        --grade-col $GRADE_COL
        --tp-threads $tp_threads
        --ap-threads $ap_threads
        --ap-range-frac $AP_RANGE_FRAC
        --ap-compute-iters $AP_COMPUTE_ITERS
    )
    
    # Add hotspot-frac only if specified (cheat interface, not used by default)
    if [ -n "${HOTSPOT_FRAC:-}" ]; then
        cmd_args+=(--hotspot-frac "$HOTSPOT_FRAC")
    fi
    
    if [ "$pull" = "true" ]; then
        cmd_args+=(--pull)
    else
        cmd_args+=(--no-pull)
    fi
    
    if [ "$use_overlay" = "false" ]; then
        cmd_args+=(--use-direct-tree)
    fi
    
    # Create result file for this benchmark
    local result_file="${db_path}.result"
    
    # Run benchmark and save output to both terminal and file
    local output=$(cargo run --release --manifest-path "$SCRIPT_DIR/Cargo.toml" -- "${cmd_args[@]}" 2>&1 | tee "$result_file")
    
    # Parse results
    local tp_ops=0
    local ap_ops=0
    local tp_ops_per_sec=0
    local ap_ops_per_sec=0
    
    if [ "$workload" = "mixed" ]; then
        tp_ops=$(echo "$output" | grep -oP 'TP: ops=\K[0-9]+' || echo "0")
        ap_ops=$(echo "$output" | grep -oP 'AP: ops=\K[0-9]+' || echo "0")
        tp_ops_per_sec=$(echo "$output" | grep -oP 'TP: ops=[0-9]+ ops/s=\K[0-9]+' || echo "0")
        ap_ops_per_sec=$(echo "$output" | grep -oP 'AP: ops=[0-9]+ ops/s=\K[0-9]+' || echo "0")
    elif [ "$workload" = "tp" ]; then
        tp_ops=$(echo "$output" | grep -oP 'ops=\K[0-9]+' | head -1 || echo "0")
        tp_ops_per_sec=$(echo "$output" | grep -oP 'ops/s=\K[0-9]+' | head -1 || echo "0")
    elif [ "$workload" = "ap" ]; then
        ap_ops=$(echo "$output" | grep -oP 'ops=\K[0-9]+' | head -1 || echo "0")
        ap_ops_per_sec=$(echo "$output" | grep -oP 'ops/s=\K[0-9]+' | head -1 || echo "0")
    fi
    
    # Calculate total using awk to avoid bc dependency
    local total_ops_per_sec=$(awk "BEGIN {printf \"%.0f\", $tp_ops_per_sec + $ap_ops_per_sec}")
    local l2_enabled=$([ "$use_overlay" = "true" ] && echo "yes" || echo "no")
    local pull_enabled=$([ "$pull" = "true" ] && echo "yes" || echo "no")
    
    # Write to CSV
    echo "$experiment,$config,$tp_ap_ratio,$l2_enabled,$pull_enabled,$tp_threads,$ap_threads,$tp_ops,$ap_ops,$tp_ops_per_sec,$ap_ops_per_sec,$total_ops_per_sec" >> "$OUTPUT_FILE"
    
    echo "  Result: TP=$tp_ops_per_sec ops/s, AP=$ap_ops_per_sec ops/s, Total=$total_ops_per_sec ops/s"
}

# Experiment 1: All optimizations enabled, varying TP:AP ratios
echo "========================================="
echo "Experiment 1: All optimizations (L2 + Pull-Push)"
echo "Varying TP:AP ratios"
echo "========================================="

ratios=("1:0" "3:1" "1:1" "1:3" "0:1")
tp_threads_list=(8 6 4 2 0)
ap_threads_list=(0 2 4 6 8)

for i in "${!ratios[@]}"; do
    ratio="${ratios[$i]}"
    tp_t="${tp_threads_list[$i]}"
    ap_t="${ap_threads_list[$i]}"
    
    echo "  Running TP:AP = $ratio (TP threads=$tp_t, AP threads=$ap_t)..."
    
    if [ $tp_t -eq 0 ]; then
        workload="ap"
        db_path="/tmp/ablation_exp1_ap"
        preload_data "$db_path" "false"  # Use OverlayTree for preload
        run_benchmark "$db_path" "$workload" 0 "$ap_t" "true" "true" "exp1_all_opt" "all_opt" "$ratio"
    elif [ $ap_t -eq 0 ]; then
        workload="tp"
        db_path="/tmp/ablation_exp1_tp"
        preload_data "$db_path" "false"  # Use OverlayTree for preload
        run_benchmark "$db_path" "$workload" "$tp_t" 0 "true" "true" "exp1_all_opt" "all_opt" "$ratio"
    else
        workload="mixed"
        db_path="/tmp/ablation_exp1_${ratio//:/_}"
        preload_data "$db_path" "false"  # Use OverlayTree for preload
        run_benchmark "$db_path" "$workload" "$tp_t" "$ap_t" "true" "true" "exp1_all_opt" "all_opt" "$ratio"
    fi
    
    sleep 1
done

# Experiment 2: L2 columnar storage on/off, varying TP:AP ratios
echo ""
echo "========================================="
echo "Experiment 2: L2 columnar storage on/off"
echo "Testing L2 cache performance impact"
echo "========================================="

# Test 2.1: Pure AP workload (L2 should show biggest advantage)
echo ""
echo "  Test 2.1: Pure AP workload (L2 advantage should be most visible)"
echo "  ----------------------------------------"
workload="ap"
tp_t=0
ap_t=8

echo "    Running pure AP with L2 ON (OverlayTree)..."
db_path="/tmp/ablation_exp2_pure_ap_l2_on"
preload_data "$db_path" "false"  # Use OverlayTree for preload
run_benchmark "$db_path" "$workload" "$tp_t" "$ap_t" "true" "true" "exp2_l2_storage" "l2_on" "0:1"

sleep 1

echo "    Running pure AP with L2 OFF (direct Tree)..."
db_path="/tmp/ablation_exp2_pure_ap_l2_off"
preload_data "$db_path" "true"  # Use direct Tree for preload
run_benchmark "$db_path" "$workload" "$tp_t" "$ap_t" "false" "false" "exp2_l2_storage" "l2_off" "0:1"

sleep 1

# Test 2.2: Pure TP workload (L2 overhead should be visible)
echo ""
echo "  Test 2.2: Pure TP workload (L2 overhead test)"
echo "  ----------------------------------------"
workload="tp"
tp_t=8
ap_t=0

echo "    Running pure TP with L2 ON (OverlayTree)..."
db_path="/tmp/ablation_exp2_pure_tp_l2_on"
preload_data "$db_path" "false"  # Use OverlayTree for preload
run_benchmark "$db_path" "$workload" "$tp_t" "$ap_t" "true" "true" "exp2_l2_storage" "l2_on" "1:0"

sleep 1

echo "    Running pure TP with L2 OFF (direct Tree)..."
db_path="/tmp/ablation_exp2_pure_tp_l2_off"
preload_data "$db_path" "true"  # Use direct Tree for preload
run_benchmark "$db_path" "$workload" "$tp_t" "$ap_t" "false" "false" "exp2_l2_storage" "l2_off" "1:0"

sleep 1

# Test 2.3: Mixed workloads with varying TP:AP ratios
echo ""
echo "  Test 2.3: Mixed workloads (varying TP:AP ratios)"
echo "  ----------------------------------------"
ratios=("3:1" "1:1" "1:3")
tp_threads_list=(6 4 2)
ap_threads_list=(2 4 6)

for i in "${!ratios[@]}"; do
    ratio="${ratios[$i]}"
    tp_t="${tp_threads_list[$i]}"
    ap_t="${ap_threads_list[$i]}"
    
    workload="mixed"
    
    echo "    Running TP:AP = $ratio with L2 ON (OverlayTree)..."
    db_path="/tmp/ablation_exp2_${ratio//:/_}_l2_on"
    preload_data "$db_path" "false"  # Use OverlayTree for preload
    run_benchmark "$db_path" "$workload" "$tp_t" "$ap_t" "true" "true" "exp2_l2_storage" "l2_on" "$ratio"
    
    sleep 1
    
    echo "    Running TP:AP = $ratio with L2 OFF (direct Tree)..."
    db_path="/tmp/ablation_exp2_${ratio//:/_}_l2_off"
    preload_data "$db_path" "true"  # Use direct Tree for preload
    run_benchmark "$db_path" "$workload" "$tp_t" "$ap_t" "false" "false" "exp2_l2_storage" "l2_off" "$ratio"
    
    sleep 1
done

# Experiment 3: Pull-Push strategy on/off (with L2 enabled), varying TP:AP ratios
echo ""
echo "========================================="
echo "Experiment 3: Pull-Push strategy on/off"
echo "Varying TP:AP ratios"
echo "========================================="

for i in "${!ratios[@]}"; do
    ratio="${ratios[$i]}"
    tp_t="${tp_threads_list[$i]}"
    ap_t="${ap_threads_list[$i]}"
    
    if [ $tp_t -eq 0 ] || [ $ap_t -eq 0 ]; then
        continue  # Skip pure TP or pure AP for this experiment
    fi
    
    workload="mixed"
    
    echo "  Running TP:AP = $ratio with Pull ON..."
    db_path="/tmp/ablation_exp3_${ratio//:/_}_pull_on"
    preload_data "$db_path" "false"  # Use OverlayTree for preload
    run_benchmark "$db_path" "$workload" "$tp_t" "$ap_t" "true" "true" "exp3_pull_strategy" "pull_on" "$ratio"
    
    sleep 1
    
    echo "  Running TP:AP = $ratio with Pull OFF..."
    db_path="/tmp/ablation_exp3_${ratio//:/_}_pull_off"
    preload_data "$db_path" "false"  # Use OverlayTree for preload
    run_benchmark "$db_path" "$workload" "$tp_t" "$ap_t" "false" "true" "exp3_pull_strategy" "pull_off" "$ratio"
    
    sleep 1
done

echo ""
echo "========================================="
echo "All experiments completed!"
echo "Results saved to: $OUTPUT_FILE"
echo "========================================="
echo ""
echo "Summary:"
cat "$OUTPUT_FILE" | column -t -s,

