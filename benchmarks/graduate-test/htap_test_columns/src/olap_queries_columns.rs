use sled::Db;
use crate::schema::TableSchema;

/// OLAP查询类型（支持列选择）
#[derive(Debug, Clone, Copy)]
pub enum QueryType {
    Q1,  // COUNT查询
    Q2,  // 投影查询
    Q3,  // 聚合查询
}

/// 执行Q1查询：COUNT(*)
pub fn execute_q1(db: &Db, schema: &TableSchema, data_access_ratio: f64, total_rows: usize) -> usize {
    let threshold = (total_rows as f64 * data_access_ratio) as u64;
    let mut count = 0;
    
    for entry in db.iter() {
        if let Ok((key, _value)) = entry {
            let pk = schema.get_pk(&key);
            if pk < threshold {
                count += 1;
            }
        }
    }
    
    count
}

/// 执行Q2查询：SELECT c1, c2, ..., cm
pub fn execute_q2(
    db: &Db,
    schema: &TableSchema,
    data_access_ratio: f64,
    total_rows: usize,
    num_columns: usize,
) -> usize {
    let threshold = (total_rows as f64 * data_access_ratio) as u64;
    let mut result_count = 0;
    
    for entry in db.iter() {
        if let Ok((key, value)) = entry {
            let pk = schema.get_pk(&key);
            if pk < threshold {
                // 读取指定数量的列
                for col_idx in 0..num_columns.min(schema.num_columns) {
                    let _col_value = schema.get_column(&value, col_idx);
                }
                result_count += 1;
            }
        }
    }
    
    result_count
}

/// 执行Q3查询：SELECT MAX(c1), MAX(c2), ..., MAX(cm)
pub fn execute_q3(
    db: &Db,
    schema: &TableSchema,
    data_access_ratio: f64,
    total_rows: usize,
    num_columns: usize,
) -> Vec<f32> {
    let threshold = (total_rows as f64 * data_access_ratio) as u64;
    let mut max_values = vec![f32::MIN; num_columns.min(schema.num_columns)];
    
    for entry in db.iter() {
        if let Ok((key, value)) = entry {
            let pk = schema.get_pk(&key);
            if pk < threshold {
                // 对指定数量的列计算MAX
                for col_idx in 0..num_columns.min(schema.num_columns) {
                    let col_value = schema.get_column(&value, col_idx);
                    if col_value > max_values[col_idx] {
                        max_values[col_idx] = col_value;
                    }
                }
            }
        }
    }
    
    max_values
}

/// 执行查询（根据类型和列数）
pub fn execute_query(
    db: &Db,
    schema: &TableSchema,
    query_type: QueryType,
    data_access_ratio: f64,
    total_rows: usize,
    num_columns: usize,
) -> usize {
    match query_type {
        QueryType::Q1 => {
            execute_q1(db, schema, data_access_ratio, total_rows)
        }
        QueryType::Q2 => {
            execute_q2(db, schema, data_access_ratio, total_rows, num_columns)
        }
        QueryType::Q3 => {
            let max_vals = execute_q3(db, schema, data_access_ratio, total_rows, num_columns);
            max_vals.len()  // 返回处理的列数作为结果
        }
    }
}
