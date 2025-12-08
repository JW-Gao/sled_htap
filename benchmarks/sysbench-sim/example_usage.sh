#!/bin/bash
# Example usage of L2 cache testing and report generation

echo "========================================="
echo "L2 Cache Test and Report Generation Example"
echo "========================================="
echo ""

# Example 1: Run test and generate report
echo "Example 1: Run AP workload test and generate report"
echo "---------------------------------------------------"
echo "# Run test"
echo "./test_l2_cache.sh --workload ap --time 30"
echo ""
echo "# Generate report"
echo "python generate_l2_report.py results/l2_test_*.csv"
echo ""
echo "# View report"
echo "jupyter notebook l2_report_*.ipynb"
echo ""

# Example 2: Test with specific TP:AP ratio
echo "Example 2: Test mixed workload with 3:1 ratio"
echo "---------------------------------------------------"
echo "./test_l2_cache.sh --workload mixed --tp-ap-ratio 3:1 --time 30"
echo "python generate_l2_report.py results/l2_test_*.csv --output mixed_3to1_report.ipynb"
echo ""

# Example 3: Test only L2 ON
echo "Example 3: Test only L2 ON (no comparison)"
echo "---------------------------------------------------"
echo "./test_l2_cache.sh --workload ap --l2-on --time 30"
echo "python generate_l2_report.py results/l2_test_*.csv"
echo ""

# Example 4: Batch testing
echo "Example 4: Run multiple tests and generate combined report"
echo "---------------------------------------------------"
echo "# Run multiple tests"
echo "./test_l2_cache.sh --workload ap --time 30 --output results/test_ap.csv"
echo "./test_l2_cache.sh --workload tp --time 30 --output results/test_tp.csv"
echo "./test_l2_cache.sh --workload mixed --tp-ap-ratio 1:1 --time 30 --output results/test_mixed.csv"
echo ""
echo "# Generate combined report"
echo "python generate_l2_report.py results/test_*.csv --output comprehensive_report.ipynb"
echo ""

echo "========================================="
echo "For more details, see:"
echo "  - REPORT_GENERATOR_README.md"
echo "  - L2_CACHE_TEST_README.md"
echo "  - L2_TEST_QUICKSTART.md"
echo "========================================="




