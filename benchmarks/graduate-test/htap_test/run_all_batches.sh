#!/bin/bash
# HTAP测试 - 运行所有6个批次的自动化脚本

echo "================================"
echo "HTAP批次测试 - 自动运行所有批次"
echo "================================"
echo ""

timestamp=$(date +%Y%m%d_%H%M%S)
log_file="htap_all_batches_${timestamp}.log"

echo "开始时间: $(date)" | tee -a "$log_file"
echo "日志文件: $log_file" | tee -a "$log_file"
echo "" | tee -a "$log_file"

# 运行所有6个批次
for batch in {1..6}; do
    echo "======================================" | tee -a "$log_file"
    echo "开始批次 $batch/6" | tee -a "$log_file"
    echo "时间: $(date)" | tee -a "$log_file"
    echo "======================================" | tee -a "$log_file"
    echo "" | tee -a "$log_file"
    
    python3 run_htap_test_batch.py $batch 2>&1 | tee -a "$log_file"
    
    if [ $? -eq 0 ]; then
        echo "" | tee -a "$log_file"
        echo "✓ 批次 $batch 完成" | tee -a "$log_file"
    else
        echo "" | tee -a "$log_file"
        echo "✗ 批次 $batch 失败" | tee -a "$log_file"
    fi
    
    echo "" | tee -a "$log_file"
done

echo "======================================" | tee -a "$log_file"
echo "所有批次测试完成" | tee -a "$log_file"
echo "结束时间: $(date)" | tee -a "$log_file"
echo "======================================" | tee -a "$log_file"
echo "" | tee -a "$log_file"

# 合并结果
echo "合并测试结果..." | tee -a "$log_file"
python3 merge_results.py 2>&1 | tee -a "$log_file"

echo "" | tee -a "$log_file"
echo "全部完成！查看 $log_file 了解详情" | tee -a "$log_file"
