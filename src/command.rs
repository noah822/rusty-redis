use crate::parser::encrypt::as_bulk_str;


pub fn echo(msg: &[u8]) -> Box<[u8]>{
   as_bulk_str(msg) 
}

pub fn ping() -> Box<[u8]>{
    as_bulk_str(b"PONG")
}
