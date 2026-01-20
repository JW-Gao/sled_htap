use std::str::FromStr;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum DataType {
    F32,
    I32,
    Date,
    Bytes(usize),
}

#[derive(Debug, Clone)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<(String, DataType)>,
    pub columnar_cols: Vec<usize>, 
    // Which columns to Keep during Ingest (Projection Pushdown). 
    // If empty, keep all.
    pub ingest_projection: Vec<usize>, 
}

impl TableSchema {
    pub fn new(
        name: &str, 
        columns: Vec<(&str, DataType)>, 
        columnar_cols: Vec<usize>,
        ingest_projection: Vec<usize>
    ) -> Self {
        Self {
            name: name.to_string(),
            columns: columns.into_iter().map(|(n, t)| (n.to_string(), t)).collect(),
            columnar_cols,
            ingest_projection,
        }
    }

    /// Parse a "|" separated line from TPC-H tbl file into a Row Binary.
    /// Only parses columns specified in `ingest_projection` to save Hot Log space.
    pub fn parse_row(&self, line: &str) -> Vec<u8> {
        let parts: Vec<&str> = line.split('|').collect();
        // Estimate size: 4 bytes per projected col
        let mut row_data = Vec::with_capacity(std::cmp::max(self.ingest_projection.len() * 4, 16));

        let cols_to_process = if self.ingest_projection.is_empty() {
            (0..self.columns.len()).collect::<Vec<_>>()
        } else {
            self.ingest_projection.clone()
        };

        for &col_idx in &cols_to_process {
            if col_idx >= parts.len() || col_idx >= self.columns.len() {
                continue; 
            }
            let val_str = parts[col_idx];
            let (_, dtype) = &self.columns[col_idx];

            match dtype {
                DataType::F32 => {
                    let val: f32 = val_str.parse().unwrap_or(0.0);
                    row_data.extend_from_slice(&val.to_le_bytes());
                }
                DataType::I32 => {
                    let val: i32 = val_str.parse().unwrap_or(0);
                    row_data.extend_from_slice(&val.to_le_bytes());
                }
                DataType::Date => {
                    // 1994-01-01 -> 19940101.0 (as f32 for compatibility)
                    let val = parse_date_as_f32(val_str);
                    row_data.extend_from_slice(&val.to_le_bytes());
                }
                DataType::Bytes(len) => {
                    // Truncate or pad string to fixed length
                    let bytes = val_str.as_bytes();
                    if bytes.len() >= *len {
                        row_data.extend_from_slice(&bytes[..*len]);
                    } else {
                        row_data.extend_from_slice(bytes);
                        row_data.resize(row_data.len() + (*len - bytes.len()), 0);
                    }
                }
            }
        }
        row_data
    }
}

fn parse_date_as_f32(date_str: &str) -> f32 {
    let s = date_str.replace("-", "");
    s.parse::<f32>().unwrap_or(0.0)
}
