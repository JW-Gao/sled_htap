# L2 Cache 性能测试说明

## 原理说明

### L2 Cache 架构

我们的系统采用两层存储架构：

```
┌─────────────────────────────────────┐
│  L1 (内存) - 行式存储 (Row-oriented) │
│  ┌─────┬─────┬─────┐                │
│  │ Key │ Row │ Row │                │
│  │  k1 │ v1  │ v2  │                │
│  └─────┴─────┴─────┘                │
└─────────────────────────────────────┘
              │ Flush (PUSH)
              ▼
┌─────────────────────────────────────┐
│  L2 (磁盘) - 列式存储 (Columnar)     │
│  ┌─────────┬─────────┐              │
│  │ Col 0   │ Col 1   │              │
│  │ c|0|k1  │ c|1|k1  │              │
│  │ c|0|k2  │ c|1|k2  │              │
│  └─────────┴─────────┘              │
└─────────────────────────────────────┘
```

### 数据格式转换

**L1 行式存储**：
- 格式：`grade|payload`
- 示例：`12345678|ABCD` (8字节grade + 4字节payload)
- 存储方式：每个key对应一个完整的行值

**L2 列式存储**：
- 格式：`c|<col_idx>|<row_key>` → `column_value`
- 示例：
  - `c|0|k1` → `12345678` (grade列)
  - `c|1|k1` → `ABCD` (payload列)
- 存储方式：每个列单独存储，相同列的数据连续存储

### L2 Cache 的优势

#### 1. **AP (分析查询) 性能提升**

**场景：计算 grade 列的平均值**

**L2 OFF (行式存储)**：
```
需要读取：k1的完整行 → 解析 → 提取grade
          k2的完整行 → 解析 → 提取grade
          k3的完整行 → 解析 → 提取grade
          ...
I/O量：N行 × (grade + payload + 分隔符) = N × 13字节
```

**L2 ON (列式存储)**：
```
只需要读取：c|0|k1, c|0|k2, c|0|k3, ...
I/O量：N行 × grade = N × 8字节
```

**优势**：减少约 38% 的 I/O（13字节 → 8字节），且列数据连续存储，缓存友好。

#### 2. **范围查询优化**

**场景：查询 key 范围 [k_start, k_end) 的 grade 列统计**

**L2 OFF**：
```rust
// 需要扫描所有行，解析每行
for (key, row) in tree.range(k_start..k_end) {
    let grade = parse_row(row);  // 需要解析整行
    sum += grade;
}
```

**L2 ON**：
```rust
// 直接扫描 grade 列
let start_col_key = make_col_key(k_start, 0);  // c|0|k_start
let end_col_key = make_col_key(k_end, 0);      // c|0|k_end
for (_, grade) in tree.range(start_col_key..end_col_key) {
    sum += grade;  // 直接使用，无需解析
}
```

**优势**：
- 只读取需要的列，减少 I/O
- 列数据连续存储，顺序读取效率高
- 无需解析行格式，减少 CPU 开销

#### 3. **缓存局部性**

列式存储中，相同列的数据物理上连续存储：
```
c|0|k1, c|0|k2, c|0|k3, ...  (grade列连续)
c|1|k1, c|1|k2, c|1|k3, ...  (payload列连续)
```

这提高了：
- **缓存命中率**：连续读取相同类型数据
- **预取效率**：CPU 预取器可以更有效地工作
- **压缩效率**：相同类型数据更容易压缩

### L2 Cache 的劣势

#### 1. **TP (事务查询) 开销**

**场景：读取单行完整数据**

**L2 OFF**：
```rust
let row = tree.get(key)?;  // 一次读取，直接返回
```

**L2 ON**：
```rust
// 需要读取多个列，然后重组
let col0 = tree.get(make_col_key(key, 0))?;  // 读取列0
let col1 = tree.get(make_col_key(key, 1))?;  // 读取列1
let row = join_cols(col0, col1);  // 重组行
```

**开销**：
- 多次键查找（每列一次）
- 需要重组数据
- 对于点查询，可能比行式存储慢

#### 2. **写入开销**

写入时需要：
1. 写入 L1（行式，快速）
2. Flush 时拆分行 → 多列写入 L2（额外开销）

### 测试策略

#### 测试场景 1：纯 AP Workload
```bash
./test_l2_cache.sh --workload ap --time 30
```
**预期结果**：L2 ON 应该显著优于 L2 OFF（2-5倍提升）

#### 测试场景 2：纯 TP Workload
```bash
./test_l2_cache.sh --workload tp --time 30
```
**预期结果**：L2 ON 可能略慢于 L2 OFF（10-20%开销）

#### 测试场景 3：混合 Workload（使用比例）
```bash
# 使用 TP:AP 比例（推荐）
./test_l2_cache.sh --workload mixed --tp-ap-ratio 3:1 --time 30

# 或手动指定线程数
./test_l2_cache.sh --workload mixed --tp-threads 6 --ap-threads 2 --time 30
```
**预期结果**：L2 ON 在混合场景下应该整体更好（AP优势 > TP开销）

#### 测试场景 4：只测试 L2 ON
```bash
./test_l2_cache.sh --workload ap --l2-on --time 30
```
**适用场景**：只需要了解 L2 ON 的绝对性能，不需要对比

#### 测试场景 5：不同范围大小
```bash
# 小范围查询
./test_l2_cache.sh --workload ap --ap-range-frac 0.01 --time 30

# 大范围查询
./test_l2_cache.sh --workload ap --ap-range-frac 0.5 --time 30
```
**预期结果**：范围越大，L2 优势越明显

## 使用方法

### 基本用法

```bash
# 测试 AP workload（默认测试 L2 ON 和 OFF）
./test_l2_cache.sh --workload ap

# 测试 TP workload
./test_l2_cache.sh --workload tp

# 测试混合 workload，使用 TP:AP 比例
./test_l2_cache.sh --workload mixed --tp-ap-ratio 3:1

# 测试混合 workload，手动指定线程数
./test_l2_cache.sh --workload mixed --tp-threads 6 --ap-threads 2
```

### L2 Cache 控制

```bash
# 只测试 L2 ON（不测试 L2 OFF）
./test_l2_cache.sh --workload ap --l2-on

# 只测试 L2 OFF（不测试 L2 ON）
./test_l2_cache.sh --workload ap --l2-off

# 测试两者（默认行为，可显式指定）
./test_l2_cache.sh --workload ap --l2-both
```

### 完整参数示例

```bash
# 测试混合负载，3:1 TP:AP 比例，只测试 L2 ON
./test_l2_cache.sh \
    --workload mixed \
    --tp-ap-ratio 3:1 \
    --l2-on \
    --table-size 1000000 \
    --time 60 \
    --preload-time 60 \
    --threads 8 \
    --ap-range-frac 0.1 \
    --output results/my_test.csv
```

### 参数说明

#### 工作负载参数

- `--workload`: 工作负载类型
  - `tp`: 纯事务处理（点查询 + 写入）
  - `ap`: 纯分析处理（范围查询 + 列聚合）
  - `mixed`: 混合负载

- `--tp-ap-ratio RATIO`: **TP:AP 比例**（例如 `3:1`, `1:1`, `1:3`）
  - 自动根据总线程数计算 TP 和 AP 线程数
  - 如果指定此参数，会自动将 workload 设置为 `mixed`
  - 示例：`--threads 8 --tp-ap-ratio 3:1` → TP=6 线程, AP=2 线程

- `--tp-threads N`: TP 线程数（用于 mixed workload，与 `--tp-ap-ratio` 互斥）
- `--ap-threads N`: AP 线程数（用于 mixed workload，与 `--tp-ap-ratio` 互斥）

#### L2 Cache 控制参数

- `--l2-on`: **只测试 L2 cache 开启**（不测试 L2 OFF）
- `--l2-off`: **只测试 L2 cache 关闭**（不测试 L2 ON）
- `--l2-both`: **测试两者**（默认行为）

#### 其他参数

- `--table-size`: 数据表大小（key空间）
- `--time`: 测试运行时间（秒）
- `--preload-time`: 数据预加载时间（秒）
- `--threads`: 总线程数（用于 tp 或 ap workload，或与 `--tp-ap-ratio` 配合使用）
- `--ap-range-frac`: AP 查询范围大小（相对于 table-size 的比例）
- `--write-pct`: TP workload 的写入百分比（0-100）
- `--output`: 输出 CSV 文件路径

## 结果解读

### CSV 输出格式

```
test_name,workload,l2_enabled,threads,tp_threads,ap_threads,ops,ops_per_sec,improvement_pct
l2_on,ap,yes,8,0,0,50000,5000,
l2_off,ap,no,8,0,0,20000,2000,150.00
```

- `improvement_pct`: L2 ON 相对于 L2 OFF 的性能提升百分比
- 正值表示 L2 ON 更快，负值表示 L2 ON 更慢

### 性能指标

1. **吞吐量 (ops/s)**: 每秒操作数，越高越好
2. **改进百分比**: L2 ON 相对于 L2 OFF 的提升
3. **不同 workload 的对比**: 观察 L2 在不同场景下的表现

## 预期结果

### AP Workload
- **L2 ON**: 高吞吐量（列式存储优势）
- **L2 OFF**: 低吞吐量（需要读取整行）
- **提升**: 通常 2-5 倍

### TP Workload
- **L2 ON**: 中等吞吐量（列重组开销）
- **L2 OFF**: 高吞吐量（直接行读取）
- **开销**: 通常 10-20%

### Mixed Workload
- **L2 ON**: 整体更好（AP 优势 > TP 开销）
- **L2 OFF**: 整体较差（AP 性能差）
- **平衡点**: 取决于 TP:AP 比例

## 故障排除

### 编译错误
```bash
cd benchmarks/sysbench-sim
cargo build --release
```

### 结果全为 0
- 检查数据是否成功预加载
- 增加 `--preload-time` 参数
- 检查数据库路径权限

### 性能差异不明显
- 增加 `--table-size` 和 `--time` 参数
- 确保使用 release 模式编译
- 检查系统资源（CPU、内存、磁盘 I/O）

