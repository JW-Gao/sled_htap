#!/bin/bash
set -e

# Build release
echo "Building benchmarks relative to workspace..."
echo "Building benchmarks relative to workspace..."
# Build stable crates
# cargo build --release --bin fjall_test --bin blp_tree_test
# Build nightly crates
# cargo +nightly build --release --bin bf_tree_test

RESULTS_FILE="benchmark_results.txt"
echo "Benchmark Results" > $RESULTS_FILE
echo "=================" >> $RESULTS_FILE

RECORDS=10000000
OPS=1000000
run_bench() {
    ENGINE=$1
    WORKLOAD=$2
    CRATE="${ENGINE}_test"
    DB_PATH="/home/rat/d/${ENGINE}_bench_$WORKLOAD"
    
    echo "Running $ENGINE Workload $WORKLOAD..."
    sleep 10
}
run_bench_1() {
    ENGINE=$1
    WORKLOAD=$2
    CRATE="${ENGINE}_test"
    DB_PATH="/home/rat/d/${ENGINE}_bench_$WORKLOAD"
    
    echo "Running $ENGINE Workload $WORKLOAD..."
    echo -n "$ENGINE Workload $WORKLOAD: " >> $RESULTS_FILE
    
    # Clean up previous run
    rm -rf $DB_PATH
    
    # Run and capture output
    # Note: Increasing loading batch size might be needed for 10M records, 
    # but ycsb_common handles it per record or small batches.
    OUTPUT=$(./target/release/$CRATE --workload $WORKLOAD --records $RECORDS --ops $OPS --path "$DB_PATH" --value-size 1024)
    
    # Extract Throughput
    THROUGHPUT=$(echo "$OUTPUT" | grep "Throughput" | awk '{print $2}')
    P99=$(echo "$OUTPUT" | grep "p99" | awk -F'p99=' '{print $2}' | awk -F',' '{print $1}')
    
    if [ -z "$THROUGHPUT" ]; then
        echo "FAILED" >> $RESULTS_FILE
        echo "Run failed:"
        echo "$OUTPUT"
    else
        echo "$THROUGHPUT ops/sec, p99=${P99}us" >> $RESULTS_FILE
        echo "  Result: $THROUGHPUT ops/sec"
    fi
}

echo "Starting Benchmarks (Records: $RECORDS, Ops: $OPS)"

for wl in a b e; do
    run_bench "fjall" $wl
    # run_bench "blp_tree" $wl # Skipped per user request
    run_bench "bf_tree" $wl
done

cat $RESULTS_FILE
