pub mod generator;
pub mod workload;

pub use workload::{YcsbConfig, load_data, run_workload};


pub trait KVEngine {
    fn put(&self, key: &[u8], value: &[u8]) -> anyhow::Result<()>;
    fn get(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>>;
    // scan returns (key, value) pairs
    fn scan(&self, start_key: &[u8], count: usize) -> anyhow::Result<Vec<(Vec<u8>, Vec<u8>)>>;
}
