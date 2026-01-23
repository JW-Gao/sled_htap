#!/bin/bash
set -e

# Build release
cargo build --release --bin mixed_workload

OUTPUT_FILE="benchmark_results.json"
echo "[" > $OUTPUT_FILE

RUN_COUNT=0
TOTAL_RUNS=$((2 * 3 * 2 * 4)) # 2 Col * 3 Ratio * 2 Mode * 4 Sel = 48 runs

run_bench() {
    COLS=$1
    RATIO=$2
    MODE=$3
    SEL=$4
    
    echo "Running: Cols=$COLS Ratio=$RATIO Mode=$MODE Sel=$SEL ($RUN_COUNT/$TOTAL_RUNS)"
    
    # Run and capture output
    # Filter only the JSON lines
    RESULT=$(./target/release/mixed_workload \
        --columns $COLS \
        --ratio $RATIO \
        --mode $MODE \
        --selectivity $SEL \
        --total-ops 50000 \
        --warmup-rows 50000 \
        | grep -A 5 "^{")
    
    if [ "$RUN_COUNT" -gt 0 ]; then
        echo "," >> $OUTPUT_FILE
    fi
    echo "$RESULT" >> $OUTPUT_FILE
    
    RUN_COUNT=$((RUN_COUNT + 1))
}

for COLS in 30 70; do
    for RATIO in read balance write; do
        for SEL in 0.1 0.4 0.7 1.0; do
            # Compare Row vs Column
            run_bench $COLS $RATIO row $SEL
            run_bench $COLS $RATIO column $SEL
        done
    done
done

echo "]" >> $OUTPUT_FILE
echo "Benchmark Complete. Results saved to $OUTPUT_FILE"
