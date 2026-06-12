use crate::olap_queries::QueryType;
use rand::seq::SliceRandom;

#[derive(Debug, Clone)]
pub enum Operation {
    Insert(u64),           // OLTP: Insert with PK
    Query(QueryType, u64), // OLAP: Query type and theta (filter threshold)
}

pub struct WorkloadGenerator {
    pub operations: Vec<Operation>,
}

impl WorkloadGenerator {
    /// Generate a mixed workload
    /// 
    /// # Arguments
    /// * `total_ops` - Total number of operations
    /// * `olap_ratio` - Ratio of OLAP operations (0.0 to 1.0)
    /// * `oltp_ratio` - Ratio of OLTP operations (0.0 to 1.0)
    /// * `data_access_ratio` - Ratio of data that AP queries will access (0.0 to 1.0)
    /// * `total_rows` - Total number of rows in the database (for calculating theta)
    pub fn new(
        total_ops: usize,
        olap_ratio: f64,
        oltp_ratio: f64,
        data_access_ratio: f64,
        total_rows: u64,
    ) -> Self {
        let mut operations = Vec::with_capacity(total_ops);
        let mut rng = rand::thread_rng();
        
        let num_olap = (total_ops as f64 * olap_ratio) as usize;
        let num_oltp = (total_ops as f64 * oltp_ratio) as usize;
        
        // Calculate theta based on data access ratio
        let theta = (total_rows as f64 * data_access_ratio) as u64;
        
        // Generate OLAP operations
        for _ in 0..num_olap {
            let query_type = QueryType::random();
            operations.push(Operation::Query(query_type, theta));
        }
        
        // Generate OLTP operations (inserts)
        // Start from total_rows to avoid conflicts with existing data
        for i in 0..num_oltp {
            let pk = total_rows + i as u64;
            operations.push(Operation::Insert(pk));
        }
        
        // Shuffle operations to simulate realistic mixed workload
        operations.shuffle(&mut rng);
        
        Self { operations }
    }
    
    /// Get total number of operations
    pub fn len(&self) -> usize {
        self.operations.len()
    }
    
    /// Check if workload is empty
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_generation() {
        let total_ops = 10000;
        let olap_ratio = 0.7;
        let oltp_ratio = 0.3;
        let workload = WorkloadGenerator::new(total_ops, olap_ratio, oltp_ratio, 0.5, 100000);
        
        assert!(workload.len() <= total_ops);
        
        let mut insert_count = 0;
        let mut query_count = 0;
        
        for op in &workload.operations {
            match op {
                Operation::Insert(_) => insert_count += 1,
                Operation::Query(_, _) => query_count += 1,
            }
        }
        
        println!("Inserts: {}, Queries: {}", insert_count, query_count);
        assert!(insert_count > 0);
        assert!(query_count > 0);
    }
}
