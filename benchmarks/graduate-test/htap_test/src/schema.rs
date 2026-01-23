use rand::Rng;

pub const PK_SIZE: usize = 8;  // u64
pub const COL_SIZE: usize = 4; // f32

pub struct TableSchema {
    pub num_columns: usize,
}

impl TableSchema {
    pub fn new(num_columns: usize) -> Self {
        Self { num_columns }
    }

    /// Generate a row with PK and random column values
    /// Format: [PK (8 bytes)] + [Col1 (4 bytes)] + ... + [ColN (4 bytes)]
    pub fn generate_row(&self, pk: u64) -> Vec<u8> {
        let row_len = PK_SIZE + self.num_columns * COL_SIZE;
        let mut buf = Vec::with_capacity(row_len);
        
        // Write PK (u64 little-endian)
        buf.extend_from_slice(&pk.to_le_bytes());

        // Write columns (f32 little-endian)
        let mut rng = rand::thread_rng();
        for _ in 0..self.num_columns {
            let val: f32 = rng.gen_range(0.0..1000.0);
            buf.extend_from_slice(&val.to_le_bytes());
        }
        
        buf
    }

    /// Extract primary key from a row
    pub fn get_pk(&self, row: &[u8]) -> u64 {
        if row.len() < PK_SIZE {
            return 0;
        }
        let pk_bytes = &row[0..PK_SIZE];
        u64::from_le_bytes(pk_bytes.try_into().unwrap())
    }

    /// Extract a column value from a row
    pub fn get_column(&self, row: &[u8], col_idx: usize) -> Option<f32> {
        let offset = PK_SIZE + col_idx * COL_SIZE;
        if offset + COL_SIZE > row.len() {
            return None;
        }
        let col_bytes = &row[offset..offset + COL_SIZE];
        Some(f32::from_le_bytes(col_bytes.try_into().unwrap()))
    }

    /// Get row size in bytes
    pub fn row_size(&self) -> usize {
        PK_SIZE + self.num_columns * COL_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_parse_row() {
        let schema = TableSchema::new(30);
        let row = schema.generate_row(12345);
        
        assert_eq!(schema.get_pk(&row), 12345);
        assert!(schema.get_column(&row, 0).is_some());
        assert!(schema.get_column(&row, 29).is_some());
        assert!(schema.get_column(&row, 30).is_none());
    }
}
