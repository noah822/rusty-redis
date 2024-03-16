use crate::parser::encrypt::{as_bulk_str, as_array};
use crate::parser::{AtomicItem, AggrRESPObject, SimpleRESPObject};
use crate::server::LaunchConfig;

use std::collections::HashMap;


pub fn echo(msg: &[u8]) -> Box<[u8]>{
   as_bulk_str(Some(msg)) 
}

pub fn ping() -> Box<[u8]>{
    as_bulk_str(Some(b"PONG"))
}


pub fn set(
    key: &[u8], val: &[u8],
    storage: &mut HashMap<Box<[u8]>, Box<[u8]>>
) -> Box<[u8]>{
    let (mut malloced_key, mut malloced_val) = (vec!(), vec!());
    malloced_key.extend_from_slice(key);
    malloced_val.extend_from_slice(val);
    
    storage.insert(malloced_key.into_boxed_slice(), malloced_val.into_boxed_slice());
    as_bulk_str(Some(b"OK"))
}

pub fn get(var: &[u8], storage: &HashMap<Box<[u8]>, Box<[u8]>>) -> Box<[u8]>{
   match storage.get(var) {
       Some(val) => as_bulk_str(Some(val)),
       None => as_bulk_str(None)
   } 
}


pub fn info(query: &[u8], server_state: &LaunchConfig) -> Box<[u8]>{
   match query{
       b"replica" => {
           match server_state.replicaof {
               Some(_) => as_bulk_str(Some(b"role:slave")),
               None => {
                  let random_seed = "8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb";
                  let response = vec!(
                      String::from("role:master"),
                      format!("master_replid:{random_seed}"),
                      String::from("master_repl_offset:0")
                  );
                  let response = response.iter().map(
                      |item| {AtomicItem::AggrItem(AggrRESPObject::BulkStr(item.as_bytes()))}
                  ).collect::<Vec<_>>();
                  as_array(response)
               }
           } 
       },
       _ => as_bulk_str(None) 
   } 
}

