use crate::schema::TableSchema;
use sled::Db;

#[derive(Debug, Clone, Copy)]
pub enum QueryType {
    Q1, // SELECT COUNT(*) FROM T WHERE pk < theta
    Q2, // SELECT c1, c3, c5, c7 FROM T WHERE filter_col < theta (multi-column projection)
    Q3, // SELECT MAX(c2), AVG(c4) FROM T WHERE filter_col < theta (aggregation)
}

impl QueryType {
    pub fn random() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..3) {
            0 => QueryType::Q1,
            1 => QueryType::Q2,
            _ => QueryType::Q3,
        }
    }
}

/// Q1: COUNT query - count rows where pk < theta
pub fn execute_q1(db: &Db, schema: &TableSchema, theta: u64) -> usize {
    let mut count = 0;
    for item in db.iter() {
        if let Ok((key, value)) = item {
            let pk = u64::from_be_bytes(key.as_ref().try_into().unwrap_or([0u8; 8]));
            if pk < theta {
                // Verify it's a valid row
                if value.len() >= schema.row_size() {
                    count += 1;
                }
            }
        }
    }
    count
}

/// Q2: Multi-column projection query - read c1, c3, c5, c7 where filter_col < theta
pub fn execute_q2(db: &Db, schema: &TableSchema, theta: u64, filter_col: usize) -> Vec<Vec<f32>> {
    let mut results = Vec::new();
    let projection_cols = [1, 3, 5, 7]; // indices of columns to project
    
    for item in db.iter() {
        if let Ok((_key, value)) = item {
            if let Some(filter_val) = schema.get_column(&value, filter_col) {
                if (filter_val as u64) < theta {
                    let mut row_result = Vec::new();
                    for &col_idx in &projection_cols {
                        if let Some(val) = schema.get_column(&value, col_idx) {
                            row_result.push(val);
                        }
                    }
                    if !row_result.is_empty() {
                        results.push(row_result);
                    }
                }
            }
        }
    }
    results
}

/// Q3: Aggregation query - compute MAX(c2) and AVG(c4) where filter_col < theta
pub fn execute_q3(db: &Db, schema: &TableSchema, theta: u64, filter_col: usize) -> (Option<f32>, Option<f32>) {
    let mut max_c2: Option<f32> = None;
    let mut sum_c4: f32 = 0.0;
    let mut count_c4: usize = 0;
    
    for item in db.iter() {
        if let Ok((_key, value)) = item {
            if let Some(filter_val) = schema.get_column(&value, filter_col) {
                if (filter_val as u64) < theta {
                    // Update MAX(c2)
                    if let Some(c2_val) = schema.get_column(&value, 2) {
                        max_c2 = Some(match max_c2 {
                            None => c2_val,
                            Some(current_max) => current_max.max(c2_val),
                        });
                    }
                    
                    // Update SUM and COUNT for AVG(c4)
                    if let Some(c4_val) = schema.get_column(&value, 4) {
                        sum_c4 += c4_val;
                        count_c4 += 1;
                    }
                }
            }
        }
    }
    
    let avg_c4 = if count_c4 > 0 {
        Some(sum_c4 / count_c4 as f32)
    } else {
        None
    };
    
    (max_c2, avg_c4)
}

/// Execute a query based on type
pub fn execute_query(db: &Db, schema: &TableSchema, query_type: QueryType, 
                     theta: u64, filter_col: usize) {
    match query_type {
        QueryType::Q1 => {
            let _count = execute_q1(db, schema, theta);
            // Result is used internally for computation
        }
        QueryType::Q2 => {
            let _results = execute_q2(db, schema, theta, filter_col);
            // Results are used internally
        }
        QueryType::Q3 => {
            let (_max, _avg) = execute_q3(db, schema, theta, filter_col);
            // Results are used internally
        }
    }
}
