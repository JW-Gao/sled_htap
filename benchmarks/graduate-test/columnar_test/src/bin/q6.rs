use sled::{Db, Config};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let db_path = "lineitem_db";
    let config = Config::new().path(db_path).mode(sled::Mode::HighThroughput);
    let db = config.open()?;
    
    println!("Running Q6 on {}", db_path);
    
    // Q6: 
    // select sum(l_extendedprice * l_discount) as revenue
    // from lineitem
    // where l_shipdate >= date '1994-01-01'
    //   and l_shipdate < date '1995-01-01'
    //   and l_discount between 0.05 and 0.07
    //   and l_quantity < 24;
    
    // Schema: 0=qty, 1=price, 2=disc, 3=date
    
    // Date: 1994-01-01 -> 19940101.0
    // Date: 1995-01-01 -> 19950101.0
    let date_min = 19940101.0;
    let date_max = 19950101.0;
    let disc_min = 0.05;
    let disc_max = 0.07;
    let qty_max = 24.0;
    
    let start = Instant::now();
    
    // Use NEW scan_column API
    // We need to iterate 4 columns.
    // Tree API currently exposes `scan_column(idx) -> Iterator`.
    // We can zip them? "Scan: node children" logic iterates Node by Node.
    // Zip might misalign if Iterators don't sync on Node Boundaries exactly?
    // They iterate LOGICALLY over keyspace. 
    // Sled Iterators are consistent snapshots (MVCC).
    // If we create 4 iterators, they might see slightly different snapshots if DB is updating?
    // But here DB is static (read-only workload).
    // So zipping is SAFE.
    
    let iter_qty = db.scan_column(0);
    let iter_price = db.scan_column(1);
    let iter_disc = db.scan_column(2);
    let iter_date = db.scan_column(3);
    
    let mut revenue = 0.0;
    let mut count = 0;
    
    for (((qty, price), disc), date) in iter_qty.zip(iter_price).zip(iter_disc).zip(iter_date) {
        // Unpack Results. scan_column returns Result<f32>? No, impl Iterator Item=f32.
        // Wait, `ColumnScanIter` item is `f32`.
        // So no Result unwrapping needed here if `ColumnScanIter` handles errors internally (returns None).
        
        if date >= date_min && date < date_max 
           && disc >= disc_min && disc <= disc_max 
           && qty < qty_max {
               revenue += price * disc;
        }
        count += 1;
    }
    
    println!("Revenue: {:.2}", revenue);
    println!("Processed {} rows in {:.2?}", count, start.elapsed());
    
    Ok(())
}
