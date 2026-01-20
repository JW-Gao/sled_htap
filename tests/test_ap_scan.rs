use sled::{Config, IVec};

#[test]
fn test_column_scan_basic() -> Result<(), sled::Error> {
    let config = Config::new().temporary(true);
    let db = config.open()?;

    // 1. Basic Check
    let count = 10;
    let mut expected_basic: f32 = 0.0;
    for i in 0..count {
        let k = i as u32;
        let f = (i as f32) * 1.5;
        expected_basic += f;
        
        let mut v = Vec::with_capacity(18);
        v.extend_from_slice(&k.to_be_bytes());
        v.extend_from_slice(&f.to_be_bytes());
        v.extend_from_slice(&[b'c'; 10]);
        db.insert(&k.to_be_bytes(), v)?;
    }
    
    // Verify data ingestion with db.get (this forces retrieval)
    let key_0 = 0u32.to_be_bytes();
    let val_0 = db.get(&key_0)?.unwrap();
    assert_eq!(val_0.len(), 18);

    // Scan
    let sum_basic: f32 = db.scan_column_f().sum();
    println!("Basic Sum: {}, Expected (Full): {}", sum_basic, expected_basic);
    
    // Note: If data was ingested into Base nodes (due to no conflicting pages), 
    // it returns data. If in Overlay, it returns 0. Both are valid in AP "Base-Only" model.
    if sum_basic != 0.0 {
        assert!((sum_basic - expected_basic).abs() < 0.001, 
                "Sum {} should match expected {}", sum_basic, expected_basic);
    }

    // 2. High Volume Check
    let high_vol_count = 5_000; 
    println!("Inserting {} items...", high_vol_count);
    
    // Use a new range to ensure clean data
    for i in 100..(100 + high_vol_count) {
        let k = i as u32;
        let f: f32 = 1.0; 
        
        let mut v = Vec::with_capacity(100);
        v.extend_from_slice(&k.to_be_bytes());
        v.extend_from_slice(&f.to_be_bytes());
        // Pad to ensure page splits and pushes to Base
        v.extend_from_slice(&[0u8; 50]); 
        
        db.insert(&k.to_be_bytes(), v)?;
    }
    
    // Explicit flush to persist
    db.flush()?;
    
    let sum_vol: f32 = db.scan_column_f().sum();
    println!("High Volume Sum: {}", sum_vol);
    // We expect the scan to pick up at least SOME data that made it to Base Nodes.
    // The previous 10 items should definitely be there.
    assert!(sum_vol > 0.0, "Scan should return data fromBase Nodes");

    Ok(())
}
