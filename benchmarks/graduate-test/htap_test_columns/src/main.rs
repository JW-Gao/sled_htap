mod schema;
mod olap_queries_columns;
mod workload;

use clap::Parser;
use schema::TableSchema;
use olap_queries_columns::{QueryType, execute_query};
use workload::WorkloadGenerator;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "htap_test_columns")]
#[command(about = "HTAP混合负载测试 - 列选择性能测试")]
struct Args {
    #[arg(long, default_value_t = 30)]
    num_columns: usize,

    #[arg(long, default_value_t = 1)]
    select_columns: usize,

    #[arg(long, default_value_t = 0.7)]
    olap_ratio: f64,

    #[arg(long, default_value_t = 0.3)]
    oltp_ratio: f64,

    #[arg(long, default_value_t = 0.2)]
    data_access_ratio: f64,

    #[arg(long, default_value_t = 50000)]
    total_ops: usize,

    #[arg(long, default_value_t = 100000)]
    prepopulate_rows: usize,

    #[arg(long, default_value = "baseline")]
    mode: String,
}

fn main() {
    let args = Args::parse();

    println!("================================================================================");
    println!("HTAP混合负载测试 - 列选择性能");
    println!("================================================================================");
    println!("配置:");
    println!("  表列数: {}", args.num_columns);
    println!("  读取列数: {}", args.select_columns);
    println!("  OLAP比例: {:.0}%", args.olap_ratio * 100.0);
    println!("  OLTP比例: {:.0}%", args.oltp_ratio * 100.0);
    println!("  数据访问比例: {:.0}%", args.data_access_ratio * 100.0);
    println!("  总操作数: {}", args.total_ops);
    println!("  预填充行数: {}", args.prepopulate_rows);
    println!("  模式: {}", args.mode);
    println!("================================================================================\n");

    // 创建数据库
    let db_path = format!("htap_column_test_db_{}", std::process::id());
    let db = sled::open(&db_path).unwrap();

    // 创建表模式
    let schema = TableSchema::new(args.num_columns);

    // 预填充数据
    println!("预填充数据库...");
    for i in 0..args.prepopulate_rows {
        let row = schema.generate_row(i as u64);
        let pk = schema.get_pk(&row);
        db.insert(&pk.to_be_bytes(), row).unwrap();
    }
    db.flush().unwrap();
    println!("✓ 预填充完成: {} 行\n", args.prepopulate_rows);

    // 生成混合负载
    let workload = WorkloadGenerator::new(
        args.olap_ratio,
        args.oltp_ratio,
        args.data_access_ratio,
        args.total_ops,
        args.prepopulate_rows,
    );

    let operations = workload.generate();

    // 执行混合负载
    println!("执行混合负载...");
    let start = Instant::now();

    let mut next_pk = args.prepopulate_rows as u64;
    
    for (idx, op) in operations.iter().enumerate() {
        match op {
            workload::Operation::OlapQuery(query_type) => {
                let _result = execute_query(
                    &db,
                    &schema,
                    *query_type,
                    args.data_access_ratio,
                    args.prepopulate_rows,
                    args.select_columns,  // 使用指定的列数
                );
            }
            workload::Operation::OltpWrite => {
                let row = schema.generate_row(next_pk);
                let pk = schema.get_pk(&row);
                db.insert(&pk.to_be_bytes(), row).unwrap();
                next_pk += 1;
            }
        }

        // 进度提示
        if (idx + 1) % 10000 == 0 {
            println!("  进度: {}/{}", idx + 1, args.total_ops);
        }
    }

    let duration = start.elapsed();

    println!("\n================================================================================");
    println!("测试完成!");
    println!("================================================================================");
    println!("Execution Time: {:.3} seconds", duration.as_secs_f64());
    println!("================================================================================\n");

    // 清理
    drop(db);
    let _ = std::fs::remove_dir_all(&db_path);
}
