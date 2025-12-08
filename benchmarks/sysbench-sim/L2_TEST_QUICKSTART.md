# L2 Cache 测试快速指南

## 核心功能

### 1. 指定 TP:AP 负载比例

使用 `--tp-ap-ratio` 参数可以方便地指定 TP 和 AP 的负载比例：

```bash
# 3:1 比例（75% TP, 25% AP）
./test_l2_cache.sh --workload mixed --tp-ap-ratio 3:1

# 1:1 比例（50% TP, 50% AP）
./test_l2_cache.sh --workload mixed --tp-ap-ratio 1:1

# 1:3 比例（25% TP, 75% AP）
./test_l2_cache.sh --workload mixed --tp-ap-ratio 1:3
```

**工作原理**：
- 脚本会根据总线程数（`--threads`，默认8）和比例自动计算 TP 和 AP 线程数
- 例如：`--threads 8 --tp-ap-ratio 3:1` → TP=6 线程, AP=2 线程

**或者手动指定线程数**：
```bash
./test_l2_cache.sh --workload mixed --tp-threads 6 --ap-threads 2
```

### 2. 控制 L2 Cache 开启/关闭

使用 `--l2-on`、`--l2-off` 或 `--l2-both` 参数控制测试哪些配置：

```bash
# 只测试 L2 ON（不测试 L2 OFF）
./test_l2_cache.sh --workload ap --l2-on

# 只测试 L2 OFF（不测试 L2 ON）
./test_l2_cache.sh --workload ap --l2-off

# 测试两者（默认行为）
./test_l2_cache.sh --workload ap --l2-both
# 或
./test_l2_cache.sh --workload ap  # 默认就是 both
```

## 常用测试场景

### 场景 1：测试 L2 在 AP 负载下的优势

```bash
./test_l2_cache.sh --workload ap --time 30
```

**预期**：L2 ON 应该比 L2 OFF 快 2-5 倍

### 场景 2：测试 L2 在 TP 负载下的开销

```bash
./test_l2_cache.sh --workload tp --time 30
```

**预期**：L2 ON 可能比 L2 OFF 慢 10-20%

### 场景 3：测试混合负载（使用比例）

```bash
# 测试 3:1 TP:AP 比例
./test_l2_cache.sh --workload mixed --tp-ap-ratio 3:1 --time 30

# 测试 1:1 TP:AP 比例
./test_l2_cache.sh --workload mixed --tp-ap-ratio 1:1 --time 30

# 测试 1:3 TP:AP 比例
./test_l2_cache.sh --workload mixed --tp-ap-ratio 1:3 --time 30
```

**预期**：L2 ON 在混合场景下应该整体更好（AP 优势 > TP 开销）

### 场景 4：只测试 L2 ON 的绝对性能

```bash
./test_l2_cache.sh --workload ap --l2-on --time 30
```

**适用**：只需要了解 L2 ON 的绝对性能，不需要对比

### 场景 5：测试不同查询范围大小

```bash
# 小范围查询（1% 的表大小）
./test_l2_cache.sh --workload ap --ap-range-frac 0.01 --time 30

# 中等范围查询（10% 的表大小，默认）
./test_l2_cache.sh --workload ap --ap-range-frac 0.1 --time 30

# 大范围查询（50% 的表大小）
./test_l2_cache.sh --workload ap --ap-range-frac 0.5 --time 30
```

**预期**：范围越大，L2 优势越明显

## 完整示例

```bash
# 测试混合负载，3:1 TP:AP 比例，只测试 L2 ON，大表，长时间运行
./test_l2_cache.sh \
    --workload mixed \
    --tp-ap-ratio 3:1 \
    --l2-on \
    --table-size 1000000 \
    --time 60 \
    --preload-time 60 \
    --threads 8 \
    --ap-range-frac 0.1 \
    --output results/l2_test_3to1.csv
```

## 参数优先级

1. **TP:AP 比例**：如果指定 `--tp-ap-ratio`，会自动覆盖 `--tp-threads` 和 `--ap-threads`
2. **L2 控制**：`--l2-on`、`--l2-off`、`--l2-both` 互斥，最后指定的生效
3. **Workload 类型**：如果指定 `--tp-ap-ratio`，会自动将 workload 设置为 `mixed`

## 输出说明

脚本会生成 CSV 文件，包含：
- `test_name`: 测试名称（l2_on 或 l2_off）
- `workload`: 工作负载类型
- `l2_enabled`: L2 是否启用
- `threads`: 线程信息
- `ops`: 总操作数
- `ops_per_sec`: 每秒操作数
- `improvement_pct`: 性能改进百分比（仅当测试两者时）

如果只测试一个配置（`--l2-on` 或 `--l2-off`），则不会计算改进百分比。




