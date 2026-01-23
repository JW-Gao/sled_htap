use rand::Rng;
// use std::io::Cursor;
// use std::io::Write;
// use sled::node::RowCell; // Keeping this if needed, else remove

pub const PK_SIZE: usize = 8;
pub const COL_SIZE: usize = 4; // f32

pub struct TableSchema {
    pub num_columns: usize,
}

impl TableSchema {
    pub fn new(num_columns: usize) -> Self {
        Self { num_columns }
    }

    pub fn generate_row(&self, pk: usize) -> Vec<u8> {
        // [PK (8)] + [Col1 (4)] + ... + [ColN (4)]
        let row_len = PK_SIZE + self.num_columns * COL_SIZE;
        let mut buf = Vec::with_capacity(row_len);
        
        // Write PK (u64 le)
        buf.extend_from_slice(&(pk as u64).to_le_bytes());

        // Write Columns (f32 le)
        let mut rng = rand::thread_rng();
        for _ in 0..self.num_columns {
            let val = rng.gen::<f32>();
            buf.extend_from_slice(&val.to_le_bytes());
        }
        
        buf
    }

    pub fn get_pk(&self, row: &[u8]) -> u64 {
        let pk_bytes = &row[0..8];
        u64::from_le_bytes(pk_bytes.try_into().unwrap())
    }

    pub fn modify_row(&self, old_row: &[u8], col_idx_to_update: usize) -> Vec<u8> {
        let mut new_row = old_row.to_vec();
        // Skip PK (0..8)
        // Col 0 starts at 8
        let offset = PK_SIZE + col_idx_to_update * COL_SIZE;
        
        if offset + 4 <= new_row.len() {
            let mut rng = rand::thread_rng();
            let val = rng.gen::<f32>();
            let bytes = val.to_le_bytes();
            new_row[offset..offset+4].copy_from_slice(&bytes);
        }
        new_row
    }
}
