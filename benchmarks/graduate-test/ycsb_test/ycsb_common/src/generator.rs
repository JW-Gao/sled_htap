use rand::prelude::*;
use rand::distributions::{Uniform, Distribution};
use zipf::ZipfDistribution;

pub enum KeyGenerator {
    Zipf(ZipfDistribution),
    Uniform(Uniform<usize>),
    Sequential(usize), // Counter
}

impl KeyGenerator {
    pub fn new_zipf(items: usize, skew: f64) -> anyhow::Result<Self> {
        let dist = ZipfDistribution::new(items, skew).map_err(|_| anyhow::anyhow!("Failed to create Zipf distribution"))?;
        Ok(KeyGenerator::Zipf(dist))
    }

    pub fn new_uniform(items: usize) -> Self {
        KeyGenerator::Uniform(Uniform::new(0, items))
    }
    
    pub fn new_sequential() -> Self {
        KeyGenerator::Sequential(0)
    }

    pub fn next(&mut self, rng: &mut impl Rng) -> usize {
        match self {
            KeyGenerator::Zipf(d) => d.sample(rng),
            KeyGenerator::Uniform(d) => d.sample(rng),
            KeyGenerator::Sequential(c) => {
                let v = *c;
                *c += 1;
                v
            }
        }
    }
}
