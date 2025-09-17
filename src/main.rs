// use std::vec;

use std::hash::Hash;

use im::HashMap;
// use crate::*;
#[allow(unused)]
use sled::*;
// use zerocopy::AsBytes;

#[allow(unused)]
fn main() -> sled::Result<()>{
  // let options = sled::Config::create_new(self, to)
  let tree = sled::open("/tmp/welcome-to-sled")?;

  let k_value = tree.get("abc")?;
  if let Some(vec_k_value) = k_value {
    println!("{}", std::str::from_utf8(vec_k_value.as_ref()).unwrap());
  }
  // insert and get, similar to std's BTreeMap
  // let old_value = tree.insert("key", "another_value")?;
  // tree.insert(b"a", b"1")?;
  // tree.insert(b"b", b"2")?;
  // tree.insert(b"c", b"3");
  let mut mp: HashMap<&str, String> = HashMap::new();
  mp.insert("abc", String::from("xyz"));

  for (k, v) in &mp {
    tree.insert(k, v);
  }

  
  // block until all operations are stable on disk
  // (flush_async also available to get a Future)
  // tree.flush()?;
  // println!("\033[1;31mTask done\033[0;0m");
  Ok(())
}