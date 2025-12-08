#!/bin/bash

# Simple script to test mixed workload (TP + AP) throughput
# Usage: ./test_mixed_workload.sh [options]
#
# Options:
#   --tp-threads N        Number of TP threads (default: 4)
#   --ap-threads N        Number of AP threads (default: 2)
#   --time N              Test duration in seconds (default: 10)
#   --table-size N        Table size (default: 100000)
#   --write-pct N         Write percentage 0-100 (default: 20)
#   --ap-range-frac F     AP range query fraction (default: 0.1)
#   --ap-compute-iters N  AP compute iterations (default: 256)
#   --use-direct-tree     Use direct Tree instead of OverlayTree (L2 off)
#   --no-pull             Disable Pull strategy
#   --hotspot-frac F      Hotspot fraction (cheat interface, optional)
#   --path PATH           Database path (default: /tmp/mixed_workload_test)
#   --no-preload          Skip data preloading

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Default parameters
# All optimizations are enabled by default (except hotspot-frac cheat interface):
# - L2 cache (OverlayTree): enabled (USE_DIRECT_TREE=false)
# - Pull-Push strategy: enabled (PULL=true)
# - Hotspot-frac: disabled (let system discover hotspots naturally)
TP_THREADS=4
AP_THREADS=2
TIME=10
TABLE_SIZE=100000
WRITE_PCT=20
AP_RANGE_FRAC=0.1
AP_COMPUTE_ITERS=256
USE_DIRECT_TREE=false  # false = use OverlayTree (L2 enabled), true = use direct Tree (L2 disabled)
PULL=true              # true = Pull strategy enabled, false = disabled
HOTSPOT_FRAC=""        # Empty = normal mode (uniform random), set value = cheat interface
DB_PATH=/tmp/mixed_workload_test
PRELOAD=true

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --tp-threads)
            TP_THREADS="$2"
            shift 2
            ;;
        --ap-threads)
            AP_THREADS="$2"
            shift 2
            ;;
        --time)
            TIME="$2"
            shift 2
            ;;
        --table-size)
            TABLE_SIZE="$2"
            shift 2
            ;;
        --write-pct)
            WRITE_PCT="$2"
            shift 2
            ;;
        --ap-range-frac)
            AP_RANGE_FRAC="$2"
            shift 2
            ;;
        --ap-compute-iters)
            AP_COMPUTE_ITERS="$2"
            shift 2
            ;;
        --use-direct-tree)
            USE_DIRECT_TREE=true
            shift
            ;;
        --no-pull)
            PULL=false
            shift
            ;;
        --hotspot-frac)
            HOTSPOT_FRAC="$2"
            shift 2
            ;;
        --path)
            DB_PATH="$2"
            shift 2
            ;;
        --no-preload)
            PRELOAD=false
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--tp-threads N] [--ap-threads N] [--time N] [--table-size N] [--write-pct N] [--ap-range-frac F] [--ap-compute-iters N] [--use-direct-tree] [--no-pull] [--hotspot-frac F] [--path PATH] [--no-preload]"
            exit 1
            ;;
    esac
done

echo "========================================="
echo "Mixed Workload Throughput Test"
echo "========================================="
echo "TP threads: $TP_THREADS"
echo "AP threads: $AP_THREADS"
echo "Test duration: ${TIME}s"
echo "Table size: $TABLE_SIZE"
echo "Write percentage: $WRITE_PCT%"
echo "AP range fraction: $AP_RANGE_FRAC"
echo "AP compute iterations: $AP_COMPUTE_ITERS"
echo "L2 enabled: $([ "$USE_DIRECT_TREE" = "true" ] && echo "no" || echo "yes")"
echo "Pull enabled: $([ "$PULL" = "true" ] && echo "yes" || echo "no")"
[ -n "$HOTSPOT_FRAC" ] && echo "Hotspot fraction: $HOTSPOT_FRAC (cheat interface)"
echo "Database path: $DB_PATH"
echo ""

# Preload data if requested
if [ "$PRELOAD" = "true" ]; then
    echo "Preloading data..."
    /bin/rm -rf "$DB_PATH"
    preload_args=(
        --path "$DB_PATH"
        --workload tp
        --time 10
        --threads 4
        --write-pct 100
        --table-size $TABLE_SIZE
    )
    [ -n "$HOTSPOT_FRAC" ] && preload_args+=(--hotspot-frac "$HOTSPOT_FRAC")
    cargo run --release --manifest-path "$SCRIPT_DIR/Cargo.toml" -- "${preload_args[@]}" > /dev/null 2>&1
    echo "Preload completed."
    echo ""
fi

echo "Running mixed workload test..."
echo ""

# Build benchmark command
cmd_args=(
    --path "$DB_PATH"
    --workload mixed
    --time $TIME
    --tp-threads $TP_THREADS
    --ap-threads $AP_THREADS
    --table-size $TABLE_SIZE
    --write-pct $WRITE_PCT
    --ap-range-frac $AP_RANGE_FRAC
    --ap-compute-iters $AP_COMPUTE_ITERS
)

if [ "$PULL" = "true" ]; then
    cmd_args+=(--pull)
else
    cmd_args+=(--no-pull)
fi

[ "$USE_DIRECT_TREE" = "true" ] && cmd_args+=(--use-direct-tree)
[ -n "$HOTSPOT_FRAC" ] && cmd_args+=(--hotspot-frac "$HOTSPOT_FRAC")

# Run benchmark
cargo run --release --manifest-path "$SCRIPT_DIR/Cargo.toml" -- "${cmd_args[@]}"

echo ""
echo "Test completed!"

