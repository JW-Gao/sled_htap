import csv
import random
import os

def run_experiment():
    # 实验说明：模拟验证单数据流与双数据流架构在不同交叉分散度 (delta) 下的读放大
    
    # 理论模型所需系统参数
    B = 8192 # 物理块大小 (Bytes)
    s_r = 256 # 行查询单次目标数据量 (Bytes)
    s_c = 1000 # 列查询单次目标数据量 (Bytes)
    V_c_V_r_ratio = 10.0 # 列格式基础数据量与行格式数据量之比 (V_c / V_r)
    V_r_V_c_ratio = 1.0 / 10.0 # (V_r / V_c)
    alpha = 0.1 # 列访问查询的列选择率
    P_row = 0.05 # 非目标行数据被后续行查询命中的概率

    # 3种典型工作负载 (f_r: 行查询频率, f_c: 列查询频率)
    workloads = {
        'TP-Heavy (80% Row, 20% Col)': (0.8, 0.2),
        'Balanced HTAP (50% Row, 50% Col)': (0.5, 0.5),
        'AP-Heavy (20% Row, 80% Col)': (0.2, 0.8)
    }

    deltas = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0]
    results = []

    # 设置随机种子以保证多次运行结果稳定且可复现
    random.seed(42)

    print("Running multi-workload read amplification mock experiment...")

    for wl_name, (f_r, f_c) in workloads.items():
        for delta in deltas:
            # ---- 1. 单数据流理论值计算 ----
            rho_r = 1.0 / (1.0 + delta * V_c_V_r_ratio)
            ra_row_single = B / (s_r + P_row * (B * rho_r - s_r))
            ra_col_single = (1.0 / alpha) * (1.0 + delta * V_r_V_c_ratio)
            expected_ra_single = f_r * ra_row_single + f_c * ra_col_single
            
            # ---- 2. 双数据流理论值计算 ----
            ra_row_dual = B / (s_r + P_row * (B - s_r))
            ra_col_dual = 1.0 / alpha
            expected_ra_dual = f_r * ra_row_dual + f_c * ra_col_dual
            
            # 增加轻微系统波动 (±2%) 模拟真实系统缓存等影响
            noise_single = random.uniform(0.98, 1.02)
            noise_dual = random.uniform(0.99, 1.01)
            
            final_single_ra = expected_ra_single * noise_single
            final_dual_ra = expected_ra_dual * noise_dual
            
            # 计算读放大恶化倍数 (单数据流由于分散导致的读放大增加比例)
            ra_ratio = final_single_ra / final_dual_ra

            results.append({
                'workload': wl_name,
                'delta': delta,
                'single_stream_ra': round(final_single_ra, 2),
                'dual_stream_ra': round(final_dual_ra, 2),
                'ra_ratio': round(ra_ratio, 3)
            })

    # 输出结果文件
    script_dir = os.path.dirname(os.path.abspath(__file__))
    output_file = os.path.join(script_dir, 'ra_results.csv')

    with open(output_file, 'w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=[
            'workload', 'delta', 'single_stream_ra', 'dual_stream_ra', 'ra_ratio'
        ])
        writer.writeheader()
        writer.writerows(results)

    print(f"Mock experiment completed successfully.")
    print(f"Results saved to: {output_file}")

if __name__ == '__main__':
    run_experiment()
